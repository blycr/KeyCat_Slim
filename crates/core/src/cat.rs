//! Procedural chibi cat model, hierarchy transforms, limbs, tail and collar —
//! 1:1 mirror of `web/cat.js`.

use crate::coats::{shine_dots, Look, ResolvedLook};
use crate::geometry::{
    cat_nose, circle, cylinder_capped, full_sphere, lathe, plane, torus, MeshData, TaperTube,
};
use crate::life::Festival;
use crate::math::{
    clamp, mix, smooth, spline_2d_points, CatmullRom3, Mat4, Quat, Vec2, Vec3, PI,
};

const TAIL_N: usize = 10;

pub const BODY_H: f32 = 1.32;
pub const HOME: Vec3 = Vec3::new(0.02, 0.0, 0.05);
pub const HEAD_R: Vec3 = Vec3::new(0.56, 0.47, 0.47);
pub const EAR_AT: Vec3 = Vec3::new(0.26, 0.21, -0.06);
pub const EAR_STRETCH: f32 = 1.12;
pub const COLLAR_Y: f32 = 1.13;
pub const COLLAR_LIFT: f32 = 0.07;

#[derive(Clone, Copy, Debug, Default)]
pub struct Bend {
    pub x: f32,
    pub z: f32,
    pub squash: f32,
}

#[derive(Clone, Debug)]
pub struct LimbMeshes {
    pub body: TaperTube,
    pub rim: TaperTube,
    pub cap_start: bool,
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub start_scale_fill: f32,
    pub start_scale_rim: f32,
    pub end_scale_fill: f32,
    pub end_scale_rim: f32,
}

impl LimbMeshes {
    pub fn new(segments: usize, radial: usize, cap_start: bool, raycast: bool) -> Self {
        Self {
            body: TaperTube::new(segments, radial, raycast),
            rim: TaperTube::new(segments, radial, false),
            cap_start,
            start_pos: Vec3::ZERO,
            end_pos: Vec3::ZERO,
            start_scale_fill: 0.05,
            start_scale_rim: 0.06,
            end_scale_fill: 0.05,
            end_scale_rim: 0.06,
        }
    }

    pub fn update<F, R, E>(&mut self, point: F, radius: R, edge_at: E)
    where
        F: FnMut(f32) -> Vec3,
        R: Fn(f32) -> f32,
        E: Fn(f32) -> f32,
    {
        TaperTube::update_pair(&mut self.body, &mut self.rim, point, &radius, &edge_at);
        self.start_pos = self.body.pts[0];
        self.end_pos = self.body.pts[self.body.segments];
        let r0 = radius(0.0);
        let r1 = radius(1.0);
        self.start_scale_fill = r0.max(0.001);
        self.start_scale_rim = (r0 + edge_at(0.0)).max(0.001);
        self.end_scale_fill = r1.max(0.001);
        self.end_scale_rim = (r1 + edge_at(1.0)).max(0.001);
    }
}

