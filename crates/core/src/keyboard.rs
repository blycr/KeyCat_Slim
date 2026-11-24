//! 3D 60% keyboard (61 instanced keycaps) and mouse prop — 1:1 mirror of `web/keyboard.js`.

use std::collections::{HashMap, HashSet};

use crate::geometry::{
    circle, cylinder_open, full_sphere, plane, rounded_slab, sphere, MeshData, TaperTube,
};
use crate::math::{approach, hex_color, mix, smooth, Mat4, Spring3, Vec3, PI, SQRT_1_2, TAU};

pub const KEY_U: f32 = 0.15;
pub const KEY_CAP: f32 = 0.129;
pub const BOARD_W: f32 = 15.0 * KEY_U + 0.16;
pub const BOARD_D: f32 = 5.0 * KEY_U + 0.16;
pub const BOARD_H: f32 = 0.12;
pub const CAP_H: f32 = 0.058;

pub const MW: f32 = 0.145;
pub const MH: f32 = 0.1;
pub const ML: f32 = 0.21;
pub const MOUSE_HOME: Vec3 = Vec3::new(-1.2, 0.02, 0.96);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    Base,
    Mod,
    Orange,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct KeyInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct KeyState {
    pub idx: usize,
    pub code: &'static str,
    pub label: &'static str,
    pub kind: KeyKind,
    pub base_color: [f32; 3],
    pub side_color: [f32; 3],
    pub x: f32,
    pub z: f32,
    pub sx: f32,
    pub down: bool,
    pub glow: f32,
    pub depth: f32,
}

pub struct Keyboard {
    pub pos: Vec3,
    pub rot_x: f32,
    pub case_mesh: MeshData,
    pub case_rim: MeshData,
    pub plate_mesh: MeshData,
    pub side_geo: MeshData,
    pub top_geo: MeshData,
    pub dot_geo: MeshData,
    pub keys: Vec<KeyState>,
    code_to_idx: HashMap<&'static str, usize>,
    aliases: HashMap<&'static str, &'static str>,
    left_hand: HashSet<&'static str>,
    active_keys: HashSet<usize>,
    pub side_instances: Vec<KeyInstance>,
    pub top_instances: Vec<KeyInstance>,
    pub dot_instances: Vec<KeyInstance>,
    pub instances_dirty: bool,
}

