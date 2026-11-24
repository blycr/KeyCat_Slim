//! Platform-independent C ABI wrapper over `keycat-core` for native GPU frontends
//! (macOS Swift + Metal, or any C/Swift/ObjC host).

use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use keycat_core::audio::PurrSynth;
use keycat_core::behavior::{HitZone, Scene, SpriteKey};
use keycat_core::cat::{COLLAR_LIFT, HEAD_R};
use keycat_core::coats::Look;
use keycat_core::geometry::{MeshData, Vertex};
use keycat_core::keyboard::KeyInstance;
use keycat_core::life::Festival;
use keycat_core::math::{hex_color, Mat4, Vec3};
use keycat_core::tuning::{ParamKind, Tuning, TUNING_PRESETS, TUNING_SCHEMA};
use serde_json::Value;

pub const KC_HIT_NONE: u32 = 0;
pub const KC_HIT_PET: u32 = 1;
pub const KC_HIT_DRAG: u32 = 2;

pub const KC_PIPE_BASIC: u32 = 0;
pub const KC_PIPE_FUR: u32 = 1;
pub const KC_PIPE_INSTANCED: u32 = 2;

pub const KC_CULL_BACK: u32 = 0;
pub const KC_CULL_FRONT: u32 = 1;
pub const KC_CULL_NONE: u32 = 2;
pub const KC_CULL_DECAL: u32 = 3;

pub const KC_DEPTH_RW: u32 = 0;
pub const KC_DEPTH_RO: u32 = 1;
pub const KC_DEPTH_NONE: u32 = 2;

pub const KC_BLEND_OPAQUE: u32 = 0;
pub const KC_BLEND_PREMUL: u32 = 1;

pub const KC_MESH_BODY: u32 = 0;
pub const KC_MESH_HEAD: u32 = 1;
pub const KC_MESH_PAW: u32 = 2;
pub const KC_MESH_TOE: u32 = 3;
pub const KC_MESH_DISC: u32 = 4;
pub const KC_MESH_CAP_START: u32 = 5;
pub const KC_MESH_CAP_END: u32 = 6;
pub const KC_MESH_ARC: u32 = 7;
pub const KC_MESH_NOSE: u32 = 8;
pub const KC_MESH_MOUTH: u32 = 9;
pub const KC_MESH_COLLAR_FILL: u32 = 10;
pub const KC_MESH_COLLAR_RIM: u32 = 11;
pub const KC_MESH_BELL_BALL: u32 = 12;
pub const KC_MESH_BELL_RIM: u32 = 13;
pub const KC_MESH_BELL_SLIT: u32 = 14;
pub const KC_MESH_BELL_GLINT: u32 = 15;
pub const KC_MESH_BELL_LOOP: u32 = 16;
pub const KC_MESH_KB_CASE: u32 = 17;
pub const KC_MESH_KB_RIM: u32 = 18;
pub const KC_MESH_KB_PLATE: u32 = 19;
pub const KC_MESH_KB_SIDE: u32 = 20;
pub const KC_MESH_KB_TOP: u32 = 21;
pub const KC_MESH_KB_DOT: u32 = 22;
pub const KC_MESH_MOUSE_DOME: u32 = 23;
pub const KC_MESH_MOUSE_WHEEL: u32 = 24;
pub const KC_MESH_MOUSE_SEAM: u32 = 25;
pub const KC_MESH_MOUSE_BTN: u32 = 26;
pub const KC_MESH_WHISKER_0: u32 = 27;
pub const KC_MESH_CUP_BODY: u32 = 33;
pub const KC_MESH_CUP_RIM: u32 = 34;
pub const KC_MESH_CUP_HANDLE: u32 = 35;
pub const KC_MESH_CUP_COFFEE: u32 = 36;
pub const KC_MESH_HAT_CONE: u32 = 37;
pub const KC_MESH_HAT_BRIM: u32 = 38;
pub const KC_MESH_HAT_RING: u32 = 39;
pub const KC_MESH_EAR_FILL: u32 = 40;
pub const KC_MESH_EAR_RIM: u32 = 41;

pub const KC_DYN_CABLE: u32 = 0;
pub const KC_DYN_TAIL_FILL: u32 = 1;
pub const KC_DYN_TAIL_RIM: u32 = 2;
pub const KC_DYN_ARM_FILL_0: u32 = 3;
pub const KC_DYN_ARM_RIM_0: u32 = 4;
pub const KC_DYN_ARM_FILL_1: u32 = 5;
pub const KC_DYN_ARM_RIM_1: u32 = 6;
pub const KC_DYN_STEAM_A: u32 = 7;
pub const KC_DYN_STEAM_B: u32 = 8;