#[derive(Clone, Debug)]
pub struct EarPairGeo {
    pub key: u32, // round(k * 100)
    pub fill: MeshData,
    pub rim: MeshData,
    pub meet: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct EarState {
    pub side: f32, // -1 left, +1 right
    pub tilt: f32,
    pub meet: Vec3,
    pub pos: Vec3,
    pub rot_x: f32,
    pub rot_z: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct EyeState {
    pub side: f32,
    pub base_pos: Vec3,
    pub base_quat: Quat,
    pub scale_x: f32,
    pub scale_y: f32,
    pub closed: bool,
    pub pupil_pos: Vec2,
    pub pupil_scale: Vec2,
}

#[derive(Clone, Debug)]
pub struct ArmState {
    pub side: f32,
    pub shoulder_local: Vec3,
    pub shoulder: Vec3,
    pub hand: Vec3,
    pub c1: Vec3,
    pub c2: Vec3,
    pub paw_pos: Vec3,
    pub paw_yaw: f32,
    pub limb: LimbMeshes,
}

pub struct Cat {
    // Static / cached geometries
    pub body_geo: MeshData,
    pub paw_geo: MeshData,
    pub toe_geo: MeshData,
    pub head_geo: MeshData,
    pub disc_geo: MeshData,
    pub cap_start_geo: MeshData,
    pub cap_end_geo: MeshData,
    pub ear_geos: Vec<EarPairGeo>,
    pub active_ear_geo: usize,
    pub arc_mesh: MeshData,
    pub whiskers: Vec<MeshData>,
    pub nose_geo: MeshData,
    pub mouth_mesh: MeshData,
    // Collar & bell static geometries
    pub collar_band: LimbMeshes,
    pub bell_ball_geo: MeshData,
    pub bell_rim_geo: MeshData,
    pub bell_slit_geo: MeshData,
    pub bell_glint_geo: MeshData,
    pub bell_loop_geo: MeshData,
    // Festival hat geometries (reused across xmas / halloween / birthday)
    pub hat_cone_geo: MeshData,
    pub hat_brim_geo: MeshData,
    pub hat_ring_geo: MeshData,
    collar_r: f32,
    outline_pts: Vec<Vec3>,

    // Dynamic state & transforms
    pub look: Look,
    pub resolved: ResolvedLook,
    pub outfit: Festival,
    pub bend: Bend,
    pub body_pos: Vec3,
    pub body_rot: Vec3,
    pub body_scale: Vec3,
    pub head_pivot_pos: Vec3,
    pub head_pivot_rot: Vec3,
    pub head_roll: f32,
    pub collar_pos: Vec3,
    pub collar_rot: Vec3,
    pub collar_scale: Vec3,
    pub bell_rot: Vec3,
    pub ears: [EarState; 2],
    pub eyes: [EyeState; 2],
    pub blush_opacity: f32,
    pub yawn_open: f32,
    pub lick_out: f32,
    pub tail_curve: CatmullRom3,
    tail_chain: Vec<Vec3>,
    tail_prev: Vec<Vec3>,
    tail_link_len: f32,
    tail_chain_fresh: bool,
    pub tail: LimbMeshes,
    pub arms: [ArmState; 2],
    pub dynamic_dirty: bool,
}

impl Cat {
    pub fn new() -> Self {
        let ctrl = [
            (0.0, 0.0), (0.44, 0.005), (0.58, 0.08), (0.63, 0.26), (0.615, 0.48),
            (0.565, 0.7), (0.5, 0.9), (0.43, 1.06), (0.34, 1.2), (0.2, 1.29), (0.0, BODY_H),
        ]
        .iter()
        .map(|&(r, y)| Vec3::new(r, y, 0.0))
        .collect();
        let outline = CatmullRom3::new(ctrl, true, 0.5);
        let mut profile: Vec<Vec2> = outline
            .get_spaced_points(48)
            .into_iter()
            .map(|q| Vec2::new(q.x.max(0.0), q.y))
            .collect();
        profile[0].x = 0.0;
        if let Some(last) = profile.last_mut() {
            last.x = 0.0;
        }
        let body_geo = lathe(&profile, 72);
        let outline_pts = outline.get_spaced_points(400);
        let collar_r = radius_at_pts(&outline_pts, COLLAR_Y);

        let mut paw_geo = full_sphere(1.0, 32, 20);
        for v in &mut paw_geo.vertices {
            let (x, y, z) = (v.pos[0], v.pos[1], v.pos[2]);
            v.pos = [x * (1.0 + 0.12 * z), if y < 0.0 { y * 0.45 } else { y }, z];
            v.rest = v.pos;
        }
        paw_geo.compute_vertex_normals();
        paw_geo.compute_bounds();

        let mut toe_geo = plane(0.03, 0.34);
        toe_geo.rotate_x(-PI * 0.5);

        let mut head_geo = full_sphere(1.0, 64, 44);
        for v in &mut head_geo.vertices {
            let (x, y, z) = (v.pos[0], v.pos[1], v.pos[2]);
            let cheek = 1.0 + 0.08 * (-(((y + 0.38) / 0.4).powi(2))).exp() - 0.08 * (y + 0.2);
            v.pos = [x * cheek, if y > 0.0 { y * (1.0 - 0.06 * y) } else { y * 0.96 }, z];
            v.rest = v.pos;
        }
        head_geo.compute_vertex_normals();
        head_geo.compute_bounds();

        let disc_geo = circle(1.0, 36, 0.0, PI * 2.0);
        let mut cap_start_geo = full_sphere(1.0, 20, 14);
        cap_start_geo.fill_along(0.0);
        let mut cap_end_geo = full_sphere(1.0, 20, 14);
        cap_end_geo.fill_along(1.0);

        let ear_default = build_ear_geo(1.0);
        let meet0 = ear_default.meet;

        let mut arc_tube = TaperTube::new(16, 6, false);
        arc_tube.update(
            |t| Vec3::new(mix(-0.12, 0.12, t), -0.045 * (t * PI).sin(), 0.004),
            |t| 0.011 + 0.006 * (t * PI).sin(),
            0.0,
        );

        let mut whiskers = Vec::with_capacity(6);
        for &s in &[-1.0f32, 1.0] {
            for i in 0..3 {
                let y0 = -0.17 - i as f32 * 0.04;
                let a = Vec3::new(s * 0.31, y0, 0.4);
                let c = Vec3::new(s * 0.54, y0 + 0.012 - i as f32 * 0.014, 0.29);
                let b = Vec3::new(s * 0.74, y0 - 0.015 - i as f32 * 0.055, 0.13);
                let mut tube = TaperTube::new(10, 4, false);
                tube.update(
                    |t| {
                        let u = 1.0 - t;
                        a * (u * u) + c * (2.0 * u * t) + b * (t * t)
                    },
                    |t| 0.006 * (1.0 - 0.5 * t),
                    0.0,
                );
                whiskers.push(tube.mesh);
            }
        }

        let nose_geo = cat_nose();
        let ny = -0.15;
        let mut mouth_tube = TaperTube::new(24, 4, false);
        mouth_tube.update(
            |t| {
                let x = mix(-0.07, 0.07, t);
                let y = ny - 0.035 - 0.022 * ((x.abs() / 0.07) * PI).sin();
                Vec3::new(x, y, face_z(x, y) + 0.006)
            },
            |_| 0.0055,
            0.0,
        );

        let mut collar_band = LimbMeshes::new(40, 8, true, false);
        collar_band.update(
            |t| {
                let a = mix(-1.3, 1.3, t);
                Vec3::new(
                    a.sin() * (collar_r + 0.004),
                    0.012 * (1.0 - a.cos()),
                    a.cos() * (collar_r + 0.004) * 0.82,
                )
            },
            |t| 0.032 * (0.25 + 0.75 * smooth(0.0, 0.22, t.min(1.0 - t))),
            |t| 0.01 * smooth(0.0, 0.22, t.min(1.0 - t)),
        );
        let r_bell = 0.064;
        let bell_ball_geo = full_sphere(r_bell, 24, 16);
        let bell_rim_geo = full_sphere(r_bell + 0.012, 24, 16);
        let bell_slit_geo = plane(0.07, 0.013);
        let bell_glint_geo = circle(0.015, 12, 0.0, PI * 2.0);
        let bell_loop_geo = torus(0.02, 0.007, 8, 16);

        // Unit festival hat primitives (scaled per outfit in renderer)
        let mut hat_cone_geo = cylinder_capped(0.0, 1.0, 1.0, 28);
        hat_cone_geo.translate(0.0, 0.5, 0.0);
        let hat_brim_geo = cylinder_capped(1.0, 1.0, 1.0, 32);
        let mut hat_ring_geo = torus(1.0, 0.22, 10, 28);
        hat_ring_geo.rotate_x(PI * 0.5);

        let eyes = [-1.0f32, 1.0].map(|s| {
            let lx = s * 0.235;
            let ly = 0.04;
            let lz = face_z(lx, ly);
            let normal = Vec3::new(
                lx / (HEAD_R.x * HEAD_R.x),
                ly / (HEAD_R.y * HEAD_R.y),
                lz / (HEAD_R.z * HEAD_R.z),
            )
            .normalize();
            EyeState {
                side: s,
                base_pos: Vec3::new(lx, ly, lz + 0.005),
                base_quat: Quat::from_unit_vectors(Vec3::new(0.0, 0.0, 1.0), normal),
                scale_x: 1.0,
                scale_y: 1.0,
                closed: false,
                pupil_pos: Vec2::new(0.0, 0.0),
                pupil_scale: Vec2::new(0.05, 0.11),
            }
        });

        let mut ears = [-1.0f32, 1.0].map(|s| EarState {
            side: s,
            tilt: ear_tilt(1.0),
            meet: Vec3::new(s * meet0[0], meet0[1], 0.0),
            pos: Vec3::new(s * EAR_AT.x, EAR_AT.y, EAR_AT.z),
            rot_x: -0.12,
            rot_z: -s * ear_tilt(1.0),
        });
        for e in &mut ears {
            turn_ear_state(e, -e.side * e.tilt, -0.12);
        }

        let tail_pts = tail_rest_pts();
        let tail_curve = CatmullRom3::new(tail_pts.to_vec(), false, 0.5);
        let sampled_tail = tail_curve.get_spaced_points(TAIL_N);
        let mut total_tail_len = 0.0f32;
        for w in sampled_tail.windows(2) {
            total_tail_len += w[0].distance(w[1]);
        }
        let tail_link_len = (total_tail_len / TAIL_N as f32).max(0.05);
        let tail = LimbMeshes::new(28, 14, false, true);

        let arms = [-1.0f32, 1.0].map(|s| ArmState {
            side: s,
            shoulder_local: Vec3::new(s * 0.27, 0.94, 0.22),
            shoulder: Vec3::ZERO,
            hand: Vec3::ZERO,
            c1: Vec3::ZERO,
            c2: Vec3::ZERO,
            paw_pos: Vec3::new(s * 0.36, 0.26, 0.68),
            paw_yaw: 0.0,
            limb: LimbMeshes::new(24, 18, true, true),
        });

        let look = Look::default();
        let resolved = look.resolve();
        let mut cat = Self {
            body_geo,
            paw_geo,
            toe_geo,
            head_geo,
            disc_geo,
            cap_start_geo,
            cap_end_geo,
            ear_geos: vec![ear_default],
            active_ear_geo: 0,
            arc_mesh: arc_tube.mesh,
            whiskers,
            nose_geo,
            mouth_mesh: mouth_tube.mesh,
            collar_band,
            bell_ball_geo,
            bell_rim_geo,
            bell_slit_geo,
            bell_glint_geo,
            bell_loop_geo,
            hat_cone_geo,
            hat_brim_geo,
            hat_ring_geo,
            collar_r,
            outline_pts,
            look,
            resolved,
            outfit: Festival::None,
            bend: Bend::default(),
            body_pos: HOME,
            body_rot: Vec3::ZERO,
            body_scale: Vec3::ONE,
            head_pivot_pos: Vec3::new(0.0, BODY_H - 0.12, 0.06),
            head_pivot_rot: Vec3::new(-0.28, 0.0, 0.0),
            head_roll: 0.0,
            collar_pos: Vec3::new(0.0, COLLAR_Y, 0.0),
            collar_rot: Vec3::ZERO,
            collar_scale: Vec3::ONE,
            bell_rot: Vec3::ZERO,
            ears,
            eyes,
            blush_opacity: 0.0,
            yawn_open: 0.0,
            lick_out: 0.0,
            tail_curve,
            tail_chain: vec![Vec3::ZERO; TAIL_N + 1],
            tail_prev: vec![Vec3::ZERO; TAIL_N + 1],
            tail_link_len,
            tail_chain_fresh: true,
            tail,
            arms,
            dynamic_dirty: true,
        };
        cat.place_collar();
        cat
    }

    pub fn set_look(&mut self, look: Look) {
        self.resolved = look.resolve();
        self.look = look;
        let k = self.resolved.ear_scale;
        let key = (k * 100.0).round() as u32;
        let idx = if let Some(pos) = self.ear_geos.iter().position(|g| g.key == key) {
            pos
        } else {
            self.ear_geos.push(build_ear_geo(k));
            self.ear_geos.len() - 1
        };
        self.active_ear_geo = idx;
        let meet = self.ear_geos[idx].meet;
        let tilt = ear_tilt(k);
        for e in &mut self.ears {
            e.tilt = tilt;
            e.meet = Vec3::new(e.side * meet[0], meet[1], 0.0);
            turn_ear_state(e, -e.side * e.tilt, -0.12);
        }
        self.place_collar();
    }

    pub fn active_ear(&self) -> &EarPairGeo {
        &self.ear_geos[self.active_ear_geo]
    }

    pub fn body_world(&self) -> Mat4 {
        Mat4::from_trs_euler(self.body_pos, self.body_rot, self.body_scale)
    }

    pub fn head_pivot_world(&self) -> Mat4 {
        self.body_world().mul(Mat4::from_trs_euler(
            self.head_pivot_pos,
            self.head_pivot_rot,
            Vec3::ONE,
        ))
    }

    pub fn head_world(&self) -> Mat4 {
        self.head_pivot_world().mul(Mat4::from_trs_euler(
            Vec3::new(0.0, 0.42, 0.04),
            Vec3::new(0.0, 0.0, self.head_roll),
            Vec3::ONE,
        ))
    }

    pub fn collar_world(&self) -> Mat4 {
        self.body_world().mul(Mat4::from_trs_euler(
            self.collar_pos,
            self.collar_rot,
            self.collar_scale,
        ))
    }

    pub fn bell_hang_world(&self) -> Mat4 {
        let r_bell = 0.064;
        let bell = Mat4::from_trs_euler(
            Vec3::new(0.0, -0.012, (self.collar_r + 0.04) * 0.82),
            self.bell_rot,
            Vec3::ONE,
        );
        let hang = Mat4::from_translation(Vec3::new(0.0, -(r_bell + 0.018), 0.0));
        self.collar_world().mul(bell).mul(hang)
    }

    pub fn ear_world(&self, i: usize) -> Mat4 {
        let e = &self.ears[i];
        self.head_world().mul(Mat4::from_trs_euler(
            e.pos,
            Vec3::new(e.rot_x, 0.0, e.rot_z),
            Vec3::new(1.0, EAR_STRETCH, 1.0),
        ))
    }

    pub fn eye_world(&self, i: usize) -> Mat4 {
        let e = &self.eyes[i];
        self.head_world().mul(Mat4::from_trs(
            e.base_pos,
            e.base_quat,
            Vec3::new(e.scale_x, e.scale_y, 1.0),
        ))
    }

    pub fn nose_world(&self) -> Mat4 {
        let ny = -0.15;
        let nz = face_z(0.0, ny);
        let n = Vec3::new(0.0, ny / (HEAD_R.y * HEAD_R.y), nz / (HEAD_R.z * HEAD_R.z)).normalize();
        let q = Quat::from_unit_vectors(Vec3::new(0.0, 0.0, 1.0), n);
        self.head_world().mul(Mat4::from_trs(Vec3::new(0.0, ny, nz + 0.006), q, Vec3::ONE))
    }

    /// World transform for the chibi open-mouth yawn oval on the face.
    pub fn yawn_world(&self) -> Mat4 {
        let yy = -0.25;
        let yz = face_z(0.0, yy);
        let n = Vec3::new(0.0, yy / (HEAD_R.y * HEAD_R.y), yz / (HEAD_R.z * HEAD_R.z)).normalize();
        let q = Quat::from_unit_vectors(Vec3::new(0.0, 0.0, 1.0), n);
        let open = clamp(self.yawn_open, 0.0, 1.0);
        let sx = 0.095 * (0.75 + 0.35 * open);
        let sy = 0.115 * open.max(0.01);
        self.head_world().mul(Mat4::from_trs(
            Vec3::new(0.0, yy - 0.03 * open, yz + 0.009),
            q,
            Vec3::new(sx, sy, 1.0),
        ))
    }

    /// World transform for the pink tongue tip sticking out while grooming.
    pub fn lick_world(&self) -> Mat4 {
        let ly = -0.21;
        let lz = face_z(0.0, ly);
        let n = Vec3::new(0.0, ly / (HEAD_R.y * HEAD_R.y), lz / (HEAD_R.z * HEAD_R.z)).normalize();
        let q = Quat::from_unit_vectors(Vec3::new(0.0, 0.0, 1.0), n);
        let out = clamp(self.lick_out, 0.0, 1.0);
        self.head_world().mul(Mat4::from_trs(
            Vec3::new(0.0, ly - 0.04 * out, lz + 0.011),
            q,
            Vec3::new(0.048, 0.068 * out.max(0.01), 1.0),
        ))
    }

    /// World transform for the festival hat anchored on top of the head.
    pub fn hat_root_world(&self) -> Mat4 {
        let (x, y, rz) = match self.outfit {
            Festival::Xmas => (0.06, 0.38, -0.34),
            Festival::Halloween => (0.04, 0.42, -0.22),
            Festival::Birthday => (0.05, 0.40, -0.28),
            _ => (0.0, 0.40, 0.0),
        };
        self.head_world().mul(Mat4::from_trs_euler(
            Vec3::new(x, y, -0.02),
            Vec3::new(-0.08, 0.0, rz),
            Vec3::ONE,
        ))
    }

    pub fn hind_paw_world(&self, side: f32, rim: bool) -> Mat4 {
        let scale = if rim {
            Vec3::new(0.186, 0.114, 0.166)
        } else {
            Vec3::new(0.17, 0.1, 0.15)
        };
        self.body_world().mul(Mat4::from_trs_euler(
            Vec3::new(side * 0.44, 0.06, 0.4),
            Vec3::new(0.0, side * 0.25, 0.0),
            scale,
        ))
    }

    pub fn arm_paw_world(&self, i: usize, rim: bool) -> Mat4 {
        let a = &self.arms[i];
        let scale = if rim {
            Vec3::new(0.149, 0.097, 0.144)
        } else {
            Vec3::new(0.135, 0.085, 0.13)
        };
        Mat4::from_trs_euler(a.paw_pos, Vec3::new(0.0, a.paw_yaw, 0.0), scale)
    }

    pub fn attach(&self, local: Vec3) -> Vec3 {
        let w = spine_weight(local.y);
        let bent = Vec3::new(
            local.x + self.bend.x * w,
            local.y * (1.0 - self.bend.squash * sink(local.y)),
            local.z + self.bend.z * w,
        );
        self.body_world().transform_point(bent)
    }

    pub fn place_head(&mut self) {
        let neck = Vec3::new(0.0, BODY_H - 0.12, 0.06);
        let w = spine_weight(neck.y);
        self.head_pivot_pos = Vec3::new(
            neck.x + self.bend.x * w,
            neck.y * (1.0 - self.bend.squash),
            neck.z + self.bend.z * w,
        );
    }

    pub fn place_collar(&mut self) {
        if !self.resolved.collar_visible && self.outfit != Festival::Cny {
            return;
        }
        let y = COLLAR_Y - 0.3 * clamp(self.head_pivot_rot.x + 0.1, 0.0, 0.6);
        let s = radius_at_pts(&self.outline_pts, y) / self.collar_r;
        self.collar_scale = Vec3::new(s, 1.0, s);
        let w = spine_weight(y);
        let dw = (spine_weight(y + 0.01) - spine_weight(y - 0.01)) / 0.02;
        self.collar_pos = Vec3::new(
            self.bend.x * w,
            y * (1.0 - self.bend.squash * sink(y)),
            self.bend.z * w,
        );
        let tx = clamp((self.bend.z * dw).atan() * 0.5, -0.18, 0.18);
        let tz = clamp(-(self.bend.x * dw).atan() * 0.5, -0.18, 0.18);
        self.collar_rot = Vec3::new(tx, 0.0, tz);
        self.bell_rot = Vec3::new(-tx, 0.0, -tz);
    }

    pub fn turn_ear(&mut self, i: usize, z: f32, x: f32) {
        turn_ear_state(&mut self.ears[i], z, x);
    }

    pub fn update_arm(&mut self, i: usize, paw_pos: Vec3, sprawl: f32) {
        let shoulder = self.attach(self.arms[i].shoulder_local);
        let arm = &mut self.arms[i];
        arm.shoulder = shoulder;
        arm.hand = paw_pos;
        let a = arm.shoulder;
        // Tuck the wrist end slightly inside the paw mesh so the arm tube never pokes out.
        let to_shoulder = (a - paw_pos).normalize();
        let b = paw_pos + Vec3::new(to_shoulder.x * 0.035, 0.015, to_shoulder.z * 0.035);
        let reach = a.distance(b);
        let fold = clamp(1.4 - reach / 0.9, 0.25, 1.0);
        let swing = smooth(0.7, 1.3, reach);
        let mut c1 = Vec3::new(
            a.x + arm.side * 0.1 * fold + (b.x - a.x) * mix(0.15, 0.4, swing),
            a.y - reach * mix(0.32, 0.2, swing),
            a.z + 0.06 + (b.z - a.z) * 0.25,
        );
        let mut c2 = Vec3::new(
            b.x - (b.x - a.x) * 0.12,
            b.y + reach * 0.3,
            b.z - (b.z - a.z) * 0.18,
        );
        // When sprawling / lying forward, bend cleanly at an outer elbow resting near the desk.
        if sprawl > 0.01 {
            let elbow = Vec3::new(
                mix(a.x, b.x, 0.48) + arm.side * 0.22,
                mix(a.y, b.y, 0.65).max(0.24),
                mix(a.z, b.z, 0.45),
            );
            let k = clamp(sprawl, 0.0, 1.0);
            c1 = c1.lerp(a.lerp(elbow, 0.85), k);
            c2 = c2.lerp(b.lerp(elbow, 0.78), k);
        }
        arm.c1 = c1;
        arm.c2 = c2;
        arm.limb.update(
            |t| {
                let u = 1.0 - t;
                a * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + b * (t * t * t)
            },
            |t| mix(0.135, 0.088, smooth(0.0, 0.76, t)) + 0.008 * smooth(0.82, 1.0, t),
            |t| mix(-0.03, 0.012, smooth(0.14, 0.42, t)),
        );
        arm.paw_yaw = (paw_pos.x - c2.x).atan2(paw_pos.z - c2.z);
        arm.paw_pos = paw_pos;
        self.dynamic_dirty = true;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_tail(
        &mut self,
        time: f32,
        sway: f32,
        sleep: f32,
        thump: f32,
        lift: f32,
        lash: f32,
        reach: f32,
        reach_tip: Option<Vec3>,
        wrap: f32,
        dt: f32,
    ) {
        let rest_pts = tail_rest_pts();
        let sleep_pts = tail_sleep_pts();
        let thump_pts = tail_thump_pts();
        let n = rest_pts.len();
        let curl = sleep.max(wrap);
        let mut want_pts = Vec::with_capacity(n);
        for i in 0..n {
            let w = i as f32 / (n - 1) as f32;
            let mut local = rest_pts[i].lerp(sleep_pts[i], curl);
            if thump > 0.0 {
                let mut tt = thump_pts[i];
                tt.y += lift * 1.05 * w.powf(1.4);
                tt.x -= lift * 0.22 * w * w;
                tt.z += lash * 0.35 * w.powf(1.5);
                local = local.lerp(tt, thump);
            }
            let attached = self.attach(local);
            let loose = local + HOME + (self.body_pos - HOME) * 0.3;
            want_pts.push(attached.lerp(loose, smooth(0.1, 0.7, w)));
        }
        if reach > 0.0 {
            if let Some(tip) = reach_tip {
                let root = want_pts[0];
                let reach_path = [
                    Vec3::new(root.x + 0.36, 0.3, root.z + 0.14),
                    Vec3::new(tip.x + 0.34, 0.31, mix(root.z, tip.z, 0.55)),
                    Vec3::new(tip.x + 0.24, 0.33, tip.z - 0.06),
                    Vec3::new(tip.x + 0.11, mix(0.33, tip.y + 0.02, 0.5), tip.z + 0.02),
                    tip,
                ];
                for i in 1..want_pts.len() {
                    want_pts[i] = want_pts[i].lerp(reach_path[i - 1], reach * if i == 1 { 0.6 } else { 1.0 });
                }
            }
        }

        // Fixed-length chain with travelling wave & Verlet inertia so the tail never stretches.
        let want_curve = CatmullRom3::new(want_pts, false, 0.5);
        let sampled = want_curve.get_spaced_points(TAIL_N);
        let mut goal = vec![Vec3::ZERO; TAIL_N + 1];
        goal[0] = sampled[0];
        let y_up = Vec3::new(0.0, 1.0, 0.0);
        let mut bend_q = Quat::IDENTITY;
        for i in 1..=TAIL_N {
            let w = i as f32 / TAIL_N as f32;
            let dir = (sampled[i] - sampled[i - 1]).normalize();
            let q_yaw = Quat::from_axis_angle(
                y_up,
                0.05 * sway * (time * 1.6 - i as f32 * 0.45).sin(),
            );
            bend_q = q_yaw.mul(bend_q);
            let mut axis = dir.cross(y_up);
            if axis.length_sq() < 1e-4 {
                axis = Vec3::new(0.0, 0.0, 1.0);
            } else {
                axis = axis.normalize();
            }
            let q_curl = Quat::from_axis_angle(
                axis,
                0.06 * sway * w * w * (time * 1.3 - i as f32 * 0.6).sin(),
            );
            bend_q = q_curl.mul(bend_q);
            let step_dir = bend_q.rotate_vec(dir);
            goal[i] = goal[i - 1] + step_dir * self.tail_link_len;
        }

        if self.tail_chain_fresh
            || dt > 0.25
            || self.tail_chain[0].distance_sq(goal[0]) > 0.04
        {
            self.tail_chain.copy_from_slice(&goal);
            self.tail_prev.copy_from_slice(&goal);
            self.tail_chain_fresh = false;
        } else {
            let f = (dt * 60.0).min(3.0);
            let keep = 0.86f32.powf(f);
            self.tail_chain[0] = goal[0];
            self.tail_prev[0] = goal[0];
            for i in 1..=TAIL_N {
                let prev = self.tail_prev[i];
                let mut q = self.tail_chain[i];
                let vel = (q - prev) * keep;
                self.tail_prev[i] = q;
                let pull = 1.0 - (1.0 - mix(0.5, 0.17, i as f32 / TAIL_N as f32)).powf(f);
                q = (q + vel).lerp(goal[i], pull);
                let anchor = self.tail_chain[i - 1];
                let d = q - anchor;
                let len = d.length();
                q = if len > 1e-5 {
                    anchor + d * (self.tail_link_len / len)
                } else {
                    goal[i]
                };
                if q.y < 0.05 {
                    q.y = 0.05;
                }
                self.tail_chain[i] = q;
            }
        }

        // Blend toward exact reach_tip during Space bar tail-pat so the tip hits the keycap cleanly.
        let mut pts = Vec::with_capacity(TAIL_N + 1);
        for i in 0..=TAIL_N {
            let mut p = self.tail_chain[i];
            if reach > 0.0 {
                p = p.lerp(sampled[i], reach);
            }
            pts.push(p);
        }
        self.tail_curve.points = pts;
        let curve = &self.tail_curve;
        self.tail.update(
            |t| curve.get_point(t),
            |t| mix(0.085, 0.068, t) + 0.014 * smooth(0.8, 1.0, t),
            |_| 0.022,
        );
        self.dynamic_dirty = true;
    }

    pub fn tail_tip(&self) -> Vec3 {
        self.tail_curve.get_point(1.0)
    }

    pub fn shine_dots(&self) -> &'static [[f32; 3]] {
        shine_dots(&self.look.shine)
    }

    pub fn raycast_pet(&self, orig: Vec3, dir: Vec3) -> bool {
        let head_m = self.head_world().mul(Mat4::from_scale(HEAD_R));
        if self.head_geo.raycast(head_m, orig, dir) {
            return true;
        }
        let ear_fill = &self.active_ear().fill;
        for i in 0..2 {
            let em = self.ear_world(i).mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.5)));
            if ear_fill.raycast(em, orig, dir) {
                return true;
            }
        }
        false
    }

    pub fn raycast_drag(&self, orig: Vec3, dir: Vec3) -> bool {
        let body_m = self.body_world().mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.82)));
        if self.body_geo.raycast(body_m, orig, dir) {
            return true;
        }
        if self.tail.body.mesh.raycast(Mat4::IDENTITY, orig, dir) {
            return true;
        }
        for i in 0..2 {
            if self.arms[i].limb.body.mesh.raycast(Mat4::IDENTITY, orig, dir) {
                return true;
            }
            if self.paw_geo.raycast(self.arm_paw_world(i, false), orig, dir) {
                return true;
            }
        }
        for &s in &[-1.0f32, 1.0] {
            if self.paw_geo.raycast(self.hind_paw_world(s, false), orig, dir) {
                return true;
            }
        }
        false
    }
}