impl Keyboard {
    pub fn new() -> Self {
        let rows: &[&[(&'static str, f32, &'static str, KeyKind)]] = &[
            &[
                ("Escape", 1.0, "esc", KeyKind::Orange), ("Digit1", 1.0, "1", KeyKind::Base),
                ("Digit2", 1.0, "2", KeyKind::Base), ("Digit3", 1.0, "3", KeyKind::Base),
                ("Digit4", 1.0, "4", KeyKind::Base), ("Digit5", 1.0, "5", KeyKind::Base),
                ("Digit6", 1.0, "6", KeyKind::Base), ("Digit7", 1.0, "7", KeyKind::Base),
                ("Digit8", 1.0, "8", KeyKind::Base), ("Digit9", 1.0, "9", KeyKind::Base),
                ("Digit0", 1.0, "0", KeyKind::Base), ("Minus", 1.0, "-", KeyKind::Base),
                ("Equal", 1.0, "=", KeyKind::Base), ("Backspace", 2.0, "⌫", KeyKind::Mod),
            ],
            &[
                ("Tab", 1.5, "⇥", KeyKind::Mod), ("KeyQ", 1.0, "Q", KeyKind::Base),
                ("KeyW", 1.0, "W", KeyKind::Base), ("KeyE", 1.0, "E", KeyKind::Base),
                ("KeyR", 1.0, "R", KeyKind::Base), ("KeyT", 1.0, "T", KeyKind::Base),
                ("KeyY", 1.0, "Y", KeyKind::Base), ("KeyU", 1.0, "U", KeyKind::Base),
                ("KeyI", 1.0, "I", KeyKind::Base), ("KeyO", 1.0, "O", KeyKind::Base),
                ("KeyP", 1.0, "P", KeyKind::Base), ("BracketLeft", 1.0, "[", KeyKind::Base),
                ("BracketRight", 1.0, "]", KeyKind::Base), ("Backslash", 1.5, "\\", KeyKind::Mod),
            ],
            &[
                ("CapsLock", 1.75, "⇪", KeyKind::Mod), ("KeyA", 1.0, "A", KeyKind::Base),
                ("KeyS", 1.0, "S", KeyKind::Base), ("KeyD", 1.0, "D", KeyKind::Base),
                ("KeyF", 1.0, "F", KeyKind::Base), ("KeyG", 1.0, "G", KeyKind::Base),
                ("KeyH", 1.0, "H", KeyKind::Base), ("KeyJ", 1.0, "J", KeyKind::Base),
                ("KeyK", 1.0, "K", KeyKind::Base), ("KeyL", 1.0, "L", KeyKind::Base),
                ("Semicolon", 1.0, ";", KeyKind::Base), ("Quote", 1.0, "'", KeyKind::Base),
                ("Enter", 2.25, "⏎", KeyKind::Orange),
            ],
            &[
                ("ShiftLeft", 2.25, "⇧", KeyKind::Mod), ("KeyZ", 1.0, "Z", KeyKind::Base),
                ("KeyX", 1.0, "X", KeyKind::Base), ("KeyC", 1.0, "C", KeyKind::Base),
                ("KeyV", 1.0, "V", KeyKind::Base), ("KeyB", 1.0, "B", KeyKind::Base),
                ("KeyN", 1.0, "N", KeyKind::Base), ("KeyM", 1.0, "M", KeyKind::Base),
                ("Comma", 1.0, ",", KeyKind::Base), ("Period", 1.0, ".", KeyKind::Base),
                ("Slash", 1.0, "/", KeyKind::Base), ("ShiftRight", 1.75, "⇧", KeyKind::Mod),
                ("ArrowUp", 1.0, "↑", KeyKind::Mod),
            ],
            &[
                ("ControlLeft", 1.25, "⌃", KeyKind::Mod), ("AltLeft", 1.25, "⌥", KeyKind::Mod),
                ("MetaLeft", 1.25, "⌘", KeyKind::Mod), ("Space", 6.25, "space", KeyKind::Base),
                ("MetaRight", 1.0, "⌘", KeyKind::Mod), ("ArrowLeft", 1.0, "←", KeyKind::Mod),
                ("ArrowDown", 1.0, "↓", KeyKind::Mod), ("ArrowRight", 1.0, "→", KeyKind::Mod),
            ],
        ];

        let mut aliases = HashMap::from([
            ("Backquote", "Escape"), ("Delete", "Backspace"), ("Fn", "ControlLeft"),
            ("ControlRight", "MetaRight"), ("AltRight", "MetaRight"),
            ("IntlBackslash", "ShiftLeft"), ("NumpadEnter", "Enter"), ("ContextMenu", "MetaRight"),
            ("F1", "Digit1"), ("F2", "Digit2"), ("F3", "Digit3"), ("F4", "Digit4"),
            ("F5", "Digit5"), ("F6", "Digit6"), ("F7", "Digit7"), ("F8", "Digit8"),
            ("F9", "Digit9"), ("F10", "Digit0"), ("F11", "Minus"), ("F12", "Equal"),
            ("Numpad0", "Digit0"), ("Numpad1", "Digit1"), ("Numpad2", "Digit2"),
            ("Numpad3", "Digit3"), ("Numpad4", "Digit4"), ("Numpad5", "Digit5"),
            ("Numpad6", "Digit6"), ("Numpad7", "Digit7"), ("Numpad8", "Digit8"),
            ("Numpad9", "Digit9"), ("NumpadAdd", "Equal"), ("NumpadSubtract", "Minus"),
            ("NumpadMultiply", "Digit8"), ("NumpadDivide", "Slash"),
            ("NumpadDecimal", "Period"), ("NumpadEqual", "Equal"),
        ]);
        let _ = &mut aliases;

        let left_hand = HashSet::from([
            "Escape", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Tab", "KeyQ", "KeyW",
            "KeyE", "KeyR", "KeyT", "CapsLock", "KeyA", "KeyS", "KeyD", "KeyF", "KeyG",
            "ShiftLeft", "KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "ControlLeft", "AltLeft",
            "MetaLeft",
        ]);

        let case_mesh = rounded_slab(BOARD_W, BOARD_D, BOARD_H, 0.07, 0.0);
        let case_rim = rounded_slab(BOARD_W + 0.05, BOARD_D + 0.05, BOARD_H + 0.02, 0.09, -0.01);
        let plate_mesh = rounded_slab(BOARD_W - 0.05, BOARD_D - 0.05, 0.01, 0.05, BOARD_H);

        let mut side_geo = cylinder_open(SQRT_1_2 * 0.8, SQRT_1_2, 1.0, 4);
        side_geo.rotate_y(PI * 0.25);
        side_geo.translate(0.0, 0.5, 0.0);

        let mut top_geo = plane(0.8, 0.8);
        top_geo.rotate_x(-PI * 0.5);
        top_geo.translate(0.0, 1.0, 0.0);

        let mut dot_geo = plane(0.014, 0.02);
        dot_geo.rotate_x(-PI * 0.5);

        let c_base = hex_color("#f8f7ef");
        let c_mod = hex_color("#e0ddd2");
        let c_orange = hex_color("#e9b064");
        let s_base = hex_color("#e6e3d8");
        let s_mod = hex_color("#cbc7ba");
        let s_orange = hex_color("#d39a4e");
        let dot_col = hex_color("#2a2a27");

        let mut keys = Vec::with_capacity(61);
        let mut code_to_idx = HashMap::with_capacity(61);
        let mut side_instances = Vec::with_capacity(61);
        let mut top_instances = Vec::with_capacity(61);
        let mut dot_instances = Vec::with_capacity(61);

        for (r, row) in rows.iter().enumerate() {
            let mut u = 0.0f32;
            for &(code, width, label, kind) in *row {
                let cx = u + width * 0.5;
                u += width;
                let x = (7.5 - cx) * KEY_U;
                let z = (r as f32 - 2.0) * -KEY_U;
                let sx = width * KEY_U - (KEY_U - KEY_CAP);
                let (base_color, side_color) = match kind {
                    KeyKind::Base => (c_base, s_base),
                    KeyKind::Mod => (c_mod, s_mod),
                    KeyKind::Orange => (c_orange, s_orange),
                };
                let idx = keys.len();
                code_to_idx.insert(code, idx);
                let k = KeyState {
                    idx, code, label, kind, base_color, side_color, x, z, sx,
                    down: false, glow: 0.0, depth: 0.0,
                };
                let (m_cap, m_dot) = key_matrices(&k);
                side_instances.push(KeyInstance {
                    model: m_cap.to_cols_array_2d(),
                    color: [side_color[0], side_color[1], side_color[2], 1.0],
                });
                top_instances.push(KeyInstance {
                    model: m_cap.to_cols_array_2d(),
                    color: [base_color[0], base_color[1], base_color[2], 1.0],
                });
                dot_instances.push(KeyInstance {
                    model: m_dot.to_cols_array_2d(),
                    color: [dot_col[0], dot_col[1], dot_col[2], 1.0],
                });
                keys.push(k);
            }
        }

        Self {
            pos: Vec3::new(0.28, 0.02, 1.0),
            rot_x: 0.06,
            case_mesh,
            case_rim,
            plate_mesh,
            side_geo,
            top_geo,
            dot_geo,
            keys,
            code_to_idx,
            aliases,
            left_hand,
            active_keys: HashSet::new(),
            side_instances,
            top_instances,
            dot_instances,
            instances_dirty: true,
        }
    }

