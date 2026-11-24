//! Procedural 3D mesh generators matching Three.js geometry formulas and CCW winding.

use crate::math::{mix, ray_sphere, ray_triangle, Mat4, Vec2, Vec3, PI, TAU};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub rest: [f32; 3],
    pub color: [f32; 3],
    pub along: f32,
}

impl Vertex {
    pub fn new(pos: Vec3, normal: Vec3) -> Self {
        Self {
            pos: pos.to_array(),
            normal: normal.to_array(),
            rest: pos.to_array(),
            color: [1.0, 1.0, 1.0],
            along: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bound_center: Vec3,
    pub bound_radius: f32,
}

impl MeshData {
    pub fn compute_bounds(&mut self) {
        if self.vertices.is_empty() {
            self.bound_center = Vec3::ZERO;
            self.bound_radius = 0.0;
            return;
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in &self.vertices {
            min.x = min.x.min(v.pos[0]);
            min.y = min.y.min(v.pos[1]);
            min.z = min.z.min(v.pos[2]);
            max.x = max.x.max(v.pos[0]);
            max.y = max.y.max(v.pos[1]);
            max.z = max.z.max(v.pos[2]);
        }
        let center = (min + max) * 0.5;
        let mut r2 = 0.0f32;
        for v in &self.vertices {
            let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
            r2 = r2.max(p.distance_sq(center));
        }
        self.bound_center = center;
        self.bound_radius = r2.sqrt();
    }

    pub fn compute_vertex_normals(&mut self) {
        let mut accum = vec![Vec3::ZERO; self.vertices.len()];
        for tri in self.indices.chunks_exact(3) {
            let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            let p0 = Vec3::new(self.vertices[i0].pos[0], self.vertices[i0].pos[1], self.vertices[i0].pos[2]);
            let p1 = Vec3::new(self.vertices[i1].pos[0], self.vertices[i1].pos[1], self.vertices[i1].pos[2]);
            let p2 = Vec3::new(self.vertices[i2].pos[0], self.vertices[i2].pos[1], self.vertices[i2].pos[2]);
            let n = (p1 - p0).cross(p2 - p0);
            accum[i0] += n;
            accum[i1] += n;
            accum[i2] += n;
        }
        for (v, n) in self.vertices.iter_mut().zip(accum.into_iter()) {
            let len = n.length();
            let u = if len > 1e-8 { n * (1.0 / len) } else { Vec3::new(0.0, 1.0, 0.0) };
            v.normal = u.to_array();
        }
    }

    pub fn copy_pos_to_rest(&mut self) {
        for v in &mut self.vertices {
            v.rest = v.pos;
        }
    }

    pub fn fill_along(&mut self, along: f32) {
        for v in &mut self.vertices {
            v.along = along;
        }
    }

    pub fn rotate_x(&mut self, angle: f32) {
        let (s, c) = angle.sin_cos();
        for v in &mut self.vertices {
            let (y, z) = (v.pos[1], v.pos[2]);
            v.pos[1] = y * c - z * s;
            v.pos[2] = y * s + z * c;
            let (ny, nz) = (v.normal[1], v.normal[2]);
            v.normal[1] = ny * c - nz * s;
            v.normal[2] = ny * s + nz * c;
            v.rest = v.pos;
        }
        self.compute_bounds();
    }

    pub fn rotate_y(&mut self, angle: f32) {
        let (s, c) = angle.sin_cos();
        for v in &mut self.vertices {
            let (x, z) = (v.pos[0], v.pos[2]);
            v.pos[0] = x * c + z * s;
            v.pos[2] = -x * s + z * c;
            let (nx, nz) = (v.normal[0], v.normal[2]);
            v.normal[0] = nx * c + nz * s;
            v.normal[2] = -nx * s + nz * c;
            v.rest = v.pos;
        }
        self.compute_bounds();
    }

    pub fn translate(&mut self, dx: f32, dy: f32, dz: f32) {
        for v in &mut self.vertices {
            v.pos[0] += dx;
            v.pos[1] += dy;
            v.pos[2] += dz;
            v.rest = v.pos;
        }
        self.compute_bounds();
    }

    /// Tests a world-space ray against this mesh transformed by `world`.
    pub fn raycast(&self, world: Mat4, ray_orig: Vec3, ray_dir: Vec3) -> bool {
        let Some(inv) = world.inverse() else { return false };
        let orig = inv.transform_point(ray_orig);
        let dir = inv.transform_vec(ray_dir).normalize();
        if !ray_sphere(orig, dir, self.bound_center, self.bound_radius) {
            return false;
        }
        for tri in self.indices.chunks_exact(3) {
            let a = self.vertices[tri[0] as usize].pos;
            let b = self.vertices[tri[1] as usize].pos;
            let c = self.vertices[tri[2] as usize].pos;
            if ray_triangle(
                orig,
                dir,
                Vec3::new(a[0], a[1], a[2]),
                Vec3::new(b[0], b[1], b[2]),
                Vec3::new(c[0], c[1], c[2]),
            )
            .is_some()
            {
                return true;
            }
        }
        false
    }
}

/// Three.js `LatheGeometry(points, segments)` — CCW front faces facing outward.
pub fn lathe(points: &[Vec2], segments: usize) -> MeshData {
    let mut vertices = Vec::with_capacity((segments + 1) * points.len());
    let mut indices = Vec::with_capacity(segments * points.len().saturating_sub(1) * 6);
    for i in 0..=segments {
        let phi = (i as f32 / segments as f32) * TAU;
        let (sin_p, cos_p) = phi.sin_cos();
        for p in points {
            let pos = Vec3::new(p.x * sin_p, p.y, p.x * cos_p);
            vertices.push(Vertex::new(pos, Vec3::new(sin_p, 0.0, cos_p)));
        }
    }
    let n = points.len() as u32;
    for i in 0..(segments as u32) {
        let base = i * n;
        for j in 0..(n - 1) {
            let a = base + j;
            let b = base + n + j;
            let c = base + n + j + 1;
            let d = base + j + 1;
            indices.extend_from_slice(&[a, b, d, b, c, d]);
        }
    }
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_vertex_normals();
    mesh.copy_pos_to_rest();
    mesh.compute_bounds();
    mesh
}

/// Three.js `SphereGeometry(radius, wSegs, hSegs, phiStart, phiLen, thetaStart, thetaLen)`.
pub fn sphere(
    radius: f32,
    w_segs: usize,
    h_segs: usize,
    phi_start: f32,
    phi_len: f32,
    theta_start: f32,
    theta_len: f32,
) -> MeshData {
    let mut vertices = Vec::with_capacity((h_segs + 1) * (w_segs + 1));
    let mut grid = vec![vec![0u32; w_segs + 1]; h_segs + 1];
    let theta_end = (theta_start + theta_len).min(PI);
    let mut idx = 0u32;
    for iy in 0..=h_segs {
        let v = iy as f32 / h_segs as f32;
        let theta = theta_start + v * theta_len;
        let (sin_t, cos_t) = theta.sin_cos();
        for ix in 0..=w_segs {
            let u = ix as f32 / w_segs as f32;
            let phi = phi_start + u * phi_len;
            let (sin_p, cos_p) = phi.sin_cos();
            let pos = Vec3::new(-radius * cos_p * sin_t, radius * cos_t, radius * sin_p * sin_t);
            let n = pos.normalize();
            vertices.push(Vertex::new(pos, n));
            grid[iy][ix] = idx;
            idx += 1;
        }
    }
    let mut indices = Vec::with_capacity(h_segs * w_segs * 6);
    for iy in 0..h_segs {
        for ix in 0..w_segs {
            let a = grid[iy][ix + 1];
            let b = grid[iy][ix];
            let c = grid[iy + 1][ix];
            let d = grid[iy + 1][ix + 1];
            if iy != 0 || theta_start > 0.0 {
                indices.extend_from_slice(&[a, b, d]);
            }
            if iy != h_segs - 1 || theta_end < PI {
                indices.extend_from_slice(&[b, c, d]);
            }
        }
    }
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

pub fn full_sphere(radius: f32, w_segs: usize, h_segs: usize) -> MeshData {
    sphere(radius, w_segs, h_segs, 0.0, TAU, 0.0, PI)
}

/// Three.js `CircleGeometry(radius, segments, thetaStart, thetaLength)` in XY plane (+Z normal).
pub fn circle(radius: f32, segments: usize, theta_start: f32, theta_len: f32) -> MeshData {
    let mut vertices = Vec::with_capacity(segments + 2);
    let n = Vec3::new(0.0, 0.0, 1.0);
    vertices.push(Vertex::new(Vec3::ZERO, n));
    for s in 0..=segments {
        let a = theta_start + (s as f32 / segments as f32) * theta_len;
        let (sin_a, cos_a) = a.sin_cos();
        vertices.push(Vertex::new(Vec3::new(radius * cos_a, radius * sin_a, 0.0), n));
    }
    let mut indices = Vec::with_capacity(segments * 3);
    for i in 1..=(segments as u32) {
        indices.extend_from_slice(&[i, i + 1, 0]);
    }
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

/// Three.js `PlaneGeometry(w, h)` in XY plane (+Z normal).
pub fn plane(w: f32, h: f32) -> MeshData {
    let (hw, hh) = (w * 0.5, h * 0.5);
    let n = Vec3::new(0.0, 0.0, 1.0);
    let vertices = vec![
        Vertex::new(Vec3::new(-hw, hh, 0.0), n),
        Vertex::new(Vec3::new(hw, hh, 0.0), n),
        Vertex::new(Vec3::new(-hw, -hh, 0.0), n),
        Vertex::new(Vec3::new(hw, -hh, 0.0), n),
    ];
    let indices = vec![0, 2, 1, 2, 3, 1];
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

/// Three.js `CylinderGeometry(rTop, rBottom, height, radialSegments, 1, openEnded=true)`.
pub fn cylinder_open(r_top: f32, r_bot: f32, height: f32, radial: usize) -> MeshData {
    let half_h = height * 0.5;
    let slope = (r_bot - r_top) / height;
    let mut vertices = Vec::with_capacity(2 * (radial + 1));
    for iy in 0..=1 {
        let v = iy as f32;
        let y = (0.5 - v) * height;
        let r = mix(r_top, r_bot, v);
        for ix in 0..=radial {
            let theta = (ix as f32 / radial as f32) * TAU;
            let (sin_t, cos_t) = theta.sin_cos();
            let pos = Vec3::new(r * sin_t, y, r * cos_t);
            let n = Vec3::new(sin_t, slope, cos_t).normalize();
            vertices.push(Vertex::new(pos, n));
        }
    }
    let row = (radial + 1) as u32;
    let mut indices = Vec::with_capacity(radial * 6);
    for ix in 0..(radial as u32) {
        let a = ix;
        let b = row + ix;
        let c = row + ix + 1;
        let d = ix + 1;
        indices.extend_from_slice(&[a, b, d, b, c, d]);
    }
    let _ = half_h;
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

/// Three.js `TorusGeometry(radius, tube, radialSegments, tubularSegments)`.
pub fn torus(radius: f32, tube: f32, radial: usize, tubular: usize) -> MeshData {
    let mut vertices = Vec::with_capacity((radial + 1) * (tubular + 1));
    for j in 0..=radial {
        let v = (j as f32 / radial as f32) * TAU;
        let (sin_v, cos_v) = v.sin_cos();
        for i in 0..=tubular {
            let u = (i as f32 / tubular as f32) * TAU;
            let (sin_u, cos_u) = u.sin_cos();
            let x = (radius + tube * cos_v) * cos_u;
            let y = (radius + tube * cos_v) * sin_u;
            let z = tube * sin_v;
            let center = Vec3::new(radius * cos_u, radius * sin_u, 0.0);
            let pos = Vec3::new(x, y, z);
            vertices.push(Vertex::new(pos, (pos - center).normalize()));
        }
    }
    let stride = (tubular + 1) as u32;
    let mut indices = Vec::with_capacity(radial * tubular * 6);
    for j in 1..=(radial as u32) {
        for i in 1..=(tubular as u32) {
            let a = stride * j + i - 1;
            let b = stride * (j - 1) + i - 1;
            let c = stride * (j - 1) + i;
            let d = stride * j + i;
            indices.extend_from_slice(&[a, b, d, b, c, d]);
        }
    }
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

/// Rounded rectangular slab matching `web/keyboard.js` `slab(w, d, h, r, color, y)`.
pub fn rounded_slab(w: f32, d: f32, h: f32, r: f32, y_base: f32) -> MeshData {
    let (x0, z0) = (-w * 0.5, -d * 0.5);
    let segs = 6usize;
    let mut contour = Vec::with_capacity(segs * 4 + 4);
    let quad = |out: &mut Vec<Vec2>, p0: Vec2, pc: Vec2, p1: Vec2| {
        for i in 0..segs {
            let t = i as f32 / segs as f32;
            let u = 1.0 - t;
            out.push(Vec2::new(
                u * u * p0.x + 2.0 * u * t * pc.x + t * t * p1.x,
                u * u * p0.y + 2.0 * u * t * pc.y + t * t * p1.y,
            ));
        }
    };
    // Same winding in XZ plane after Three.js rotateX(PI/2) + translate(0, y + h, 0):
    // top face at y = y_base + h (+Y normal), bottom face at y = y_base (-Y normal).
    quad(&mut contour, Vec2::new(x0 + w - r, z0), Vec2::new(x0 + w, z0), Vec2::new(x0 + w, z0 + r));
    quad(&mut contour, Vec2::new(x0 + w, z0 + d - r), Vec2::new(x0 + w, z0 + d), Vec2::new(x0 + w - r, z0 + d));
    quad(&mut contour, Vec2::new(x0 + r, z0 + d), Vec2::new(x0, z0 + d), Vec2::new(x0, z0 + d - r));
    quad(&mut contour, Vec2::new(x0, z0 + r), Vec2::new(x0, z0), Vec2::new(x0 + r, z0));

    let n = contour.len();
    let y_top = y_base + h;
    let y_bot = y_base;
    let mut vertices = Vec::with_capacity(n * 4 + 2);
    let mut indices = Vec::with_capacity(n * 12);

    // Top cap (+Y normal, CCW when viewed from +Y: (x0,z0)->(x0,z1)->(x1,z1) -> reverse of CW contour)
    let top_center = vertices.len() as u32;
    vertices.push(Vertex::new(Vec3::new(0.0, y_top, 0.0), Vec3::new(0.0, 1.0, 0.0)));
    let top_ring = vertices.len() as u32;
    for p in &contour {
        vertices.push(Vertex::new(Vec3::new(p.x, y_top, p.y), Vec3::new(0.0, 1.0, 0.0)));
    }
    for i in 0..n {
        let a = top_ring + i as u32;
        let b = top_ring + ((i + 1) % n) as u32;
        // Contour goes CW when viewed from +Y (top-right -> bottom-right -> bottom-left -> top-left),
        // so (center, b, a) is CCW facing +Y.
        indices.extend_from_slice(&[top_center, b, a]);
    }

    // Bottom cap (-Y normal)
    let bot_center = vertices.len() as u32;
    vertices.push(Vertex::new(Vec3::new(0.0, y_bot, 0.0), Vec3::new(0.0, -1.0, 0.0)));
    let bot_ring = vertices.len() as u32;
    for p in &contour {
        vertices.push(Vertex::new(Vec3::new(p.x, y_bot, p.y), Vec3::new(0.0, -1.0, 0.0)));
    }
    for i in 0..n {
        let a = bot_ring + i as u32;
        let b = bot_ring + ((i + 1) % n) as u32;
        indices.extend_from_slice(&[bot_center, a, b]);
    }

    // Side walls (outward normals)
    for i in 0..n {
        let p0 = contour[i];
        let p1 = contour[(i + 1) % n];
        let dx = p1.x - p0.x;
        let dz = p1.y - p0.y;
        // Outward normal for CW contour in XZ (X right, Z down): (+dz, 0, -dx)
        let nrm = Vec3::new(dz, 0.0, -dx).normalize();
        let base = vertices.len() as u32;
        vertices.push(Vertex::new(Vec3::new(p0.x, y_top, p0.y), nrm));
        vertices.push(Vertex::new(Vec3::new(p1.x, y_top, p1.y), nrm));
        vertices.push(Vertex::new(Vec3::new(p0.x, y_bot, p0.y), nrm));
        vertices.push(Vertex::new(Vec3::new(p1.x, y_bot, p1.y), nrm));
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

/// Cat nose rounded triangle matching `web/cat.js:395-401` (XY plane, +Z normal).
pub fn cat_nose() -> MeshData {
    let segs = 10usize;
    let curves = [
        (Vec2::new(-0.045, 0.012), Vec2::new(0.0, 0.026), Vec2::new(0.045, 0.012)),
        (Vec2::new(0.045, 0.012), Vec2::new(0.05, 0.004), Vec2::new(0.012, -0.024)),
        (Vec2::new(0.012, -0.024), Vec2::new(0.0, -0.032), Vec2::new(-0.012, -0.024)),
        (Vec2::new(-0.012, -0.024), Vec2::new(-0.05, 0.004), Vec2::new(-0.045, 0.012)),
    ];
    let nrm = Vec3::new(0.0, 0.0, 1.0);
    let mut vertices = Vec::with_capacity(curves.len() * segs + 1);
    vertices.push(Vertex::new(Vec3::ZERO, nrm));
    for (p0, pc, p1) in curves {
        for i in 0..segs {
            let t = i as f32 / segs as f32;
            let u = 1.0 - t;
            let x = u * u * p0.x + 2.0 * u * t * pc.x + t * t * p1.x;
            let y = u * u * p0.y + 2.0 * u * t * pc.y + t * t * p1.y;
            vertices.push(Vertex::new(Vec3::new(x, y, 0.0), nrm));
        }
    }
    let count = (vertices.len() - 1) as u32;
    let mut indices = Vec::with_capacity(count as usize * 3);
    // Contour goes CW around origin -> (0, next, cur) is CCW facing +Z.
    for i in 0..count {
        let a = 1 + i;
        let b = 1 + ((i + 1) % count);
        indices.extend_from_slice(&[0, b, a]);
    }
    let mut mesh = MeshData { vertices, indices, ..Default::default() };
    mesh.compute_bounds();
    mesh
}

/// Dynamic tapered tube with precomputed cos/sin tables and shared-frame pair update.
#[derive(Clone, Debug)]
pub struct TaperTube {
    pub segments: usize,
    pub radial: usize,
    pub cos: Vec<f32>,
    pub sin: Vec<f32>,
    pub pts: Vec<Vec3>,
    pub mesh: MeshData,
    pub compute_bounds: bool,
}

impl TaperTube {
    pub fn new(segments: usize, radial: usize, compute_bounds: bool) -> Self {
        let mut cos = Vec::with_capacity(radial);
        let mut sin = Vec::with_capacity(radial);
        for j in 0..radial {
            let a = (j as f32 / radial as f32) * TAU;
            cos.push(a.cos());
            sin.push(a.sin());
        }
        let count = (segments + 1) * radial;
        let mut vertices = vec![Vertex::default(); count];
        for (k, v) in vertices.iter_mut().enumerate() {
            v.color = [1.0, 1.0, 1.0];
            v.along = (k / radial) as f32 / segments as f32;
        }
        let mut indices = Vec::with_capacity(segments * radial * 6);
        for i in 0..segments {
            for j in 0..radial {
                let a = (i * radial + j) as u32;
                let b = (i * radial + (j + 1) % radial) as u32;
                let c = a + radial as u32;
                let d = b + radial as u32;
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        Self {
            segments,
            radial,
            cos,
            sin,
            pts: vec![Vec3::ZERO; segments + 1],
            mesh: MeshData { vertices, indices, ..Default::default() },
            compute_bounds,
        }
    }

    pub fn update<F, R>(&mut self, mut point: F, radius: R, grow: f32)
    where
        F: FnMut(f32) -> Vec3,
        R: Fn(f32) -> f32,
    {
        let n = self.segments;
        let m = self.radial;
        for i in 0..=n {
            self.pts[i] = point(i as f32 / n as f32);
        }
        let mut n_vec = Vec3::ZERO;
        for i in 0..=n {
            let p_prev = self.pts[i.saturating_sub(1)];
            let p_next = self.pts[(i + 1).min(n)];
            let t_vec = (p_next - p_prev).normalize();
            if i == 0 {
                let mut tmp = Vec3::new(0.0, 1.0, 0.0);
                if t_vec.dot(tmp).abs() > 0.9 {
                    tmp = Vec3::new(1.0, 0.0, 0.0);
                }
                n_vec = t_vec.cross(tmp).normalize();
            } else {
                let b = n_vec.cross(t_vec);
                n_vec = t_vec.cross(b).normalize();
            }
            let b_vec = t_vec.cross(n_vec).normalize();
            let t = i as f32 / n as f32;
            let r = radius(t) + grow;
            let p = self.pts[i];
            for j in 0..m {
                let ca = self.cos[j];
                let sa = self.sin[j];
                let dir = n_vec * ca + b_vec * sa;
                let pos = p + dir * r;
                let v = &mut self.mesh.vertices[i * m + j];
                v.pos = pos.to_array();
                v.rest = v.pos;
                v.normal = dir.to_array();
            }
        }
        if self.compute_bounds {
            self.mesh.compute_bounds();
        }
    }

    /// Updates a fill + outline pair sharing one centreline and parallel-transport frame.
    pub fn update_pair<F, R, E>(
        body: &mut TaperTube,
        rim: &mut TaperTube,
        mut point: F,
        radius: R,
        edge_at: E,
    ) where
        F: FnMut(f32) -> Vec3,
        R: Fn(f32) -> f32,
        E: Fn(f32) -> f32,
    {
        let n = body.segments;
        let m = body.radial;
        for i in 0..=n {
            body.pts[i] = point(i as f32 / n as f32);
        }
        let mut n_vec = Vec3::ZERO;
        for i in 0..=n {
            let p_prev = body.pts[i.saturating_sub(1)];
            let p_next = body.pts[(i + 1).min(n)];
            let t_vec = (p_next - p_prev).normalize();
            if i == 0 {
                let mut tmp = Vec3::new(0.0, 1.0, 0.0);
                if t_vec.dot(tmp).abs() > 0.9 {
                    tmp = Vec3::new(1.0, 0.0, 0.0);
                }
                n_vec = t_vec.cross(tmp).normalize();
            } else {
                let b = n_vec.cross(t_vec);
                n_vec = t_vec.cross(b).normalize();
            }
            let b_vec = t_vec.cross(n_vec).normalize();
            let t = i as f32 / n as f32;
            let r1 = radius(t);
            let r2 = r1 + edge_at(t);
            let p = body.pts[i];
            for j in 0..m {
                let ca = body.cos[j];
                let sa = body.sin[j];
                let dir = n_vec * ca + b_vec * sa;
                let dir_a = dir.to_array();
                let idx = i * m + j;

                let p1 = (p + dir * r1).to_array();
                let vb = &mut body.mesh.vertices[idx];
                vb.pos = p1;
                vb.rest = p1;
                vb.normal = dir_a;

                let p2 = (p + dir * r2).to_array();
                let vr = &mut rim.mesh.vertices[idx];
                vr.pos = p2;
                vr.rest = p2;
                vr.normal = dir_a;
            }
        }
        if body.compute_bounds {
            body.mesh.compute_bounds();
        }
    }
}