pub fn spine_weight(y: f32) -> f32 {
    smooth(0.15, BODY_H, y).powf(1.6)
}

pub fn sink(y: f32) -> f32 {
    smooth(0.2, BODY_H, y)
}

fn radius_at_pts(pts: &[Vec3], y: f32) -> f32 {
    let mut best = pts[0];
    let mut best_d = (best.y - y).abs();
    for &q in &pts[1..] {
        let d = (q.y - y).abs();
        if d < best_d {
            best = q;
            best_d = d;
        }
    }
    best.x
}

fn face_z(x: f32, y: f32) -> f32 {
    HEAD_R.z * (1.0 - (x / HEAD_R.x).powi(2) - (y / HEAD_R.y).powi(2)).max(0.0).sqrt()
}

fn ear_tilt(k: f32) -> f32 {
    0.2 + 0.5 * (k - 1.0)
}

fn smax(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (a - b) / k, 0.0, 1.0);
    mix(b, a, h) + k * h * (1.0 - h)
}

fn head_in_ear(k: f32, grow: f32) -> Vec<(f32, f32)> {
    let t = ear_tilt(k);
    let (sn, c) = t.sin_cos();
    let mut out = Vec::with_capacity(201);
    for i in 0..=200 {
        let y = -0.6 + 1.6 * i as f32 / 200.0;
        let cheek = 1.0 + 0.08 * (-(((y + 0.38) / 0.4).powi(2))).exp() - 0.08 * (y + 0.2);
        let dx = (HEAD_R.x + grow) * cheek * (1.0 - y * y).max(0.0).sqrt() - EAR_AT.x;
        let dy = (HEAD_R.y + grow) * (if y > 0.0 { y * (1.0 - 0.06 * y) } else { y * 0.96 }) - EAR_AT.y;
        out.push((c * dx - sn * dy, (sn * dx + c * dy) / EAR_STRETCH));
    }
    out
}