    pub fn world_matrix(&self) -> Mat4 {
        Mat4::from_trs_euler(self.pos, Vec3::new(self.rot_x, 0.0, 0.0), Vec3::ONE)
    }

    pub fn resolve(&self, code: &str) -> Option<usize> {
        self.code_to_idx
            .get(code)
            .copied()
            .or_else(|| self.aliases.get(code).and_then(|a| self.code_to_idx.get(a).copied()))
    }

    pub fn is_left_hand(&self, code: &str) -> bool {
        let canonical = self.aliases.get(code).copied().unwrap_or(code);
        self.left_hand.contains(canonical)
    }

    pub fn set_down(&mut self, idx: usize, down: bool) {
        if let Some(k) = self.keys.get_mut(idx) {
            if k.down != down {
                k.down = down;
                self.active_keys.insert(idx);
            }
        }
    }

    pub fn release_all(&mut self) {
        for i in 0..self.keys.len() {
            if self.keys[i].down {
                self.keys[i].down = false;
                self.active_keys.insert(i);
            }
        }
    }

    pub fn key_top(&self, idx: usize) -> Vec3 {
        let k = &self.keys[idx];
        self.world_matrix()
            .transform_point(Vec3::new(k.x, BOARD_H + 0.01 + CAP_H - k.depth, k.z))
    }