pub const KC_SPRITE_CAP: u32 = 0;
pub const KC_SPRITE_SLEEP_Z: u32 = 1;
pub const KC_SPRITE_ANGER: u32 = 2;
pub const KC_SPRITE_BANG: u32 = 3;
pub const KC_SPRITE_DUST: u32 = 4;
pub const KC_SPRITE_HEART: u32 = 5;
pub const KC_SPRITE_BUBBLE: u32 = 6;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KcMeshView {
    pub vertices: *const Vertex,
    pub vertex_count: u32,
    pub indices: *const u32,
    pub index_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct KcFrameCB {
    pub view: [f32; 16],
    pub proj: [f32; 16],
    pub bend: [f32; 4],
    pub cam_right: [f32; 4],
    pub cam_up: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct KcFurCB {
    pub fur: [f32; 4],
    pub light: [f32; 4],
    pub dark: [f32; 4],
    pub patch_a: [f32; 4],
    pub patch_b: [f32; 4],
    pub point: [f32; 4],
    pub inner: [f32; 4],
    pub w0: [f32; 4],
    pub w1: [f32; 4],
    pub w2: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct KcObjCB {
    pub model: [f32; 16],
    pub normal_mat: [f32; 16],
    pub color: [f32; 4],
    pub params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KcDrawCmd {
    pub pipeline: u32,
    pub cull: u32,
    pub depth: u32,
    pub blend: u32,
    pub is_dynamic: u32,
    pub mesh_id: u32,
    pub inst_slot: u32,
    pub instance_count: u32,
    pub obj: KcObjCB,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KcSpriteCmd {
    pub kind: u32,
    pub label: *const c_char,
    pub pos: [f32; 3],
    pub scale_x: f32,
    pub scale_y: f32,
    pub opacity: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KcFrameData {
    pub frame_cb: KcFrameCB,
    pub fur_cb: KcFurCB,
    pub ear_key: u32,
    pub kb_dirty: u8,
    pub cable_dirty: u8,
    pub cat_dyn_dirty: u8,
    pub steam_dirty: u8,
    pub is_busy: u8,
    pub opacity: f32,
    pub draws: *const KcDrawCmd,
    pub draw_count: u32,
    pub sprites: *const KcSpriteCmd,
    pub sprite_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KcTuningMeta {
    pub group: *const c_char,
    pub key: *const c_char,
    pub label: *const c_char,
    pub hint: *const c_char,
    pub unit: *const c_char,
    pub is_toggle: u8,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub def_val: f32,
}

pub struct KcScene {
    scene: Scene,
    draws: Vec<KcDrawCmd>,
    sprite_labels: Vec<CString>,
    sprites: Vec<KcSpriteCmd>,
}

pub struct KcAudio {
    synth: Mutex<PurrSynth>,
}

fn mat_flat(m: Mat4) -> [f32; 16] {
    let c = m.to_cols_array_2d();
    [
        c[0][0], c[0][1], c[0][2], c[0][3],
        c[1][0], c[1][1], c[1][2], c[1][3],
        c[2][0], c[2][1], c[2][2], c[2][3],
        c[3][0], c[3][1], c[3][2], c[3][3],
    ]
}

fn hit_to_u32(h: HitZone) -> u32 {
    match h {
        HitZone::None => KC_HIT_NONE,
        HitZone::Pet => KC_HIT_PET,
        HitZone::Drag => KC_HIT_DRAG,
    }
}

fn mesh_view(m: &MeshData) -> KcMeshView {
    KcMeshView {
        vertices: m.vertices.as_ptr(),
        vertex_count: m.vertices.len() as u32,
        indices: m.indices.as_ptr(),
        index_count: m.indices.len() as u32,
    }
}

fn empty_mesh_view() -> KcMeshView {
    KcMeshView {
        vertices: ptr::null(),
        vertex_count: 0,
        indices: ptr::null(),
        index_count: 0,
    }
}

unsafe fn cstr_or<'a>(p: *const c_char, fallback: &'a str) -> &'a str {
    if p.is_null() {
        return fallback;
    }
    CStr::from_ptr(p).to_str().unwrap_or(fallback)
}

impl KcScene {
    #[allow(clippy::too_many_arguments)]
    fn push_draw(
        &mut self,
        pipeline: u32,
        cull: u32,
        depth: u32,
        blend: u32,
        is_dynamic: bool,
        mesh_id: u32,
        model: Mat4,
        view: Mat4,
        color: [f32; 3],
        alpha: f32,
        params: [f32; 4],
    ) {
        let nrm = view.mul(model).normal_matrix();
        self.draws.push(KcDrawCmd {
            pipeline,
            cull,
            depth,
            blend,
            is_dynamic: u32::from(is_dynamic),
            mesh_id,
            inst_slot: 0,
            instance_count: 0,
            obj: KcObjCB {
                model: mat_flat(model),
                normal_mat: mat_flat(nrm),
                color: [color[0], color[1], color[2], alpha],
                params,
            },
        });
    }

    fn build_frame(&mut self) -> KcFrameData {
        self.draws.clear();
        self.sprite_labels.clear();
        self.sprites.clear();

        let kb_dirty = u8::from(self.scene.keyboard.instances_dirty);
        let cable_dirty = u8::from(self.scene.mouse.cable_dirty);
        let cat_dyn_dirty = u8::from(self.scene.cat.tail_dirty)
            | (u8::from(self.scene.cat.arms_dirty[0]) << 1)
            | (u8::from(self.scene.cat.arms_dirty[1]) << 2);
        let steam_dirty = u8::from(self.scene.cup.steam_dirty);
        self.scene.keyboard.instances_dirty = false;
        self.scene.mouse.cable_dirty = false;
        self.scene.cat.tail_dirty = false;
        self.scene.cat.arms_dirty = [false, false];
        self.scene.cat.dynamic_dirty = false;
        self.scene.cup.steam_dirty = false;

        let ear_key = self.scene.cat.active_ear().key;
        let frame_cb = KcFrameCB {
            view: mat_flat(self.scene.view),
            proj: mat_flat(self.scene.proj),
            bend: [self.scene.cat.bend.x, self.scene.cat.bend.squash, self.scene.cat.bend.z, 0.0],
            cam_right: [self.scene.cam_right.x, self.scene.cam_right.y, self.scene.cam_right.z, 0.0],
            cam_up: [self.scene.cam_up.x, self.scene.cam_up.y, self.scene.cam_up.z, 0.0],
        };

        let rl = self.scene.cat.resolved;
        let fur_cb = KcFurCB {
            fur: [rl.fur[0], rl.fur[1], rl.fur[2], 1.0],
            light: [rl.light[0], rl.light[1], rl.light[2], 1.0],
            dark: [rl.dark[0], rl.dark[1], rl.dark[2], 1.0],
            patch_a: [rl.patch_a[0], rl.patch_a[1], rl.patch_a[2], 1.0],
            patch_b: [rl.patch_b[0], rl.patch_b[1], rl.patch_b[2], 1.0],
            point: [rl.point[0], rl.point[1], rl.point[2], 1.0],
            inner: [rl.inner[0], rl.inner[1], rl.inner[2], 1.0],
            w0: [rl.stripes, rl.muzzle, rl.bib, rl.calico],
            w1: [rl.tuxedo, rl.points, rl.curl, rl.inner_amt],
            w2: [rl.shade, 0.0, 0.0, 0.0],
        };

        let view = self.scene.view;
        let body_w = self.scene.cat.body_world();
        let head_w = self.scene.cat.head_world();
        let board_w = self.scene.keyboard.world_matrix();
        let paw_rim_col = hex_color("#8f8f8a");

        // ===== Order -2: desk cup, mouse outer shell + hind paw rims =====
        if self.scene.cup.visible() {
            let cw = self.scene.cup.world_matrix();
            let hw = self.scene.cup.handle_world();
            let cfw = self.scene.cup.coffee_world();
            let alpha = self.scene.cup.opacity;
            let blend = if alpha < 0.99 { KC_BLEND_PREMUL } else { KC_BLEND_OPAQUE };
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, blend, false, KC_MESH_CUP_RIM, cw, view, hex_color("#2a241f"), alpha, [0.0; 4]);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, blend, false, KC_MESH_CUP_BODY, cw, view, hex_color("#f7f2e7"), alpha, [0.0; 4]);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, blend, false, KC_MESH_CUP_HANDLE, hw, view, hex_color("#efe7d7"), alpha, [0.0; 4]);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, blend, false, KC_MESH_CUP_COFFEE, cfw, view, hex_color("#5b3a24"), alpha, [0.0; 4]);
            if self.scene.cup.steam_opacity_a > 0.01 {
                self.push_draw(KC_PIPE_BASIC, KC_CULL_NONE, KC_DEPTH_RO, KC_BLEND_PREMUL, true, KC_DYN_STEAM_A, Mat4::IDENTITY, view, hex_color("#fbfaf3"), self.scene.cup.steam_opacity_a, [0.0; 4]);
            }
            if self.scene.cup.steam_opacity_b > 0.01 {
                self.push_draw(KC_PIPE_BASIC, KC_CULL_NONE, KC_DEPTH_RO, KC_BLEND_PREMUL, true, KC_DYN_STEAM_B, Mat4::IDENTITY, view, hex_color("#fbfaf3"), self.scene.cup.steam_opacity_b, [0.0; 4]);
            }
        }

        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_MOUSE_DOME,
            self.scene.mouse.shell_world(0.03), view, hex_color("#f4f1e6"), 1.0, [0.0; 4],
        );
        for &s in &[-1.0f32, 1.0] {
            let m = self.scene.cat.hind_paw_world(s, true);
            self.push_draw(
                KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE,
                false, KC_MESH_PAW, m, view, paw_rim_col, 1.0, [0.0; 4],
            );
        }

        // ===== Order -1: mouse inner shell + body/head/ear/tail rims =====
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_MOUSE_DOME,
            self.scene.mouse.shell_world(0.013), view, hex_color("#1d1d1b"), 1.0, [0.0; 4],
        );
        let body_rim_m = body_w.mul(Mat4::from_scale(Vec3::new(1.045, 1.018, 0.86)));
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE,
            false, KC_MESH_BODY, body_rim_m, view, rl.rim_color, 1.0, [2.0, 1.0 / 1.045, 1.0 / 0.86, 0.0],
        );
        let head_rim_m = head_w.mul(Mat4::from_scale(Vec3::new(HEAD_R.x + 0.024, HEAD_R.y + 0.024, HEAD_R.z + 0.02)));
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE,
            false, KC_MESH_HEAD, head_rim_m, view, rl.rim_color, 1.0, [0.0; 4],
        );
        for i in 0..2 {
            let ear_rim_m = self.scene.cat.ear_world(i).mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.56)));
            self.push_draw(
                KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE,
                false, KC_MESH_EAR_RIM, ear_rim_m, view, rl.rim_color, 1.0, [0.0; 4],
            );
        }
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE,
            true, KC_DYN_TAIL_RIM, Mat4::IDENTITY, view, rl.rim_color, 1.0, [0.0; 4],
        );
        let tail_cap_rim_m = Mat4::from_trs_euler(
            self.scene.cat.tail.end_pos,
            Vec3::ZERO,
            Vec3::splat(self.scene.cat.tail.end_scale_rim),
        );
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE,
            false, KC_MESH_CAP_END, tail_cap_rim_m, view, rl.rim_color, 1.0, [0.0; 4],
        );

        // ===== Order 0: keyboard case/plate/keys, mouse dome, cat fur, chinLine =====
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_KB_RIM, board_w, view, hex_color("#f4f1e6"), 1.0, [0.0; 4],
        );
        if rl.chin_visible {
            let chin_m = head_w.mul(Mat4::from_scale(HEAD_R));
            self.push_draw(
                KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE,
                false, KC_MESH_HEAD, chin_m, view, rl.chin_color, 1.0, [3.0, 0.05, 0.0, 0.0],
            );
        }
        let kb_case_col = self.scene.keyboard.case_color;
        let kb_plate_col = self.scene.keyboard.plate_color;
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_KB_CASE, board_w, view, kb_case_col, 1.0, [0.0; 4],
        );
        self.push_draw(
            KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_KB_PLATE, board_w, view, kb_plate_col, 1.0, [0.0; 4],
        );
        let mouse_dome_w = self.scene.mouse.dome_world();
        if rl.style_3d {
            self.push_draw(
                KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
                false, KC_MESH_MOUSE_DOME, mouse_dome_w, view, [1.0, 1.0, 1.0], 1.0, [1.0, 0.0, 0.0, 0.0],
            );
        } else {
            self.push_draw(
                KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
                false, KC_MESH_MOUSE_DOME, mouse_dome_w, view, hex_color("#f8f7f0"), 1.0, [0.0; 4],
            );
        }

        // Instanced keyboard (3 draw calls for all 61 keys)
        let key_count = self.scene.keyboard.keys.len() as u32;
        let kb_obj = KcObjCB {
            model: mat_flat(board_w),
            normal_mat: mat_flat(Mat4::IDENTITY),
            color: [1.0; 4],
            params: [0.0; 4],
        };
        for &(mesh_id, slot) in &[
            (KC_MESH_KB_SIDE, 0u32),
            (KC_MESH_KB_TOP, 1u32),
            (KC_MESH_KB_DOT, 2u32),
        ] {
            self.draws.push(KcDrawCmd {
                pipeline: KC_PIPE_INSTANCED,
                cull: KC_CULL_BACK,
                depth: KC_DEPTH_RW,
                blend: KC_BLEND_OPAQUE,
                is_dynamic: 0,
                mesh_id,
                inst_slot: slot,
                instance_count: key_count,
                obj: kb_obj,
            });
        }

        // Fur meshes (body, head, ears, tail)
        let body_m = body_w.mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.82)));
        self.push_draw(
            KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_BODY, body_m, view, [1.0; 3], 1.0, [0.0, 1.0, 1.0 / 0.82, 1.0],
        );
        let head_m = head_w.mul(Mat4::from_scale(HEAD_R));
        self.push_draw(
            KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_HEAD, head_m, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 0.0],
        );
        for i in 0..2 {
            let ear_m = self.scene.cat.ear_world(i).mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.5)));
            let part = if i == 0 { 2.0 } else { 3.0 };
            self.push_draw(
                KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
                false, KC_MESH_EAR_FILL, ear_m, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, part],
            );
        }
        self.push_draw(
            KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            true, KC_DYN_TAIL_FILL, Mat4::IDENTITY, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 4.0],
        );
        let tail_cap_m = Mat4::from_trs_euler(
            self.scene.cat.tail.end_pos,
            Vec3::ZERO,
            Vec3::splat(self.scene.cat.tail.end_scale_fill),
        );
        self.push_draw(
            KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
            false, KC_MESH_CAP_END, tail_cap_m, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 4.0],
        );

        // ===== Order 0~1: collar / CNY scarf + mouse cable =====
        let outfit = self.scene.cat.outfit;
        let wears_scarf = outfit == Festival::Cny;
        if wears_scarf {
            let cw = self.scene.cat.collar_world().mul(Mat4::from_scale(Vec3::new(1.05, 1.45, 1.05)));
            let ink = hex_color("#7a1d17");
            let scarf_red = hex_color("#d93829");
            let lift = [0.0, 0.0, COLLAR_LIFT, 0.0];
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_COLLAR_RIM, cw, view, ink, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_COLLAR_FILL, cw, view, scarf_red, 1.0, lift);
        } else if rl.collar_visible {
            let cw = self.scene.cat.collar_world();
            let bh = self.scene.cat.bell_hang_world();
            let r_bell: f32 = 0.064;
            let ink = hex_color("#1d1d1b");
            let gold = hex_color("#e8b93a");
            let lift = [0.0, 0.0, COLLAR_LIFT, 0.0];

            let cs_r = cw.mul(Mat4::from_trs_euler(self.scene.cat.collar_band.start_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.collar_band.start_scale_rim)));
            let ce_r = cw.mul(Mat4::from_trs_euler(self.scene.cat.collar_band.end_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.collar_band.end_scale_rim)));
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_COLLAR_RIM, cw, view, ink, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_START, cs_r, view, ink, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_END, ce_r, view, ink, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_RIM, bh, view, ink, 1.0, lift);

            let cs_f = cw.mul(Mat4::from_trs_euler(self.scene.cat.collar_band.start_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.collar_band.start_scale_fill)));
            let ce_f = cw.mul(Mat4::from_trs_euler(self.scene.cat.collar_band.end_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.collar_band.end_scale_fill)));
            let loop_m = bh.mul(Mat4::from_translation(Vec3::new(0.0, r_bell + 0.01, 0.0)));
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_COLLAR_FILL, cw, view, rl.collar_color, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_START, cs_f, view, rl.collar_color, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_END, ce_f, view, rl.collar_color, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_BALL, bh, view, gold, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_LOOP, loop_m, view, ink, 1.0, lift);

            let slit_z = (r_bell * r_bell - 0.022 * 0.022).sqrt() + 0.002;
            let slit_m = bh.mul(Mat4::from_trs_euler(Vec3::new(0.0, -0.022, slit_z), Vec3::new(-0.33, 0.0, 0.0), Vec3::ONE));
            let glint_z = (r_bell * r_bell - 0.022 * 0.022 - 0.024 * 0.024).sqrt() + 0.002;
            let glint_m = bh.mul(Mat4::from_trs_euler(Vec3::new(-0.022, 0.024, glint_z), Vec3::new(-0.35, -0.35, 0.0), Vec3::ONE));
            self.push_draw(KC_PIPE_BASIC, KC_CULL_DECAL, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_SLIT, slit_m, view, ink, 1.0, lift);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_DECAL, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_GLINT, glint_m, view, hex_color("#fff6d8"), 1.0, lift);
        }

        if self.scene.mouse.cable_visible {
            self.push_draw(
                KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE,
                true, KC_DYN_CABLE, Mat4::IDENTITY, view, hex_color("#2a2a27"), 1.0, [0.0; 4],
            );
        }

        // ===== Order 2~3: hind paws + toes =====
        for &s in &[-1.0f32, 1.0] {
            let pm = self.scene.cat.hind_paw_world(s, false);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_PAW, pm, view, rl.paw_color, 1.0, [0.0; 4]);
            for &tx in &[-0.3f32, 0.3] {
                let tm = pm.mul(Mat4::from_translation(Vec3::new(tx, 0.66, 0.72)));
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_TOE, tm, view, rl.toe_color, 1.0, [0.0; 4]);
            }
        }

        // ===== Order 5~7: mouse overlays =====
        if self.scene.mouse.pressed > 0.005 {
            let bm = self.scene.mouse.button_world();
            let alpha = 0.9 * self.scene.mouse.pressed;
            self.push_draw(KC_PIPE_BASIC, KC_CULL_NONE, KC_DEPTH_NONE, KC_BLEND_PREMUL, false, KC_MESH_MOUSE_BTN, bm, view, hex_color("#b9d84f"), alpha, [0.0; 4]);
        }
        let shape_w = self.scene.mouse.shape_world();
        let wheel_w = self.scene.mouse.wheel_world();
        self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_MOUSE_SEAM, shape_w, view, hex_color("#2a2a27"), 1.0, [0.0; 4]);
        self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_MOUSE_WHEEL, wheel_w, view, hex_color("#e9a94f"), 1.0, [0.0; 4]);

        // ===== Order 10~15: face overlays, yawn, lick, festival hats =====
        let pupil_col = hex_color("#141414");
        let shine_col = [1.0, 1.0, 1.0];
        let blush_col = hex_color("#f08a8a");
        let dots = self.scene.cat.shine_dots();
        let blush_opacity = self.scene.cat.blush_opacity;
        for i in 0..2 {
            let ew = self.scene.cat.eye_world(i);
            let eye = self.scene.cat.eyes[i];
            if !eye.closed {
                let iris_m = ew.mul(Mat4::from_scale(Vec3::new(0.155, 0.14, 1.0)));
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_DISC, iris_m, view, rl.eye_colors[i], 1.0, [0.0; 4]);

                let pupil_m = ew.mul(Mat4::from_trs_euler(
                    Vec3::new(eye.pupil_pos.x, eye.pupil_pos.y, 0.002),
                    Vec3::ZERO,
                    Vec3::new(eye.pupil_scale.x, eye.pupil_scale.y, 1.0),
                ));
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_DISC, pupil_m, view, pupil_col, 1.0, [0.0; 4]);

                for d in dots {
                    let sm = ew.mul(Mat4::from_trs_euler(
                        Vec3::new(eye.side * d[0], d[1], 0.004),
                        Vec3::ZERO,
                        Vec3::new(d[2], d[2], 1.0),
                    ));
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_DISC, sm, view, shine_col, 1.0, [0.0; 4]);
                }
            } else {
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_ARC, ew, view, rl.lid_color, 1.0, [0.0; 4]);
            }

            if blush_opacity > 0.005 {
                let bm = ew.mul(Mat4::from_trs_euler(
                    Vec3::new(eye.side * 0.06, -0.15, 0.001),
                    Vec3::ZERO,
                    Vec3::new(0.085, 0.048, 1.0),
                ));
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_PREMUL, false, KC_MESH_DISC, bm, view, blush_col, blush_opacity, [0.0; 4]);
            }
        }
        if rl.whiskers_visible {
            let n = self.scene.cat.whiskers.len().min(6) as u32;
            for i in 0..n {
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_WHISKER_0 + i, head_w, view, rl.whisker_color, 1.0, [0.0; 4]);
            }
        }
        if rl.face_visible {
            let nose_w = self.scene.cat.nose_world();
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_NOSE, nose_w, view, rl.nose_color, 1.0, [0.0; 4]);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_MOUTH, head_w, view, rl.mouth_color, 1.0, [0.0; 4]);
        }
        if self.scene.cat.yawn_open > 0.03 {
            let yw = self.scene.cat.yawn_world();
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_DISC, yw, view, hex_color("#4a1b24"), 1.0, [0.0; 4]);
            let tongue = yw.mul(Mat4::from_trs_euler(Vec3::new(0.0, -0.32, 0.002), Vec3::ZERO, Vec3::new(0.65, 0.48, 1.0)));
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_DISC, tongue, view, hex_color("#f08a9a"), 1.0, [0.0; 4]);
        }
        if self.scene.cat.lick_out > 0.03 {
            let lw = self.scene.cat.lick_world();
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_DISC, lw, view, hex_color("#f393a3"), 1.0, [0.0; 4]);
        }

        // Festival hats
        if matches!(outfit, Festival::Xmas | Festival::Halloween | Festival::Birthday) {
            let hr = self.scene.cat.hat_root_world();
            match outfit {
                Festival::Xmas => {
                    let cone_m = hr.mul(Mat4::from_scale(Vec3::new(0.28, 0.56, 0.28)));
                    let ring_m = hr.mul(Mat4::from_scale(Vec3::new(0.27, 0.27, 0.27)));
                    let pom_m = hr.mul(Mat4::from_trs_euler(Vec3::new(0.0, 0.56, 0.0), Vec3::ZERO, Vec3::splat(1.3)));
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_CONE, cone_m, view, hex_color("#d9483b"), 1.0, [0.0; 4]);
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_RING, ring_m, view, hex_color("#fbf9f3"), 1.0, [0.0; 4]);
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_BALL, pom_m, view, hex_color("#fbf9f3"), 1.0, [0.0; 4]);
                }
                Festival::Halloween => {
                    let brim_m = hr.mul(Mat4::from_scale(Vec3::new(0.45, 0.028, 0.45)));
                    let cone_m = hr.mul(Mat4::from_scale(Vec3::new(0.22, 0.60, 0.22)));
                    let band_m = hr.mul(Mat4::from_trs_euler(Vec3::new(0.0, 0.05, 0.0), Vec3::ZERO, Vec3::new(0.21, 0.18, 0.21)));
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_BRIM, brim_m, view, hex_color("#2b2440"), 1.0, [0.0; 4]);
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_CONE, cone_m, view, hex_color("#2b2440"), 1.0, [0.0; 4]);
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_RING, band_m, view, hex_color("#9a6bd1"), 1.0, [0.0; 4]);
                }
                Festival::Birthday => {
                    let cone_m = hr.mul(Mat4::from_scale(Vec3::new(0.21, 0.54, 0.21)));
                    let ring_m = hr.mul(Mat4::from_scale(Vec3::new(0.21, 0.16, 0.21)));
                    let pom_m = hr.mul(Mat4::from_trs_euler(Vec3::new(0.0, 0.54, 0.0), Vec3::ZERO, Vec3::splat(1.15)));
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_CONE, cone_m, view, hex_color("#ee8fb4"), 1.0, [0.0; 4]);
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_HAT_RING, ring_m, view, hex_color("#70c4b8"), 1.0, [0.0; 4]);
                    self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_BELL_BALL, pom_m, view, hex_color("#f2c94c"), 1.0, [0.0; 4]);
                }
                _ => {}
            }
        }

        // ===== Order 18~23: front paw rims, arm rims/bodies, front paws/toes =====
        for i in 0..2 {
            let pm = self.scene.cat.arm_paw_world(i, true);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RO, KC_BLEND_OPAQUE, false, KC_MESH_PAW, pm, view, paw_rim_col, 1.0, [0.0; 4]);
        }
        for i in 0..2 {
            let dyn_rim = if i == 0 { KC_DYN_ARM_RIM_0 } else { KC_DYN_ARM_RIM_1 };
            let ms = Mat4::from_trs_euler(self.scene.cat.arms[i].limb.start_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.arms[i].limb.start_scale_rim));
            let me = Mat4::from_trs_euler(self.scene.cat.arms[i].limb.end_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.arms[i].limb.end_scale_rim));
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, true, dyn_rim, Mat4::IDENTITY, view, rl.arm_rim_color, 1.0, [4.0, 0.0, 0.0, 0.0]);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_START, ms, view, rl.arm_rim_color, 1.0, [4.0, 0.0, 0.0, 0.0]);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_FRONT, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_END, me, view, rl.arm_rim_color, 1.0, [4.0, 0.0, 0.0, 0.0]);
        }
        for i in 0..2 {
            let dyn_fill = if i == 0 { KC_DYN_ARM_FILL_0 } else { KC_DYN_ARM_FILL_1 };
            let ms = Mat4::from_trs_euler(self.scene.cat.arms[i].limb.start_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.arms[i].limb.start_scale_fill));
            let me = Mat4::from_trs_euler(self.scene.cat.arms[i].limb.end_pos, Vec3::ZERO, Vec3::splat(self.scene.cat.arms[i].limb.end_scale_fill));
            self.push_draw(KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, true, dyn_fill, Mat4::IDENTITY, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 5.0]);
            self.push_draw(KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_START, ms, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 5.0]);
            self.push_draw(KC_PIPE_FUR, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_CAP_END, me, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 5.0]);
        }
        for i in 0..2 {
            let pm = self.scene.cat.arm_paw_world(i, false);
            self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_RW, KC_BLEND_OPAQUE, false, KC_MESH_PAW, pm, view, rl.paw_color, 1.0, [0.0; 4]);
            for &tx in &[-0.3f32, 0.3] {
                let tm = pm.mul(Mat4::from_translation(Vec3::new(tx, 0.66, 0.72)));
                self.push_draw(KC_PIPE_BASIC, KC_CULL_BACK, KC_DEPTH_NONE, KC_BLEND_OPAQUE, false, KC_MESH_TOE, tm, view, rl.toe_color, 1.0, [0.0; 4]);
            }
        }

        // ===== Order 40~48: 2D Billboard Sprites =====
        for sp in &self.scene.sprites {
            let label = match &sp.key {
                SpriteKey::Cap(s) | SpriteKey::Bubble(s) => s.as_str(),
                _ => "",
            };
            self.sprite_labels.push(CString::new(label).unwrap_or_default());
        }
        for (idx, sp) in self.scene.sprites.iter().enumerate() {
            let kind = match &sp.key {
                SpriteKey::Cap(_) => KC_SPRITE_CAP,
                SpriteKey::SleepZ => KC_SPRITE_SLEEP_Z,
                SpriteKey::Anger => KC_SPRITE_ANGER,
                SpriteKey::Bang => KC_SPRITE_BANG,
                SpriteKey::Dust => KC_SPRITE_DUST,
                SpriteKey::Heart => KC_SPRITE_HEART,
                SpriteKey::Bubble(_) => KC_SPRITE_BUBBLE,
            };
            self.sprites.push(KcSpriteCmd {
                kind,
                label: self.sprite_labels[idx].as_ptr(),
                pos: sp.pos.to_array(),
                scale_x: sp.scale_x,
                scale_y: sp.scale_y,
                opacity: sp.opacity,
            });
        }

        KcFrameData {
            frame_cb,
            fur_cb,
            ear_key,
            kb_dirty,
            cable_dirty,
            cat_dyn_dirty,
            steam_dirty,
            is_busy: u8::from(self.scene.is_busy()),
            opacity: self.scene.opacity,
            draws: self.draws.as_ptr(),
            draw_count: self.draws.len() as u32,
            sprites: self.sprites.as_ptr(),
            sprite_count: self.sprites.len() as u32,
        }
    }
}