fn eval_head_in_ear(pts: &[(f32, f32)], y: f32) -> f32 {
    for i in 1..pts.len() {
        let (r0, y0) = pts[i - 1];
        let (r1, y1) = pts[i];
        if (y0 - y) * (y1 - y) <= 0.0 && y0 != y1 {
            return r0 + (r1 - r0) * (y - y0) / (y1 - y0);
        }
    }
    0.0
}

fn ear_profile(k: f32, grow: f32) -> (Vec<(f32, f32)>, [f32; 2], f32) {
    let ear_top: [(f32, f32); 6] = [
        (0.225, 0.1), (0.185, 0.19), (0.135, 0.27), (0.08, 0.33), (0.035, 0.36), (0.0, 0.37),
    ];
    let n = (ear_top.len() - 1) as f32;
    let top: Vec<(f32, f32)> = ear_top
        .iter()
        .enumerate()
        .map(|(i, &(r, y))| {
            let br = r * (1.0 + (k - 1.0) * smooth(-0.06, 0.2, y));
            let by = -0.06 + (y + 0.06) * (1.0 + 0.6 * (k - 1.0));
            let nr = if br != 0.0 { br + grow } else { 0.0 };
            let ny = by + grow * (i as f32 / n).powi(2);
            (nr, ny)
        })
        .collect();
    let head_pts = head_in_ear(k, if grow != 0.0 { 0.024 } else { 0.0 });
    let (ra, ya) = top[0];
    let (rb, yb) = top[1];
    let edge = |y: f32| ra + (rb - ra) * (y - ya) / (yb - ya);
    let mut meet = ya;
    while meet > -0.2 && edge(meet) > eval_head_in_ear(&head_pts, meet) {
        meet -= 0.002;
    }
    let side = |y: f32| {
        smax(
            edge(y),
            eval_head_in_ear(&head_pts, y) - 0.006 - 0.07 * smooth(meet - 0.01, meet - 0.13, y),
            0.03,
        )
    };
    let mut pts = vec![(0.0, -0.25), (0.2, -0.24)];
    for i in 0..16 {
        let y = -0.2 + (ya + 0.2) * i as f32 / 16.0;
        pts.push((side(y), y));
    }
    pts.extend_from_slice(&top);
    (pts, [side(meet - 0.03), meet - 0.03], meet)
}