    pub fn update(&mut self, dt: f32) {
        if self.active_keys.is_empty() {
            return;
        }
        let hit = hex_color("#b9d84f");
        let mut finished = Vec::new();
        for &idx in &self.active_keys {
            let k = &mut self.keys[idx];
            let target_d = if k.down { 0.032 } else { 0.0 };
            k.depth += (target_d - k.depth) * approach(if k.down { 60.0 } else { 24.0 }, dt);
            let target_g = if k.down { 1.0 } else { 0.0 };
            k.glow += (target_g - k.glow) * approach(if k.down { 40.0 } else { 5.0 }, dt);
            if !k.down && k.depth < 1e-4 && k.glow < 1e-4 {
                k.depth = 0.0;
                k.glow = 0.0;
                finished.push(idx);
            }
            let (m_cap, m_dot) = key_matrices(k);
            self.side_instances[idx].model = m_cap.to_cols_array_2d();
            self.top_instances[idx].model = m_cap.to_cols_array_2d();
            self.dot_instances[idx].model = m_dot.to_cols_array_2d();
            let col = [
                mix(k.base_color[0], hit[0], k.glow),
                mix(k.base_color[1], hit[1], k.glow),
                mix(k.base_color[2], hit[2], k.glow),
                1.0,
            ];
            self.top_instances[idx].color = col;
        }
        for idx in finished {
            self.active_keys.remove(&idx);
        }
        self.instances_dirty = true;
    }

    pub fn raycast(&self, orig: Vec3, dir: Vec3) -> bool {
        let w = self.world_matrix();
        self.case_mesh.raycast(w, orig, dir) || self.plate_mesh.raycast(w, orig, dir)
    }
}

fn key_matrices(k: &KeyState) -> (Mat4, Mat4) {
    let y = BOARD_H + 0.01 - k.depth;
    let mut cap = Mat4::from_scale(Vec3::new(k.sx, CAP_H, KEY_CAP));
    cap.cols[3] = [k.x, y, k.z, 1.0];
    let dot = Mat4::from_translation(Vec3::new(k.x, y + CAP_H + 0.001, k.z - KEY_CAP * 0.36));
    (cap, dot)
}

pub struct MouseProp {
    pub pos: Vec3,
    pub offset: Vec3,
    pub kick: Vec3,
    pub pressed: f32,
    pub follow: Spring3,
    pub shape_scale: Vec3,
    pub dome_geo: MeshData,
    pub wheel_geo: MeshData,
    pub seam: TaperTube,
    pub button_geo: MeshData,
    pub cable: TaperTube,
    pub cable_dirty: bool,
    last_cable_start: Vec3,
    cable_init: bool,
    room: f32,
}

impl MouseProp {
    pub fn new() -> Self {
        let room = 0.28 - (BOARD_W + 0.05) * 0.5 - 0.19 - MOUSE_HOME.x;
        let mut dome_geo = sphere(1.0, 40, 24, 0.0, TAU, 0.0, PI * 0.5);
        let top_c = hex_color("#fbfbf3");
        let side_c = hex_color("#dedad0");
        for v in &mut dome_geo.vertices {
            let (x, y, z) = (v.pos[0], v.pos[1], v.pos[2]);
            v.pos = [x * (1.0 - 0.14 * z), y * (1.0 - 0.18 * z), z];
            v.rest = v.pos;
            let t = smooth(0.15, 0.75, y);
            v.color = [
                mix(side_c[0], top_c[0], t),
                mix(side_c[1], top_c[1], t),
                mix(side_c[2], top_c[2], t),
            ];
        }
        dome_geo.compute_vertex_normals();
        dome_geo.compute_bounds();

        let wheel_geo = full_sphere(1.0, 16, 10);
        let mut seam = TaperTube::new(12, 4, false);
        seam.update(
            |t| {
                let z = mix(ML * 0.02, ML * 0.95, t);
                Vec3::new(0.0, mouse_on_top(0.0, z) + 0.002, z)
            },
            |_| 0.0045,
            0.0,
        );

        let mut button_geo = circle(1.0, 24, 0.0, PI * 0.5);
        button_geo.rotate_x(PI * 0.5);

        let cable = TaperTube::new(24, 6, false);

        Self {
            pos: MOUSE_HOME,
            offset: Vec3::ZERO,
            kick: Vec3::ZERO,
            pressed: 0.0,
            follow: Spring3::new(Vec3::ZERO, 4.0, 0.75),
            shape_scale: Vec3::ONE,
            dome_geo,
            wheel_geo,
            seam,
            button_geo,
            cable,
            cable_dirty: true,
            last_cable_start: Vec3::ZERO,
            cable_init: false,
            room,
        }
    }

