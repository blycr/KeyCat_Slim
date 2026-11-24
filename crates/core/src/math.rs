//! Right-handed 3D math matching Three.js conventions, with D3D11/Metal [0, 1] NDC depth.
//! All matrices are stored column-major (`m[col][row]`), matching HLSL `mul(matrix, vec)`
//! (or row-major upload with `mul(vec, matrix)` — renderer documents the upload layout).

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = std::f32::consts::TAU;
pub const SQRT_1_2: f32 = std::f32::consts::FRAC_1_SQRT_2;

#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

#[inline]
pub fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn approach(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

#[inline]
pub fn hex_color(hex: &str) -> [f32; 3] {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    let v = u32::from_str_radix(s, 16).unwrap_or(0);
    [
        ((v >> 16) & 0xff) as f32 / 255.0,
        ((v >> 8) & 0xff) as f32 / 255.0,
        (v & 0xff) as f32 / 255.0,
    ]
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    #[inline]
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    #[inline]
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    #[inline]
    pub fn cross(self, o: Self) -> Self {
        Self {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }

    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    #[inline]
    pub fn distance(self, o: Self) -> f32 {
        (self - o).length()
    }

    #[inline]
    pub fn distance_sq(self, o: Self) -> f32 {
        (self - o).length_sq()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-8 {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self {
            x: mix(self.x, o.x, t),
            y: mix(self.y, o.y, t),
            z: mix(self.z, o.z, t),
        }
    }

    #[inline]
    pub fn mul_elem(self, o: Self) -> Self {
        Self { x: self.x * o.x, y: self.y * o.y, z: self.z * o.z }
    }

    /// Applies a Three.js XYZ Euler rotation (`rotation.set(rx, ry, rz)`).
    pub fn apply_euler_xyz(self, rx: f32, ry: f32, rz: f32) -> Self {
        let m = Mat4::from_euler_xyz(rx, ry, rz);
        m.transform_vec(self)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, o: Self) {
        self.x += o.x;
        self.y += o.y;
        self.z += o.z;
    }
}
impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, o: Self) {
        self.x -= o.x;
        self.y -= o.y;
        self.z -= o.z;
    }
}
impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Equivalent to Three.js `Quaternion.setFromUnitVectors(from, to)`.
    pub fn from_unit_vectors(from: Vec3, to: Vec3) -> Self {
        let mut r = from.dot(to) + 1.0;
        let (x, y, z, w);
        if r < 1e-6 {
            r = 0.0;
            if from.x.abs() > from.z.abs() {
                x = -from.y;
                y = from.x;
                z = 0.0;
                w = r;
            } else {
                x = 0.0;
                y = -from.z;
                z = from.y;
                w = r;
            }
        } else {
            let c = from.cross(to);
            x = c.x;
            y = c.y;
            z = c.z;
            w = r;
        }
        let len = (x * x + y * y + z * z + w * w).sqrt();
        if len > 1e-8 {
            Self { x: x / len, y: y / len, z: z / len, w: w / len }
        } else {
            Self::IDENTITY
        }
    }

    /// Equivalent to Three.js `Euler(x, y, z, 'XYZ')` -> `Quaternion`.
    pub fn from_euler_xyz(rx: f32, ry: f32, rz: f32) -> Self {
        let (sx, cx) = (rx * 0.5).sin_cos();
        let (sy, cy) = (ry * 0.5).sin_cos();
        let (sz, cz) = (rz * 0.5).sin_cos();
        Self {
            x: sx * cy * cz + cx * sy * sz,
            y: cx * sy * cz - sx * cy * sz,
            z: cx * cy * sz + sx * sy * cz,
            w: cx * cy * cz - sx * sy * sz,
        }
    }

    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let (s, c) = (angle * 0.5).sin_cos();
        Self { x: axis.x * s, y: axis.y * s, z: axis.z * s, w: c }
    }

    pub fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        }
    }

    pub fn rotate_vec(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(v) * 2.0;
        v + t * self.w + qv.cross(t)
    }
}