fn ear_lathe(pts: &[(f32, f32)], join: f32) -> MeshData {
    let sampled = spline_2d_points(pts, 48);
    let mut g = lathe(&sampled, 40);
    for v in &mut g.vertices {
        let z = v.pos[2] * mix(1.0, 0.3, smooth(join, join - 0.1, v.pos[1]));
        v.pos[2] = z;
    }
    g
}

fn build_ear_geo(k: f32) -> EarPairGeo {
    let (p0, meet, j0) = ear_profile(k, 0.0);
    let (p1, _, j1) = ear_profile(k, 0.026);
    let (b0, _, bj0) = ear_profile(1.0, 0.0);
    let (b1, _, bj1) = ear_profile(1.0, 0.026);

    let mut fill = ear_lathe(&p0, j0);
    let base_fill = ear_lathe(&b0, bj0);
    for (v, bv) in fill.vertices.iter_mut().zip(base_fill.vertices.iter()) {
        v.rest = bv.pos;
    }
    fill.compute_vertex_normals();
    fill.compute_bounds();

    let mut rim = ear_lathe(&p1, j1);
    let base_rim = ear_lathe(&b1, bj1);
    for (v, bv) in rim.vertices.iter_mut().zip(base_rim.vertices.iter()) {
        v.rest = bv.pos;
    }
    rim.compute_vertex_normals();
    rim.compute_bounds();

    EarPairGeo {
        key: (k * 100.0).round() as u32,
        fill,
        rim,
        meet,
    }
}