    pub fn shape_world(&self) -> Mat4 {
        let group = Mat4::from_translation(self.pos);
        let shape = Mat4::from_trs_euler(Vec3::ZERO, Vec3::new(0.0, -0.18, 0.0), self.shape_scale);
        group.mul(shape)
    }

    pub fn dome_world(&self) -> Mat4 {
        self.shape_world().mul(Mat4::from_scale(Vec3::new(MW, MH, ML)))
    }

    pub fn shell_world(&self, grow: f32) -> Mat4 {
        self.shape_world().mul(Mat4::from_trs_euler(
            Vec3::new(0.0, -grow * 0.3, 0.0),
            Vec3::ZERO,
            Vec3::new(MW + grow, MH + grow, ML + grow),
        ))
    }

    pub fn wheel_world(&self) -> Mat4 {
        self.shape_world().mul(Mat4::from_trs_euler(
            Vec3::new(0.0, mouse_on_top(0.0, ML * 0.5) + 0.004, ML * 0.5),
            Vec3::ZERO,
            Vec3::new(0.016, 0.012, 0.03),
        ))
    }

    pub fn button_world(&self) -> Mat4 {
        self.shape_world().mul(Mat4::from_trs_euler(
            Vec3::new(0.004, mouse_on_top(0.04, ML * 0.4) + 0.001, ML * 0.08),
            Vec3::ZERO,
            Vec3::new(MW * 0.78, 1.0, ML * 0.82),
        ))
    }

    pub fn top(&self) -> Vec3 {
        self.shape_world().transform_point(Vec3::new(0.0, MH + 0.045, -ML * 0.3))
    }

    pub fn update(&mut self, dt: f32, down: bool) {
        self.kick = self.kick * (1.0 - approach(3.0, dt));
        self.offset = self.follow.step(self.kick, dt);
        if self.offset.x > 0.0 {
            self.offset.x = self.room * (self.offset.x / self.room).tanh();
        }
        self.pos = MOUSE_HOME + self.offset;
        self.pressed += ((if down { 1.0 } else { 0.0 }) - self.pressed) * approach(30.0, dt);
        self.shape_scale = Vec3::new(1.0 + 0.03 * self.pressed, 1.0 - 0.1 * self.pressed, 1.0);

        let cable_start = self.shape_world().transform_point(Vec3::new(0.0, MH * 0.25, ML * 0.98));
        if !self.cable_init || cable_start.distance_sq(self.last_cable_start) > 1e-10 {
            self.cable_init = true;
            self.last_cable_start = cable_start;
            let cable_end = Vec3::new(-1.6, 0.02, 1.7);
            let cable_mid = Vec3::new(
                (cable_start.x + cable_end.x) * 0.5 + 0.1,
                0.02,
                (cable_start.z + cable_end.z) * 0.5 + 0.12,
            );
            self.cable.update(
                |t| {
                    let u = 1.0 - t;
                    Vec3::new(
                        u * u * cable_start.x + 2.0 * u * t * cable_mid.x + t * t * cable_end.x,
                        u * u * cable_start.y + 2.0 * u * t * cable_mid.y + t * t * cable_end.y,
                        u * u * cable_start.z + 2.0 * u * t * cable_mid.z + t * t * cable_end.z,
                    )
                },
                |_| 0.011,
                0.0,
            );
            self.cable_dirty = true;
        }
    }

    pub fn raycast(&self, orig: Vec3, dir: Vec3) -> bool {
        self.dome_geo.raycast(self.dome_world(), orig, dir)
    }
}

fn mouse_on_top(x: f32, z: f32) -> f32 {
    let denom = MW * (1.0 - 0.14 * z / ML);
    let inside = (1.0 - (x / denom).powi(2) - (z / ML).powi(2)).max(0.0);
    MH * (1.0 - 0.18 * z / ML) * inside.sqrt()
}