/// Column-major 4x4 matrix (`cols[col][row]`), identical layout to Three.js `Matrix4.elements`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub cols: [[f32; 4]; 4],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    #[inline]
    pub fn to_cols_array_2d(self) -> [[f32; 4]; 4] {
        self.cols
    }

    pub fn from_translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[3] = [t.x, t.y, t.z, 1.0];
        m
    }

    pub fn from_scale(s: Vec3) -> Self {
        Self {
            cols: [
                [s.x, 0.0, 0.0, 0.0],
                [0.0, s.y, 0.0, 0.0],
                [0.0, 0.0, s.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn from_quat(q: Quat) -> Self {
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, xy, xz) = (x * x2, x * y2, x * z2);
        let (yy, yz, zz) = (y * y2, y * z2, z * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);
        Self {
            cols: [
                [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
                [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
                [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn from_euler_xyz(rx: f32, ry: f32, rz: f32) -> Self {
        Self::from_quat(Quat::from_euler_xyz(rx, ry, rz))
    }

    /// Composes `T * R * S` matching Three.js `Object3D.updateMatrix()`.
    pub fn from_trs(pos: Vec3, rot: Quat, scale: Vec3) -> Self {
        let mut m = Self::from_quat(rot);
        for r in 0..3 {
            m.cols[0][r] *= scale.x;
            m.cols[1][r] *= scale.y;
            m.cols[2][r] *= scale.z;
        }
        m.cols[3] = [pos.x, pos.y, pos.z, 1.0];
        m
    }

    pub fn from_trs_euler(pos: Vec3, euler_xyz: Vec3, scale: Vec3) -> Self {
        Self::from_trs(pos, Quat::from_euler_xyz(euler_xyz.x, euler_xyz.y, euler_xyz.z), scale)
    }

    pub fn mul(self, rhs: Self) -> Self {
        let mut out = [[0.0f32; 4]; 4];
        for c in 0..4 {
            for r in 0..4 {
                out[c][r] = self.cols[0][r] * rhs.cols[c][0]
                    + self.cols[1][r] * rhs.cols[c][1]
                    + self.cols[2][r] * rhs.cols[c][2]
                    + self.cols[3][r] * rhs.cols[c][3];
            }
        }
        Self { cols: out }
    }

    #[inline]
    pub fn transform_point(self, p: Vec3) -> Vec3 {
        Vec3::new(
            self.cols[0][0] * p.x + self.cols[1][0] * p.y + self.cols[2][0] * p.z + self.cols[3][0],
            self.cols[0][1] * p.x + self.cols[1][1] * p.y + self.cols[2][1] * p.z + self.cols[3][1],
            self.cols[0][2] * p.x + self.cols[1][2] * p.y + self.cols[2][2] * p.z + self.cols[3][2],
        )
    }

    #[inline]
    pub fn transform_vec(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.cols[0][0] * v.x + self.cols[1][0] * v.y + self.cols[2][0] * v.z,
            self.cols[0][1] * v.x + self.cols[1][1] * v.y + self.cols[2][1] * v.z,
            self.cols[0][2] * v.x + self.cols[1][2] * v.y + self.cols[2][2] * v.z,
        )
    }

    /// Projects a world point to clip-space NDC `(x in [-1,1], y in [-1,1], z in [0,1])`.
    pub fn project_point(self, p: Vec3) -> Vec3 {
        let x = self.cols[0][0] * p.x + self.cols[1][0] * p.y + self.cols[2][0] * p.z + self.cols[3][0];
        let y = self.cols[0][1] * p.x + self.cols[1][1] * p.y + self.cols[2][1] * p.z + self.cols[3][1];
        let z = self.cols[0][2] * p.x + self.cols[1][2] * p.y + self.cols[2][2] * p.z + self.cols[3][2];
        let w = self.cols[0][3] * p.x + self.cols[1][3] * p.y + self.cols[2][3] * p.z + self.cols[3][3];
        let inv_w = if w.abs() > 1e-8 { 1.0 / w } else { 1.0 };
        Vec3::new(x * inv_w, y * inv_w, z * inv_w)
    }

    #[inline]
    pub fn translation(self) -> Vec3 {
        Vec3::new(self.cols[3][0], self.cols[3][1], self.cols[3][2])
    }

    /// Right-handed look-at view matrix (camera looks down -Z, +Y up), matching Three.js.
    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let z = (eye - target).normalize();
        let x = up.cross(z).normalize();
        let y = z.cross(x);
        Self {
            cols: [
                [x.x, y.x, z.x, 0.0],
                [x.y, y.y, z.y, 0.0],
                [x.z, y.z, z.z, 0.0],
                [-x.dot(eye), -y.dot(eye), -z.dot(eye), 1.0],
            ],
        }
    }

    /// Right-handed perspective frustum with [0, 1] depth (D3D11 / Metal), matching
    /// Three.js `PerspectiveCamera.setViewOffset(full_w, full_h, off_x, off_y, sub_w, sub_h)`.
    pub fn perspective_rh_zo_offset(
        fov_deg: f32,
        full_w: f32,
        full_h: f32,
        off_x: f32,
        off_y: f32,
        sub_w: f32,
        sub_h: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let top = near * (fov_deg * 0.5 * PI / 180.0).tan();
        let height = 2.0 * top;
        let width = (full_w / full_h) * height;
        let left = -0.5 * width + off_x * width / full_w;
        let top_sub = top - off_y * height / full_h;
        let w = sub_w * width / full_w;
        let h = sub_h * height / full_h;
        let right = left + w;
        let bottom = top_sub - h;
        Self::frustum_rh_zo(left, right, bottom, top_sub, near, far)
    }

    pub fn frustum_rh_zo(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let x = 2.0 * near / (right - left);
        let y = 2.0 * near / (top - bottom);
        let a = (right + left) / (right - left);
        let b = (top + bottom) / (top - bottom);
        let c = far / (near - far);
        let d = (far * near) / (near - far);
        Self {
            cols: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [a, b, c, -1.0],
                [0.0, 0.0, d, 0.0],
            ],
        }
    }

    /// Inverse of a general 4x4 matrix (used for screen-to-ray unprojection and normal matrix).
    pub fn inverse(self) -> Option<Self> {
        let m = self.cols;
        let mut a = [[0.0f32; 8]; 4];
        for r in 0..4 {
            for c in 0..4 {
                a[r][c] = m[c][r];
            }
            a[r][4 + r] = 1.0;
        }
        for i in 0..4 {
            let mut pivot = i;
            for r in (i + 1)..4 {
                if a[r][i].abs() > a[pivot][i].abs() {
                    pivot = r;
                }
            }
            if a[pivot][i].abs() < 1e-10 {
                return None;
            }
            a.swap(i, pivot);
            let inv = 1.0 / a[i][i];
            for c in 0..8 {
                a[i][c] *= inv;
            }
            for r in 0..4 {
                if r != i {
                    let f = a[r][i];
                    for c in 0..8 {
                        a[r][c] -= f * a[i][c];
                    }
                }
            }
        }
        let mut out = [[0.0f32; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                out[c][r] = a[r][4 + c];
            }
        }
        Some(Self { cols: out })
    }

    /// Inverse-transpose of the upper-left 3x3 (stored in 4x4), matching Three.js `normalMatrix`.
    pub fn normal_matrix(self) -> Self {
        let a00 = self.cols[0][0];
        let a01 = self.cols[1][0];
        let a02 = self.cols[2][0];
        let a10 = self.cols[0][1];
        let a11 = self.cols[1][1];
        let a12 = self.cols[2][1];
        let a20 = self.cols[0][2];
        let a21 = self.cols[1][2];
        let a22 = self.cols[2][2];

        let b01 = a22 * a11 - a12 * a21;
        let b11 = -a22 * a10 + a12 * a20;
        let b21 = a21 * a10 - a11 * a20;
        let det = a00 * b01 + a01 * b11 + a02 * b21;
        if det.abs() < 1e-10 {
            return Self::IDENTITY;
        }
        let inv = 1.0 / det;
        // Columns of (M^-1)^T are rows of M^-1.
        let r0 = [b01 * inv, (-a22 * a01 + a02 * a21) * inv, (a12 * a01 - a02 * a11) * inv];
        let r1 = [b11 * inv, (a22 * a00 - a02 * a20) * inv, (-a12 * a00 + a02 * a10) * inv];
        let r2 = [b21 * inv, (-a21 * a00 + a01 * a20) * inv, (a11 * a00 - a01 * a10) * inv];
        Self {
            cols: [
                [r0[0], r0[1], r0[2], 0.0],
                [r1[0], r1[1], r1[2], 0.0],
                [r2[0], r2[1], r2[2], 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

/// Damped spring follower (matches `web/core.js` `Spring`), with settled early-exit.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
    pub hz: f32,
    pub damping: f32,
}

impl Spring {
    pub const fn new(x: f32, hz: f32, damping: f32) -> Self {
        Self { x, v: 0.0, hz, damping }
    }

    pub fn step(&mut self, target: f32, dt: f32) -> f32 {
        if (target - self.x).abs() < 1e-5 && self.v.abs() < 1e-5 {
            self.x = target;
            self.v = 0.0;
            return self.x;
        }
        let w = TAU * self.hz;
        let n = ((dt * 240.0).ceil() as u32).max(1);
        let h = dt / (n as f32);
        for _ in 0..n {
            self.v += (w * w * (target - self.x) - 2.0 * self.damping * w * self.v) * h;
            self.x += self.v * h;
        }
        self.x
    }

    pub fn kick(&mut self, dv: f32) {
        self.v += dv;
    }

    pub fn set(&mut self, x: f32) {
        self.x = x;
        self.v = 0.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Spring3 {
    pub x: Vec3,
    pub v: Vec3,
    pub hz: f32,
    pub damping: f32,
}

impl Spring3 {
    pub const fn new(x: Vec3, hz: f32, damping: f32) -> Self {
        Self { x, v: Vec3::ZERO, hz, damping }
    }

    pub fn step(&mut self, target: Vec3, dt: f32) -> Vec3 {
        if self.x.distance_sq(target) < 1e-10 && self.v.length_sq() < 1e-10 {
            self.x = target;
            self.v = Vec3::ZERO;
            return self.x;
        }
        let w = TAU * self.hz;
        let n = ((dt * 240.0).ceil() as u32).max(1);
        let h = dt / (n as f32);
        for _ in 0..n {
            self.v.x += (w * w * (target.x - self.x.x) - 2.0 * self.damping * w * self.v.x) * h;
            self.x.x += self.v.x * h;
            self.v.y += (w * w * (target.y - self.x.y) - 2.0 * self.damping * w * self.v.y) * h;
            self.x.y += self.v.y * h;
            self.v.z += (w * w * (target.z - self.x.z) - 2.0 * self.damping * w * self.v.z) * h;
            self.x.z += self.v.z * h;
        }
        self.x
    }

    pub fn set(&mut self, x: Vec3) {
        self.x = x;
        self.v = Vec3::ZERO;
    }
}

/// Catmull-Rom 3D curve matching Three.js `CatmullRomCurve3` ('centripetal' and 'catmullrom').
#[derive(Clone, Debug)]
pub struct CatmullRom3 {
    pub points: Vec<Vec3>,
    pub centripetal: bool,
    pub tension: f32,
}

impl CatmullRom3 {
    pub fn new(points: Vec<Vec3>, centripetal: bool, tension: f32) -> Self {
        Self { points, centripetal, tension }
    }

    pub fn get_point(&self, t: f32) -> Vec3 {
        let pts = &self.points;
        let l = pts.len();
        if l == 0 {
            return Vec3::ZERO;
        }
        if l == 1 {
            return pts[0];
        }
        let p = (l - 1) as f32 * clamp(t, 0.0, 1.0);
        let mut int_pt = p.floor() as usize;
        let mut weight = p - int_pt as f32;
        if weight == 0.0 && int_pt == l - 1 {
            int_pt = l - 2;
            weight = 1.0;
        }
        let p0 = if int_pt > 0 {
            pts[int_pt - 1]
        } else {
            pts[0] - pts[1] + pts[0]
        };
        let p1 = pts[int_pt];
        let p2 = pts[int_pt + 1];
        let p3 = if int_pt + 2 < l {
            pts[int_pt + 2]
        } else {
            pts[l - 1] - pts[l - 2] + pts[l - 1]
        };

        if self.centripetal {
            let mut dt0 = p0.distance_sq(p1).powf(0.25);
            let mut dt1 = p1.distance_sq(p2).powf(0.25);
            let mut dt2 = p2.distance_sq(p3).powf(0.25);
            if dt1 < 1e-4 {
                dt1 = 1.0;
            }
            if dt0 < 1e-4 {
                dt0 = dt1;
            }
            if dt2 < 1e-4 {
                dt2 = dt1;
            }
            Vec3::new(
                nonuniform_catmull_rom(p0.x, p1.x, p2.x, p3.x, dt0, dt1, dt2, weight),
                nonuniform_catmull_rom(p0.y, p1.y, p2.y, p3.y, dt0, dt1, dt2, weight),
                nonuniform_catmull_rom(p0.z, p1.z, p2.z, p3.z, dt0, dt1, dt2, weight),
            )
        } else {
            Vec3::new(
                cubic_catmull_rom(p0.x, p1.x, p2.x, p3.x, self.tension, weight),
                cubic_catmull_rom(p0.y, p1.y, p2.y, p3.y, self.tension, weight),
                cubic_catmull_rom(p0.z, p1.z, p2.z, p3.z, self.tension, weight),
            )
        }
    }

    /// Arc-length parameterized sampling matching Three.js `Curve.getSpacedPoints(divisions)`.
    pub fn get_spaced_points(&self, divisions: usize) -> Vec<Vec3> {
        let arc_divs = 200usize;
        let mut lengths = Vec::with_capacity(arc_divs + 1);
        let mut last = self.get_point(0.0);
        let mut sum = 0.0f32;
        lengths.push(0.0);
        for p in 1..=arc_divs {
            let cur = self.get_point(p as f32 / arc_divs as f32);
            sum += cur.distance(last);
            lengths.push(sum);
            last = cur;
        }
        let mut out = Vec::with_capacity(divisions + 1);
        for d in 0..=divisions {
            let u = d as f32 / divisions as f32;
            let target = u * sum;
            // Binary search in cumulative lengths.
            let mut low = 0usize;
            let mut high = arc_divs;
            while low <= high {
                let mid = (low + high) / 2;
                let diff = lengths[mid] - target;
                if diff < 0.0 {
                    low = mid + 1;
                } else if diff > 0.0 {
                    if mid == 0 {
                        break;
                    }
                    high = mid - 1;
                } else {
                    high = mid;
                    break;
                }
            }
            let i = high.min(arc_divs);
            let mapped = if (lengths[i] - target).abs() < 1e-6 || i == arc_divs {
                i as f32 / arc_divs as f32
            } else {
                let seg = lengths[i + 1] - lengths[i];
                let frac = if seg > 1e-8 { (target - lengths[i]) / seg } else { 0.0 };
                (i as f32 + frac) / arc_divs as f32
            };
            out.push(self.get_point(mapped));
        }
        out
    }
}

fn nonuniform_catmull_rom(
    x0: f32,
    x1: f32,
    x2: f32,
    x3: f32,
    dt0: f32,
    dt1: f32,
    dt2: f32,
    t: f32,
) -> f32 {
    let mut t1 = (x1 - x0) / dt0 - (x2 - x0) / (dt0 + dt1) + (x2 - x1) / dt1;
    let mut t2 = (x2 - x1) / dt1 - (x3 - x1) / (dt1 + dt2) + (x3 - x2) / dt2;
    t1 *= dt1;
    t2 *= dt1;
    let c0 = x1;
    let c1 = t1;
    let c2 = -3.0 * x1 + 3.0 * x2 - t2 - 2.0 * t1;
    let c3 = 2.0 * x1 - 2.0 * x2 + t2 + t1;
    c0 + c1 * t + c2 * t * t + c3 * t * t * t
}

fn cubic_catmull_rom(x0: f32, x1: f32, x2: f32, x3: f32, tension: f32, t: f32) -> f32 {
    let v0 = (x2 - x0) * tension;
    let v1 = (x3 - x1) * tension;
    let t2 = t * t;
    let t3 = t2 * t;
    (2.0 * x1 - 2.0 * x2 + v0 + v1) * t3
        + (-3.0 * x1 + 3.0 * x2 - 2.0 * v0 - v1) * t2
        + v0 * t
        + x1
}

/// 2D Catmull-Rom spline matching Three.js `SplineCurve.getPoints(divisions)`.
pub fn spline_2d_points(pts: &[(f32, f32)], divisions: usize) -> Vec<Vec2> {
    let l = pts.len();
    let mut out = Vec::with_capacity(divisions + 1);
    for d in 0..=divisions {
        let t = d as f32 / divisions as f32;
        let p = (l - 1) as f32 * t;
        let int_pt = (p.floor() as usize).min(l - 1);
        let weight = p - int_pt as f32;
        let p0 = pts[if int_pt == 0 { 0 } else { int_pt - 1 }];
        let p1 = pts[int_pt];
        let p2 = pts[(int_pt + 1).min(l - 1)];
        let p3 = pts[(int_pt + 2).min(l - 1)];
        out.push(Vec2::new(
            cubic_catmull_rom(p0.0, p1.0, p2.0, p3.0, 0.5, weight),
            cubic_catmull_rom(p0.1, p1.1, p2.1, p3.1, 0.5, weight),
        ));
    }
    out
}

/// Ray-triangle intersection (Möller–Trumbore) for exact HitTest matching Three.js Raycaster.
pub fn ray_triangle(orig: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let h = dir.cross(e2);
    let a = e1.dot(h);
    if a.abs() < 1e-7 {
        return None;
    }
    let f = 1.0 / a;
    let s = orig - v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * e2.dot(q);
    (t > 1e-5).then_some(t)
}

/// Ray-sphere intersection for fast bounding-sphere rejection.
pub fn ray_sphere(orig: Vec3, dir: Vec3, center: Vec3, radius: f32) -> bool {
    let oc = center - orig;
    let tca = oc.dot(dir);
    let d2 = oc.dot(oc) - tca * tca;
    if d2 > radius * radius {
        return false;
    }
    let thc = (radius * radius - d2).sqrt();
    tca + thc >= 0.0
}