#[no_mangle]
pub extern "C" fn kc_scene_new() -> *mut KcScene {
    Box::into_raw(Box::new(KcScene {
        scene: Scene::new(),
        draws: Vec::with_capacity(112),
        sprite_labels: Vec::with_capacity(16),
        sprites: Vec::with_capacity(16),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_free(s: *mut KcScene) {
    if !s.is_null() {
        drop(Box::from_raw(s));
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_viewport(s: *mut KcScene, w: f32, h: f32) {
    if let Some(s) = s.as_mut() {
        s.scene.set_viewport(w, h);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_step(s: *mut KcScene, dt: f32) {
    if let Some(s) = s.as_mut() {
        s.scene.step(dt);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_is_busy(s: *const KcScene) -> u8 {
    s.as_ref().map(|s| u8::from(s.scene.is_busy())).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_on_key(s: *mut KcScene, code: *const c_char, down: u8) {
    if let Some(s) = s.as_mut() {
        let code_str = cstr_or(code, "");
        if !code_str.is_empty() {
            s.scene.on_key(code_str, down != 0);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_on_cursor(s: *mut KcScene, x: f32, y: f32) -> u32 {
    if let Some(s) = s.as_mut() {
        s.scene.on_cursor(x, y);
        let hit = s.scene.hit_test(x, y);
        s.scene.hit = hit;
        return hit_to_u32(hit);
    }
    KC_HIT_NONE
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_on_mouse_move(s: *mut KcScene, dx: f32, dy: f32) {
    if let Some(s) = s.as_mut() {
        s.scene.on_mouse_move(dx, dy);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_on_mouse_button(s: *mut KcScene, down: u8) {
    if let Some(s) = s.as_mut() {
        s.scene.on_mouse_button(down != 0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_on_pet(s: *mut KcScene, down: u8, x: f32) {
    if let Some(s) = s.as_mut() {
        s.scene.on_pet(down != 0, x);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_on_double_click(s: *mut KcScene) {
    if let Some(s) = s.as_mut() {
        s.scene.on_head_double_click();
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_hit(s: *const KcScene) -> u32 {
    s.as_ref().map(|s| hit_to_u32(s.scene.hit)).unwrap_or(KC_HIT_NONE)
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_look(
    s: *mut KcScene,
    style_3d: u8,
    coat: *const c_char,
    pattern: *const c_char,
    eye: *const c_char,
    shine: *const c_char,
    collar: *const c_char,
    blush: u8,
    whiskers: u8,
) {
    if let Some(s) = s.as_mut() {
        let mut look = s.scene.cat.look.clone();
        look.style_3d = style_3d != 0;
        look.coat = cstr_or(coat, "black").to_string();
        look.pattern = cstr_or(pattern, "auto").to_string();
        look.eye = cstr_or(eye, "auto").to_string();
        look.shine = cstr_or(shine, "classic").to_string();
        look.collar = cstr_or(collar, "none").to_string();
        look.blush = blush != 0;
        look.whiskers = whiskers != 0;
        s.scene.set_look(look);
    }
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn kc_scene_set_look_ext(
    s: *mut KcScene,
    style_3d: u8,
    coat: *const c_char,
    pattern: *const c_char,
    eye: *const c_char,
    shine: *const c_char,
    collar: *const c_char,
    blush: u8,
    whiskers: u8,
    gloves: *const c_char,
    keyboard_color: *const c_char,
    mouse_cable: u8,
    opacity: f32,
    cup: u8,
    festival: u8,
    sound: u8,
) {
    if let Some(s) = s.as_mut() {
        s.scene.set_look(Look {
            style_3d: style_3d != 0,
            coat: cstr_or(coat, "black").to_string(),
            pattern: cstr_or(pattern, "auto").to_string(),
            eye: cstr_or(eye, "auto").to_string(),
            shine: cstr_or(shine, "classic").to_string(),
            collar: cstr_or(collar, "none").to_string(),
            blush: blush != 0,
            whiskers: whiskers != 0,
            gloves: cstr_or(gloves, "auto").to_string(),
            keyboard_color: cstr_or(keyboard_color, "cream").to_string(),
            mouse_cable: mouse_cable != 0,
            opacity,
            cup: cup != 0,
            festival: festival != 0,
            sound: sound != 0,
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_options(s: *mut KcScene, privacy: u8, sleep_after: f32) {
    if let Some(s) = s.as_mut() {
        s.scene.privacy = privacy != 0;
        s.scene.sleep_after = if sleep_after > 0.0 { sleep_after } else { 0.0 };
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_clock(
    s: *mut KcScene,
    year: i32,
    month: u32,
    day: u32,
    hour: f32,
) {
    if let Some(s) = s.as_mut() {
        s.scene.set_local_clock(year, month, day, hour);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_pomodoro(s: *mut KcScene, on: u8) {
    if let Some(s) = s.as_mut() {
        s.scene.set_pomodoro(on != 0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_load_memory_json(s: *mut KcScene, json_str: *const c_char) {
    if let Some(s) = s.as_mut() {
        let raw = cstr_or(json_str, "");
        if !raw.is_empty() {
            s.scene.life.load_json(raw);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_take_memory_json(s: *mut KcScene, force: u8) -> *mut c_char {
    let Some(s) = s.as_mut() else { return ptr::null_mut() };
    let now = s.scene.time;
    let Some(json) = s.scene.life.take_dirty_json(now, force != 0) else {
        return ptr::null_mut();
    };
    CString::new(json).map(CString::into_raw).unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn kc_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_tuning_json(s: *mut KcScene, json_str: *const c_char) {
    if let Some(s) = s.as_mut() {
        let raw = cstr_or(json_str, "{}");
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) {
            s.scene.tuning = Tuning::from_json(&map);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_set_tuning_value(s: *mut KcScene, key: *const c_char, value: f32) {
    if let Some(s) = s.as_mut() {
        let k = cstr_or(key, "");
        if !k.is_empty() {
            s.scene.tuning.set_value(k, value);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_get_tuning_value(s: *const KcScene, key: *const c_char) -> f32 {
    if let Some(s) = s.as_ref() {
        let k = cstr_or(key, "");
        return s.scene.tuning.get_value(k);
    }
    0.0
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_reset_tuning(s: *mut KcScene) {
    if let Some(s) = s.as_mut() {
        s.scene.tuning = Tuning::default();
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_apply_preset(s: *mut KcScene, index: u32) {
    if let Some(s) = s.as_mut() {
        s.scene.tuning.apply_preset(index as usize);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_matching_preset(s: *const KcScene) -> i32 {
    s.as_ref()
        .and_then(|s| s.scene.tuning.matching_preset())
        .map(|i| i as i32)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_annoy_now(s: *mut KcScene) {
    if let Some(s) = s.as_mut() {
        s.scene.annoy_now();
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_curious_now(s: *mut KcScene, seconds: f32) {
    if let Some(s) = s.as_mut() {
        s.scene.curious_now(seconds);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_static_mesh(s: *const KcScene, mesh_id: u32) -> KcMeshView {
    let Some(s) = s.as_ref() else { return empty_mesh_view() };
    let c = &s.scene.cat;
    let kb = &s.scene.keyboard;
    let mp = &s.scene.mouse;
    let cup = &s.scene.cup;
    match mesh_id {
        KC_MESH_BODY => mesh_view(&c.body_geo),
        KC_MESH_HEAD => mesh_view(&c.head_geo),
        KC_MESH_PAW => mesh_view(&c.paw_geo),
        KC_MESH_TOE => mesh_view(&c.toe_geo),
        KC_MESH_DISC => mesh_view(&c.disc_geo),
        KC_MESH_CAP_START => mesh_view(&c.cap_start_geo),
        KC_MESH_CAP_END => mesh_view(&c.cap_end_geo),
        KC_MESH_ARC => mesh_view(&c.arc_mesh),
        KC_MESH_NOSE => mesh_view(&c.nose_geo),
        KC_MESH_MOUTH => mesh_view(&c.mouth_mesh),
        KC_MESH_COLLAR_FILL => mesh_view(&c.collar_band.body.mesh),
        KC_MESH_COLLAR_RIM => mesh_view(&c.collar_band.rim.mesh),
        KC_MESH_BELL_BALL => mesh_view(&c.bell_ball_geo),
        KC_MESH_BELL_RIM => mesh_view(&c.bell_rim_geo),
        KC_MESH_BELL_SLIT => mesh_view(&c.bell_slit_geo),
        KC_MESH_BELL_GLINT => mesh_view(&c.bell_glint_geo),
        KC_MESH_BELL_LOOP => mesh_view(&c.bell_loop_geo),
        KC_MESH_KB_CASE => mesh_view(&kb.case_mesh),
        KC_MESH_KB_RIM => mesh_view(&kb.case_rim),
        KC_MESH_KB_PLATE => mesh_view(&kb.plate_mesh),
        KC_MESH_KB_SIDE => mesh_view(&kb.side_geo),
        KC_MESH_KB_TOP => mesh_view(&kb.top_geo),
        KC_MESH_KB_DOT => mesh_view(&kb.dot_geo),
        KC_MESH_MOUSE_DOME => mesh_view(&mp.dome_geo),
        KC_MESH_MOUSE_WHEEL => mesh_view(&mp.wheel_geo),
        KC_MESH_MOUSE_SEAM => mesh_view(&mp.seam.mesh),
        KC_MESH_MOUSE_BTN => mesh_view(&mp.button_geo),
        id if (KC_MESH_WHISKER_0..KC_MESH_WHISKER_0 + 6).contains(&id) => {
            c.whiskers.get((id - KC_MESH_WHISKER_0) as usize).map(mesh_view).unwrap_or_else(empty_mesh_view)
        }
        KC_MESH_CUP_BODY => mesh_view(&cup.body_geo),
        KC_MESH_CUP_RIM => mesh_view(&cup.rim_geo),
        KC_MESH_CUP_HANDLE => mesh_view(&cup.handle_geo),
        KC_MESH_CUP_COFFEE => mesh_view(&cup.coffee_geo),
        KC_MESH_HAT_CONE => mesh_view(&c.hat_cone_geo),
        KC_MESH_HAT_BRIM => mesh_view(&c.hat_brim_geo),
        KC_MESH_HAT_RING => mesh_view(&c.hat_ring_geo),
        _ => empty_mesh_view(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_ear_mesh(s: *const KcScene, ear_key: u32, is_rim: u8) -> KcMeshView {
    let Some(s) = s.as_ref() else { return empty_mesh_view() };
    let eg = s.scene.cat.ear_geos.iter().find(|e| e.key == ear_key).or_else(|| s.scene.cat.ear_geos.first());
    match eg {
        Some(e) if is_rim != 0 => mesh_view(&e.rim),
        Some(e) => mesh_view(&e.fill),
        None => empty_mesh_view(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_dynamic_mesh(s: *const KcScene, dyn_id: u32) -> KcMeshView {
    let Some(s) = s.as_ref() else { return empty_mesh_view() };
    match dyn_id {
        KC_DYN_CABLE => mesh_view(&s.scene.mouse.cable.mesh),
        KC_DYN_TAIL_FILL => mesh_view(&s.scene.cat.tail.body.mesh),
        KC_DYN_TAIL_RIM => mesh_view(&s.scene.cat.tail.rim.mesh),
        KC_DYN_ARM_FILL_0 => mesh_view(&s.scene.cat.arms[0].limb.body.mesh),
        KC_DYN_ARM_RIM_0 => mesh_view(&s.scene.cat.arms[0].limb.rim.mesh),
        KC_DYN_ARM_FILL_1 => mesh_view(&s.scene.cat.arms[1].limb.body.mesh),
        KC_DYN_ARM_RIM_1 => mesh_view(&s.scene.cat.arms[1].limb.rim.mesh),
        KC_DYN_STEAM_A => mesh_view(&s.scene.cup.steam_a.mesh),
        KC_DYN_STEAM_B => mesh_view(&s.scene.cup.steam_b.mesh),
        _ => empty_mesh_view(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_kb_instances(
    s: *const KcScene,
    slot: u32,
    out_count: *mut u32,
) -> *const KeyInstance {
    let Some(s) = s.as_ref() else {
        if !out_count.is_null() {
            *out_count = 0;
        }
        return ptr::null();
    };
    let slice = match slot {
        0 => &s.scene.keyboard.side_instances,
        1 => &s.scene.keyboard.top_instances,
        2 => &s.scene.keyboard.dot_instances,
        _ => {
            if !out_count.is_null() {
                *out_count = 0;
            }
            return ptr::null();
        }
    };
    if !out_count.is_null() {
        *out_count = slice.len() as u32;
    }
    slice.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn kc_scene_prepare_frame(s: *mut KcScene) -> KcFrameData {
    s.as_mut().expect("kc_scene_prepare_frame null scene").build_frame()
}

struct TuningStrings {
    group: CString,
    key: CString,
    label: CString,
    hint: CString,
    unit: CString,
    is_toggle: u8,
    min_val: f32,
    max_val: f32,
    step: f32,
    def_val: f32,
}

fn tuning_cache() -> &'static [TuningStrings] {
    static CACHE: OnceLock<Vec<TuningStrings>> = OnceLock::new();
    CACHE.get_or_init(|| {
        TUNING_SCHEMA
            .iter()
            .map(|m| TuningStrings {
                group: CString::new(m.group).unwrap_or_default(),
                key: CString::new(m.key).unwrap_or_default(),
                label: CString::new(m.label).unwrap_or_default(),
                hint: CString::new(m.hint).unwrap_or_default(),
                unit: CString::new(m.unit).unwrap_or_default(),
                is_toggle: u8::from(m.kind == ParamKind::Toggle),
                min_val: m.min,
                max_val: m.max,
                step: m.step,
                def_val: m.def,
            })
            .collect()
    })
}

#[no_mangle]
pub extern "C" fn kc_tuning_count() -> u32 {
    tuning_cache().len() as u32
}

#[no_mangle]
pub extern "C" fn kc_tuning_meta(index: u32) -> KcTuningMeta {
    if let Some(t) = tuning_cache().get(index as usize) {
        KcTuningMeta {
            group: t.group.as_ptr(),
            key: t.key.as_ptr(),
            label: t.label.as_ptr(),
            hint: t.hint.as_ptr(),
            unit: t.unit.as_ptr(),
            is_toggle: t.is_toggle,
            min_val: t.min_val,
            max_val: t.max_val,
            step: t.step,
            def_val: t.def_val,
        }
    } else {
        KcTuningMeta {
            group: ptr::null(),
            key: ptr::null(),
            label: ptr::null(),
            hint: ptr::null(),
            unit: ptr::null(),
            is_toggle: 0,
            min_val: 0.0,
            max_val: 0.0,
            step: 0.0,
            def_val: 0.0,
        }
    }
}

struct PresetStrings {
    name: CString,
    hint: CString,
}

fn preset_cache() -> &'static [PresetStrings] {
    static CACHE: OnceLock<Vec<PresetStrings>> = OnceLock::new();
    CACHE.get_or_init(|| {
        TUNING_PRESETS
            .iter()
            .map(|p| PresetStrings {
                name: CString::new(p.name).unwrap_or_default(),
                hint: CString::new(p.hint).unwrap_or_default(),
            })
            .collect()
    })
}

#[no_mangle]
pub extern "C" fn kc_preset_count() -> u32 {
    preset_cache().len() as u32
}

#[no_mangle]
pub extern "C" fn kc_preset_name(index: u32) -> *const c_char {
    preset_cache().get(index as usize).map(|p| p.name.as_ptr()).unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn kc_preset_hint(index: u32) -> *const c_char {
    preset_cache().get(index as usize).map(|p| p.hint.as_ptr()).unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn kc_audio_new(sample_rate: u32) -> *mut KcAudio {
    Box::into_raw(Box::new(KcAudio {
        synth: Mutex::new(PurrSynth::new(sample_rate)),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn kc_audio_free(a: *mut KcAudio) {
    if !a.is_null() {
        drop(Box::from_raw(a));
    }
}

#[no_mangle]
pub unsafe extern "C" fn kc_audio_sync_from_scene(a: *mut KcAudio, s: *const KcScene) -> u8 {
    let (Some(a), Some(s)) = (a.as_ref(), s.as_ref()) else { return 0 };
    let (enabled, level, breath) = s.scene.purr_audio_state();
    if let Ok(mut synth) = a.synth.lock() {
        synth.set_target(enabled, level, breath);
        return u8::from(synth.is_active());
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn kc_audio_fill_f32(a: *mut KcAudio, out: *mut f32, len: u32) {
    if a.is_null() || out.is_null() || len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(out, len as usize);
    if let Ok(mut synth) = (*a).synth.lock() {
        synth.fill_f32(slice);
    } else {
        slice.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_frame_preparation_and_static_meshes_are_valid() {
        unsafe {
            let s = kc_scene_new();
            assert!(!s.is_null());
            kc_scene_set_viewport(s, 360.0, 368.0);
            kc_scene_set_clock(s, 2026, 12, 24, 15.0);
            let coat = CString::new("calico").unwrap();
            let auto = CString::new("auto").unwrap();
            let shine = CString::new("sparkle").unwrap();
            let collar = CString::new("red").unwrap();
            let pink = CString::new("pink").unwrap();
            kc_scene_set_look_ext(
                s, 1, coat.as_ptr(), auto.as_ptr(), auto.as_ptr(), shine.as_ptr(), collar.as_ptr(),
                1, 1, pink.as_ptr(), pink.as_ptr(), 1, 0.85, 1, 1, 1,
            );

            for id in 0..40u32 {
                let mv = kc_scene_static_mesh(s, id);
                assert!(!mv.vertices.is_null() && mv.vertex_count > 0, "static mesh {id} empty");
                assert!(!mv.indices.is_null() && mv.index_count > 0, "static mesh {id} indices empty");
            }
            for dyn_id in 0..9u32 {
                let mv = kc_scene_dynamic_mesh(s, dyn_id);
                assert!(!mv.vertices.is_null() && mv.vertex_count > 0, "dyn mesh {dyn_id} empty");
            }

            let key_f = CString::new("KeyF").unwrap();
            kc_scene_on_key(s, key_f.as_ptr(), 1);
            for _ in 0..10 {
                kc_scene_step(s, 1.0 / 60.0);
            }
            assert_eq!(kc_scene_on_cursor(s, 171.0, 135.0), KC_HIT_PET);
            assert_eq!(kc_scene_on_cursor(s, 171.0, 240.0), KC_HIT_DRAG);
            assert_eq!(kc_scene_on_cursor(s, 20.0, 20.0), KC_HIT_NONE);
            let frame = kc_scene_prepare_frame(s);
            assert!(frame.draw_count >= 55, "expected full draw list, got {}", frame.draw_count);
            assert!(frame.sprite_count >= 1, "expected key popup sprite");
            assert!((frame.opacity - 0.85).abs() < 1e-3);

            assert!(kc_tuning_count() >= 20);
            assert_eq!(kc_preset_count(), 5);
            kc_scene_apply_preset(s, 2);
            assert_eq!(kc_scene_matching_preset(s), 2);

            let audio = kc_audio_new(48_000);
            let _ = kc_audio_sync_from_scene(audio, s);
            let mut buf = [0.0f32; 128];
            kc_audio_fill_f32(audio, buf.as_mut_ptr(), 128);
            kc_audio_free(audio);

            let mem = kc_scene_take_memory_json(s, 1);
            assert!(!mem.is_null());
            kc_string_free(mem);

            kc_scene_free(s);
        }
    }
}