fn turn_ear_state(ear: &mut EarState, z: f32, x: f32) {
    ear.rot_x = x;
    ear.rot_z = z;
    let pivot = ear.meet
        * smooth(0.0, 0.1, ear.tilt + ear.side * z);
    let pivot = pivot.mul_elem(Vec3::new(1.0, EAR_STRETCH, 1.0));
    let held = pivot.apply_euler_xyz(-0.12, 0.0, -ear.side * ear.tilt);
    let turned = pivot.apply_euler_xyz(x, 0.0, z);
    ear.pos = Vec3::new(ear.side * EAR_AT.x, EAR_AT.y, EAR_AT.z) + held - turned;
}

fn tail_rest_pts() -> [Vec3; 6] {
    [
        Vec3::new(0.42, 0.42, -0.25),
        Vec3::new(0.72, 0.28, -0.3),
        Vec3::new(0.96, 0.36, -0.32),
        Vec3::new(1.05, 0.56, -0.3),
        Vec3::new(1.12, 0.7, -0.28),
        Vec3::new(1.3, 0.7, -0.26),
    ]
}

fn tail_sleep_pts() -> [Vec3; 6] {
    [
        Vec3::new(0.42, 0.12, -0.25),
        Vec3::new(0.85, 0.1, -0.15),
        Vec3::new(1.05, 0.1, 0.1),
        Vec3::new(1.0, 0.1, 0.35),
        Vec3::new(0.82, 0.1, 0.5),
        Vec3::new(0.6, 0.1, 0.55),
    ]
}

fn tail_thump_pts() -> [Vec3; 6] {
    [
        Vec3::new(0.42, 0.36, -0.25),
        Vec3::new(0.7, 0.16, -0.24),
        Vec3::new(0.94, 0.07, -0.14),
        Vec3::new(1.12, 0.05, -0.02),
        Vec3::new(1.28, 0.05, 0.06),
        Vec3::new(1.42, 0.06, 0.1),
    ]
}
