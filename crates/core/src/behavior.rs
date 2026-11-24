//! Full interactive behaviour state machine, camera projection, sprite billboards and hit testing —
//! 1:1 mirror of `web/behavior.js`.

use std::collections::VecDeque;

use crate::cat::Cat;
use crate::coats::Look;
use crate::keyboard::{CupProp, Keyboard, MouseProp, CUP_HOME};
use crate::life::{Festival, LifeMemory};
use crate::math::{approach, clamp, mix, smooth, Mat4, Spring, Spring3, Vec2, Vec3, PI};
use crate::tuning::Tuning;

pub const FRAME_W: f32 = 539.0;
pub const FRAME_H: f32 = 458.0;
pub const CROP_X: f32 = 95.0;
pub const CROP_Y: f32 = 34.0;
pub const CROP_W: f32 = 360.0;
pub const CROP_H: f32 = 368.0;
pub const AIM_X: f32 = 294.0;
pub const AIM_Y: f32 = 355.0;
pub const CAM_FOV: f32 = 16.0;
pub const CAM_ELEV_DEG: f32 = 28.0;
pub const CAM_DIST: f32 = 15.65;
pub const CAM_ANCHOR: Vec3 = Vec3::new(0.28, 0.1, 1.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitZone {
    None,
    Pet,
    Drag,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpriteKey {
    Cap(String),
    SleepZ,
    Anger,
    Bang,
    Dust,
    Heart,
    Bubble(String),
}

#[derive(Clone, Debug)]
pub struct SpriteDraw {
    pub key: SpriteKey,
    pub pos: Vec3,
    pub scale_x: f32,
    pub scale_y: f32,
    pub opacity: f32,
    pub render_order: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FidgetKind {
    Tilt,
    Ear,
    SlowBlink,
    Look,
    Tail,
    Yawn,
    Stretch,
    Lick,
    PushCup,
}

#[derive(Clone, Copy, Debug)]
enum CupAnim {
    Idle,
    Pushing { t0: f32 },
    Falling { t0: f32, pos: Vec3, vel: Vec3, roll: f32 },
    Respawn { t0: f32 },
}

#[derive(Clone, Copy, Debug)]
struct Fidget {
    kind: FidgetKind,
    t0: f32,
    dur: f32,
    side: f32,
}

struct PawState {
    side: f32,
    pos: Vec3,
    from: Vec3,
    last: Vec3,
    key: Option<usize>,
    t0: f32,
    dur: f32,
    down: bool,
    spring: Spring3,
}

struct TailTap {
    key: Option<usize>,
    t0: f32,
    dur: f32,
    from: f32,
    down: bool,
    land_at: f32,
    reach: f32,
    hover: f32,
    tip: Vec3,
}

struct PopupItem {
    key: usize,
    label: String,
    aspect: f32,
    base: Vec3,
    t0: f32,
    up_at: Option<f32>,
}

struct ZItem {
    base: Vec3,
    t0: f32,
}

struct FxItem {
    kind: SpriteKey,
    from: Vec3,
    vel: Vec3,
    t0: f32,
    life: f32,
    size: f32,
    order: i32,
}

pub struct Scene {
    pub cat: Cat,
    pub keyboard: Keyboard,
    pub mouse: MouseProp,
    pub cup: CupProp,
    pub life: LifeMemory,
    pub tuning: Tuning,
    pub privacy: bool,
    pub sleep_after: f32,
    pub sound_enabled: bool,
    pub festival_enabled: bool,
    pub opacity: f32,
    pub pomodoro: bool,
    pub pomo_is_break: bool,
    pomo_phase_start: f32,
    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,
    pub inv_view_proj: Mat4,
    pub cam_pos: Vec3,
    pub cam_right: Vec3,
    pub cam_up: Vec3,
    pub viewport_w: f32,
    pub viewport_h: f32,

    // Animation state
    pub time: f32,
    pub busy_until: f32,
    last_input: f32,
    last_key: f32,
    last_mouse: f32,
    mouse_down: bool,
    cursor: Option<Vec2>,
    cursor_seen: f32,
    cursor_dirty: bool,
    pub hit: HitZone,
    petting: bool,
    pet_until: f32,
    pet_x: f32,
    pet_start: f32,
    pub purr: f32,
    breath_phase: f32,
    next_heart: f32,
    pub sleep: f32,
    pub asleep: bool,
    sleep_start: Option<f32>,
    pub sprawl: f32,
    wake_at: f32,
    next_blink: f32,
    next_z: f32,
    rub: f32,
    excite: f32,
    strokes: VecDeque<f32>,
    tail_phase: f32,
    act: Option<Fidget>,
    next_act: f32,
    annoy: f32,
    last_poke: f32,
    cross: bool,
    cross_since: f32,
    thump_phase: f32,
    thump_lift: f32,
    last_slam: f32,
    moves: VecDeque<(f32, f32)>,
    curious_until: f32,
    caps_release_at: Option<f32>,
    // Care / daily rhythm state
    work_start: Option<f32>,
    work_last: f32,
    pub nag_active: bool,
    nag_snooze_until: f32,
    nag_stop_until: f32,
    last_night_hint: f32,
    last_needy: f32,
    cup_anim: CupAnim,
    bubble: Option<(String, f32, f32)>,

    // Pose springs
    s_shift: Spring,
    s_hunch: Spring,
    s_squash: Spring,
    s_yaw: Spring,
    s_pitch: Spring,
    s_roll: Spring,
    s_gaze_x: Spring,
    s_gaze_y: Spring,
    s_ears: Spring,
    s_cross: Spring,
    s_curious: Spring,
    s_desk: Spring,

    paw_l: PawState,
    paw_r: PawState,
    tail_tap: TailTap,
    popups: Vec<PopupItem>,
    zs: Vec<ZItem>,
    effects: Vec<FxItem>,
    pub sprites: Vec<SpriteDraw>,
    rng: u64,
}

impl Scene {
    pub fn new() -> Self {
        let e = CAM_ELEV_DEG * PI / 180.0;
        let cam_pos = Vec3::new(
            CAM_ANCHOR.x,
            CAM_ANCHOR.y + CAM_DIST * e.sin(),
            CAM_ANCHOR.z + CAM_DIST * e.cos(),
        );
        let view = Mat4::look_at_rh(cam_pos, CAM_ANCHOR, Vec3::new(0.0, 1.0, 0.0));
        let proj = Mat4::perspective_rh_zo_offset(
            CAM_FOV,
            FRAME_W,
            FRAME_H,
            CROP_X + FRAME_W * 0.5 - AIM_X,
            CROP_Y + FRAME_H * 0.5 - AIM_Y,
            CROP_W,
            CROP_H,
            0.1,
            100.0,
        );
        let view_proj = proj.mul(view);
        let inv_view_proj = view_proj.inverse().unwrap_or(Mat4::IDENTITY);
        let cam_right = Vec3::new(view.cols[0][0], view.cols[1][0], view.cols[2][0]).normalize();
        let cam_up = Vec3::new(view.cols[0][1], view.cols[1][1], view.cols[2][1]).normalize();

        let make_paw = |side: f32| {
            let p = Vec3::new(side * 0.36, 0.26, 0.68);
            PawState {
                side,
                pos: p,
                from: Vec3::ZERO,
                last: p,
                key: None,
                t0: -100.0,
                dur: 0.08,
                down: false,
                spring: Spring3::new(p, 3.2, 0.78),
            }
        };

        let mut scene = Self {
            cat: Cat::new(),
            keyboard: Keyboard::new(),
            mouse: MouseProp::new(),
            cup: CupProp::new(),
            life: LifeMemory::new(),
            tuning: Tuning::default(),
            privacy: false,
            sleep_after: 60.0,
            sound_enabled: false,
            festival_enabled: true,
            opacity: 1.0,
            pomodoro: false,
            pomo_is_break: false,
            pomo_phase_start: 0.0,
            view,
            proj,
            view_proj,
            inv_view_proj,
            cam_pos,
            cam_right,
            cam_up,
            viewport_w: CROP_W,
            viewport_h: CROP_H,

            time: 0.0,
            busy_until: 2.0,
            last_input: 0.0,
            last_key: -100.0,
            last_mouse: -100.0,
            mouse_down: false,
            cursor: None,
            cursor_seen: -100.0,
            cursor_dirty: false,
            hit: HitZone::None,
            petting: false,
            pet_until: 0.0,
            pet_x: 0.0,
            pet_start: -1.0,
            purr: 0.0,
            breath_phase: 0.0,
            next_heart: 0.0,
            sleep: 0.0,
            asleep: false,
            sleep_start: None,
            sprawl: 0.0,
            wake_at: -100.0,
            next_blink: 3.0,
            next_z: 0.0,
            rub: 0.0,
            excite: 0.0,
            strokes: VecDeque::new(),
            tail_phase: 0.0,
            act: None,
            next_act: 4.0,
            annoy: 0.0,
            last_poke: 0.0,
            cross: false,
            cross_since: 0.0,
            thump_phase: 0.0,
            thump_lift: 0.0,
            last_slam: 0.0,
            moves: VecDeque::new(),
            curious_until: -100.0,
            caps_release_at: None,
            work_start: None,
            work_last: -1000.0,
            nag_active: false,
            nag_snooze_until: -1000.0,
            nag_stop_until: -1000.0,
            last_night_hint: -1000.0,
            last_needy: 0.0,
            cup_anim: CupAnim::Idle,
            bubble: None,

            s_shift: Spring::new(0.0, 1.8, 0.72),
            s_hunch: Spring::new(0.0, 1.6, 0.8),
            s_squash: Spring::new(0.0, 2.2, 0.55),
            s_yaw: Spring::new(0.0, 2.6, 0.7),
            s_pitch: Spring::new(-0.28, 2.4, 0.68),
            s_roll: Spring::new(0.0, 2.2, 0.6),
            s_gaze_x: Spring::new(0.0, 7.0, 0.9),
            s_gaze_y: Spring::new(0.0, 7.0, 0.9),
            s_ears: Spring::new(0.0, 5.0, 0.35),
            s_cross: Spring::new(0.0, 1.6, 0.9),
            s_curious: Spring::new(0.0, 3.5, 0.75),
            s_desk: Spring::new(0.0, 9.0, 0.35),

            paw_l: make_paw(-1.0),
            paw_r: make_paw(1.0),
            tail_tap: TailTap {
                key: None,
                t0: -100.0,
                dur: 0.18,
                from: 0.0,
                down: false,
                land_at: -100.0,
                reach: 0.0,
                hover: 0.12,
                tip: Vec3::ZERO,
            },
            popups: Vec::new(),
            zs: Vec::new(),
            effects: Vec::new(),
            sprites: Vec::new(),
            rng: 0x9e37_79b9_7f4a_7c15,
        };
        // Initialize arms/tail/cable once so frame 0 is ready to render immediately.
        scene.step(1.0 / 60.0);
        scene
    }

    fn rand01(&mut self) -> f32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let v = x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40;
        (v as f32) / ((1u32 << 24) as f32)
    }

    pub fn set_viewport(&mut self, w: f32, h: f32) {
        self.viewport_w = w.max(1.0);
        self.viewport_h = h.max(1.0);
    }

    pub fn set_look(&mut self, look: Look) {
        self.keyboard.set_theme(&look.keyboard_color);
        self.mouse.cable_visible = look.mouse_cable;
        self.opacity = clamp(look.opacity, 0.3, 1.0);
        self.cup.enabled = look.cup;
        self.festival_enabled = look.festival;
        self.sound_enabled = look.sound;
        self.cat.set_look(look);
        self.sync_outfit();
        self.busy_until = self.busy_until.max(self.time + 1.0);
    }

    pub fn set_local_clock(&mut self, year: i32, month: u32, day: u32, hour: f32) {
        self.life.set_clock(year, month, day, hour);
        self.sync_outfit();
    }

    fn sync_outfit(&mut self) {
        let want = if self.festival_enabled {
            self.life.festival()
        } else {
            Festival::None
        };
        if self.cat.outfit != want {
            self.cat.outfit = want;
            self.cat.place_collar();
            self.busy_until = self.busy_until.max(self.time + 0.5);
        }
    }

    pub fn set_pomodoro(&mut self, on: bool) {
        self.pomodoro = on;
        self.pomo_is_break = false;
        self.pomo_phase_start = self.time;
        if on {
            self.nag_active = false;
            self.act = None;
            let mins = self.tuning.pomo_focus.round() as i32;
            self.say(&format!("专注 {mins} 分钟"), 2.4);
        } else {
            self.nag_active = false;
            self.say("番茄钟已结束", 2.0);
        }
        self.busy_until = self.busy_until.max(self.time + 2.5);
    }

    pub fn say(&mut self, text: &str, dur: f32) {
        self.bubble = Some((text.to_string(), self.time, dur.max(0.8)));
        self.busy_until = self.busy_until.max(self.time + dur + 0.5);
    }

    pub fn spawn_heart(&mut self, size: f32) {
        let rx = self.rand01();
        let vx = (self.rand01() - 0.5) * 0.14;
        let pos = self.cat.head_world().translation() + Vec3::new((rx - 0.5) * 0.25, 0.55, 0.12);
        self.effects.push(FxItem {
            kind: SpriteKey::Heart,
            from: pos,
            vel: Vec3::new(vx, 0.36, 0.0),
            t0: self.time,
            life: 1.25,
            size,
            order: 44,
        });
        self.busy_until = self.busy_until.max(self.time + 1.3);
    }

    pub fn purr_audio_state(&self) -> (bool, f32, f32) {
        (self.sound_enabled, self.purr, self.breath_phase)
    }

    /// Double-clicking the cat's head sends a slow blink; it blinks back based on `blink_back` + bond.
    pub fn on_head_double_click(&mut self) {
        if self.asleep || self.cross {
            return;
        }
        self.wake();
        let (lvl, _) = self.life.level();
        let prob = clamp(self.tuning.blink_back / 100.0 + (lvl.saturating_sub(1) as f32) * 0.08, 0.0, 1.0);
        if self.rand01() <= prob {
            self.act = Some(Fidget {
                kind: FidgetKind::SlowBlink,
                t0: self.time,
                dur: 1.4,
                side: 1.0,
            });
            self.life.add_blinks(1.0);
            self.life.add_bond(0.8);
            self.spawn_heart(0.22);
        }
    }

    pub fn is_busy(&self) -> bool {
        self.time < self.busy_until
    }

    fn poke(&mut self, amount: f32, force: bool) {
        if !self.tuning.annoy_on && !force {
            return;
        }
        self.annoy = clamp(self.annoy + amount, 0.0, 1.0);
        self.last_poke = self.time;
    }

    fn wake(&mut self) {
        let t = self.time;
        if self.asleep || self.sleep > 0.3 {
            self.wake_at = t;
            let amt = self.tuning.wake_annoy / 100.0;
            self.poke(amt, false);
        }
        self.asleep = false;
        self.sleep_start = None;
        self.last_input = t;
        self.busy_until = self.busy_until.max(t + 3.0);
        self.act = None;
        self.next_act = t + 4.0 + self.rand01() * 4.0;
    }

    fn note_work_activity(&mut self) {
        let t = self.time;
        if t - self.work_last > 180.0 {
            self.work_start = Some(t);
        } else if self.work_start.is_none() {
            self.work_start = Some(t);
        }
        self.work_last = t;
    }

    fn mousing(&self) -> bool {
        self.mouse_down || (self.time - self.last_mouse) < 1.4
    }

    pub fn on_key(&mut self, code: &str, down: bool) {
        let Some(k_idx) = self.keyboard.resolve(code) else { return };
        let t = self.time;
        if !down {
            if self.tail_tap.key == Some(k_idx) {
                self.tail_tap.down = false;
                return;
            }
            self.keyboard.set_down(k_idx, false);
            if self.paw_l.key == Some(k_idx) {
                self.paw_l.down = false;
            }
            if self.paw_r.key == Some(k_idx) {
                self.paw_r.down = false;
            }
            return;
        }
        self.wake();
        self.note_work_activity();
        self.life.add_keys(1.0);
        if self.nag_active {
            // Cat is sitting on the keyboard telling you to rest; raise a paw to block typing.
            self.nag_stop_until = t + 1.4;
            self.s_squash.kick(0.2);
            self.busy_until = self.busy_until.max(t + 1.6);
            return;
        }
        if self.keyboard.keys[k_idx].down
            || (self.tail_tap.key == Some(k_idx) && self.tail_tap.down)
        {
            return;
        }
        if self.keyboard.keys[k_idx].code == "Space" && !self.cross {
            self.tail_tap.from = self.tail_tap.reach;
            self.tail_tap.dur = mix(0.18, 0.07, self.tail_tap.reach);
            self.tail_tap.key = Some(k_idx);
            self.tail_tap.t0 = t;
            self.tail_tap.down = true;
            self.strokes.push_back(t);
            let mash = self.strokes.iter().filter(|&&s| t - s < 0.15).count();
            if mash >= self.tuning.mash_keys {
                self.poke(0.3, false);
            }
            return;
        }
        let use_r = self.keyboard.is_left_hand(code) || self.mousing();
        let top = self.keyboard.key_top(k_idx);
        let paw = if use_r { &mut self.paw_r } else { &mut self.paw_l };
        paw.from = paw.pos;
        paw.key = Some(k_idx);
        paw.t0 = t;
        paw.dur = clamp(paw.pos.distance(top) * 0.09, 0.05, 0.1);
        paw.down = true;
        let paw_side = paw.side;
        self.keyboard.set_down(k_idx, true);
        self.last_key = t;
        self.strokes.push_back(t);
        let mash = self.strokes.iter().filter(|&&s| t - s < 0.15).count();
        if mash >= self.tuning.mash_keys {
            self.poke(0.3, false);
        }
        self.s_squash.kick(0.35);
        self.s_pitch.kick(0.9);
        if code == "CapsLock" {
            self.caps_release_at = Some(t + 0.12);
        }
        if !self.privacy {
            let label = if code.starts_with('F') && code[1..].bytes().all(|b| b.is_ascii_digit()) {
                code.to_string()
            } else {
                self.keyboard.keys[k_idx].label.to_string()
            };
            self.spawn_popup(k_idx, paw_side, label);
        }
    }

    pub fn on_mouse_move(&mut self, dx: f32, dy: f32) {
        if dx.abs() + dy.abs() < 0.5 {
            return;
        }
        let t = self.time;
        self.last_mouse = t;
        self.note_work_activity();
        self.moves.push_back((t, dx.hypot(dy)));
        self.mouse.kick.x = clamp(self.mouse.kick.x + dx * 0.004, -0.16, 0.16);
        self.mouse.kick.z = clamp(self.mouse.kick.z + dy * 0.004, -0.12, 0.12);
        self.wake();
    }

    pub fn on_mouse_button(&mut self, down: bool) {
        self.mouse_down = down;
        self.last_mouse = self.time;
        if down {
            self.note_work_activity();
        }
        self.wake();
    }

    pub fn on_cursor(&mut self, x: f32, y: f32) {
        let next = Vec2::new(x, y);
        if self.cursor.map_or(true, |c| (c.x - x).abs() + (c.y - y).abs() > 3.0) {
            self.cursor_seen = self.time;
        }
        if self.cursor != Some(next) {
            self.cursor_dirty = true;
        }
        self.cursor = Some(next);
        self.busy_until = self.busy_until.max(self.time + 0.6);
    }

    pub fn on_pet(&mut self, down: bool, x: f32) {
        let t = self.time;
        let was_petting = self.petting;
        self.petting = down;
        self.pet_start = if down {
            if self.pet_start < 0.0 { t } else { self.pet_start }
        } else {
            -1.0
        };
        if down {
            self.pet_until = t + 0.9;
            let hs = self.project_screen(self.cat.head_world().translation());
            self.pet_x = (x - hs.x) / 120.0;
            if !was_petting {
                self.life.add_pets(1.0);
                self.life.add_bond(0.4);
                if self.nag_active {
                    self.nag_active = false;
                    self.nag_snooze_until = t + 600.0;
                    self.say("好吧，再给你 10 分钟", 2.4);
                }
            }
            self.wake();
        }
    }

    pub fn is_cross(&self) -> bool {
        self.cross
    }

    pub fn annoy_now(&mut self) {
        self.wake();
        self.curious_until = -100.0;
        self.poke(1.0, true);
        self.cross = true;
        self.cross_since = self.time;
        self.thump_phase = 0.0;
        self.act = None;
        self.busy_until = self.busy_until.max(self.time + 4.0);
    }

    pub fn curious_now(&mut self, seconds: f32) {
        let t = self.time;
        self.cross = false;
        self.annoy = 0.0;
        self.curious_until = t + seconds;
        self.busy_until = self.busy_until.max(t + seconds + 1.0);
        self.wake();
    }

    pub fn sleep_now(&mut self) {
        self.asleep = true;
        self.last_input = self.time - self.sleep_after;
    }

    fn spawn_popup(&mut self, k_idx: usize, side: f32, label: String) {
        self.popups.retain(|p| p.key != k_idx);
        let wide = label.chars().count() > 1;
        let base = self.keyboard.key_top(k_idx) + Vec3::new(side * 0.3, 0.2, -0.1);
        self.popups.push(PopupItem {
            key: k_idx,
            label,
            aspect: if wide { 2.0 } else { 1.0 },
            base,
            t0: self.time,
            up_at: None,
        });
    }

    fn slam(&mut self) {
        let t = self.time;
        self.last_slam = t;
        let mut tip = self.cat.tail_tip();
        tip.y = 0.08;
        self.effects.push(FxItem {
            kind: SpriteKey::Bang,
            from: tip,
            vel: Vec3::new(0.0, 0.15, 0.0),
            t0: t,
            life: 0.24,
            size: 0.36,
            order: 42,
        });
        for s in [-1.0f32, 0.0, 1.0] {
            let r1 = self.rand01();
            let r2 = self.rand01();
            self.effects.push(FxItem {
                kind: SpriteKey::Dust,
                from: tip,
                vel: Vec3::new(s * 0.5 + 0.1, 0.3 + 0.15 * r2, 0.1),
                t0: t,
                life: 0.5,
                size: 0.13 + 0.04 * r1,
                order: 41,
            });
        }
        self.s_desk.kick(0.35);
        self.s_squash.kick(0.25);
    }

    fn project_screen(&self, p: Vec3) -> Vec2 {
        let ndc = self.view_proj.project_point(p);
        Vec2::new(
            (ndc.x + 1.0) * 0.5 * self.viewport_w,
            (1.0 - ndc.y) * 0.5 * self.viewport_h,
        )
    }

    pub fn hit_test(&self, x: f32, y: f32) -> HitZone {
        if x < 0.0 || x > self.viewport_w || y < 0.0 || y > self.viewport_h {
            return HitZone::None;
        }
        let ndc_x = x / self.viewport_w * 2.0 - 1.0;
        let ndc_y = -(y / self.viewport_h) * 2.0 + 1.0;
        let near_p = self.inv_view_proj.project_point(Vec3::new(ndc_x, ndc_y, 0.0));
        let far_p = self.inv_view_proj.project_point(Vec3::new(ndc_x, ndc_y, 1.0));
        let dir = (far_p - near_p).normalize();
        let orig = self.cam_pos;
        if self.cat.raycast_pet(orig, dir) {
            return HitZone::Pet;
        }
        if self.cat.raycast_drag(orig, dir)
            || self.keyboard.raycast(orig, dir)
            || self.mouse.raycast(orig, dir)
            || self.cup.raycast(orig, dir)
        {
            return HitZone::Drag;
        }
        HitZone::None
    }

    fn is_late_night(&self) -> bool {
        if !self.tuning.night_on {
            return false;
        }
        let h = self.life.clock.hour;
        let threshold = self.tuning.night_at;
        if threshold >= 24.0 {
            let early = threshold - 24.0;
            h >= early && h < 5.0
        } else {
            h >= threshold || h < 4.5
        }
    }

    fn update_cup(&mut self, t: f32, dt: f32) {
        if !self.cup.enabled {
            self.cup_anim = CupAnim::Idle;
            self.cup.pos = CUP_HOME;
            self.cup.rot = Vec3::ZERO;
            self.cup.opacity = 1.0;
            self.cup.steam_opacity_a = 0.0;
            self.cup.steam_opacity_b = 0.0;
            return;
        }
        let calm = match self.cup_anim {
            CupAnim::Idle => 1.0,
            CupAnim::Pushing { t0 } => {
                let u = clamp((t - t0) / 1.4, 0.0, 1.0);
                let slide = smooth(0.35, 1.0, u);
                self.cup.pos = CUP_HOME + Vec3::new(-0.34 * slide, 0.0, 0.04 * slide);
                self.cup.rot = Vec3::new(0.0, 0.0, 0.25 * slide);
                if u >= 1.0 {
                    self.life.add_pushes(1.0);
                    self.cup_anim = CupAnim::Falling {
                        t0: t,
                        pos: self.cup.pos,
                        vel: Vec3::new(-0.75, 0.15, 0.08),
                        roll: self.cup.rot.z,
                    };
                }
                1.0 - slide
            }
            CupAnim::Falling { t0, mut pos, mut vel, mut roll } => {
                vel.y -= 4.8 * dt;
                pos += vel * dt;
                roll += 3.2 * dt;
                self.cup.pos = pos;
                self.cup.rot = Vec3::new(0.0, 0.0, roll);
                let age = t - t0;
                self.cup.opacity = clamp(1.0 - (age - 0.15) / 0.45, 0.0, 1.0);
                if age >= 3.0 {
                    self.cup.pos = CUP_HOME;
                    self.cup.rot = Vec3::ZERO;
                    self.cup_anim = CupAnim::Respawn { t0: t };
                } else {
                    self.cup_anim = CupAnim::Falling { t0, pos, vel, roll };
                }
                0.0
            }
            CupAnim::Respawn { t0 } => {
                let u = clamp((t - t0) / 0.8, 0.0, 1.0);
                self.cup.opacity = u;
                if u >= 1.0 {
                    self.cup_anim = CupAnim::Idle;
                }
                u
            }
        };
        self.cup.update_steam(t, calm);
    }

    /// Advances the entire scene by `dt` seconds and refreshes `self.sprites` and `self.hit`.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        let t = self.time;

        // Caps Lock auto-release & popup guard (fixes sticky Caps Lock popup)
        if let Some(release_at) = self.caps_release_at {
            if t >= release_at {
                self.caps_release_at = None;
                self.on_key("CapsLock", false);
                if let Some(k_idx) = self.keyboard.resolve("CapsLock") {
                    for p in &mut self.popups {
                        if p.key == k_idx && p.up_at.is_none() {
                            p.up_at = Some(t);
                        }
                    }
                }
            }
        }

        // Sleep state (sleep_after <= 0 means never sleep)
        let idle = t - self.last_input;
        if self.sleep_after > 0.0 && !self.asleep && !self.petting && !self.nag_active && idle > self.sleep_after {
            self.asleep = true;
            self.sleep_start = Some(t);
        }
        if self.asleep {
            self.life.add_nap_sec(dt as f64);
        }
        self.sleep += ((if self.asleep { 1.0 } else { 0.0 }) - self.sleep)
            * approach(if self.asleep { 1.2 } else { 6.0 }, dt);

        if self.petting {
            self.pet_until = t + 0.25;
            self.life.add_pet_sec(dt as f64);
        }
        let pet = t < self.pet_until;

        // Purring & kneading (triggered after continuous petting >= purr_after; suppresses pet annoyance!)
        let want_purr = self.petting
            && !self.cross
            && self.pet_start >= 0.0
            && (t - self.pet_start) >= self.tuning.purr_after;
        self.purr += ((if want_purr { 1.0 } else { 0.0 }) - self.purr)
            * approach(if want_purr { 2.5 } else { 4.5 }, dt);
        if self.purr > 0.25 {
            self.life.add_bond(0.18 * dt);
            if t > self.next_heart {
                self.next_heart = t + 2.4;
                self.spawn_heart(0.2);
            }
        }

        // Pomodoro & break reminder
        if self.pomodoro {
            let limit = if self.pomo_is_break {
                self.tuning.pomo_break * 60.0
            } else {
                self.tuning.pomo_focus * 60.0
            };
            if t - self.pomo_phase_start >= limit.max(10.0) {
                self.pomo_phase_start = t;
                self.pomo_is_break = !self.pomo_is_break;
                if self.pomo_is_break {
                    self.life.add_focus(1.0);
                    self.life.add_bond(1.5);
                    self.nag_active = true;
                    let mins = self.tuning.pomo_break.round() as i32;
                    self.say(&format!("专注完成，休息 {mins} 分钟"), 3.2);
                } else {
                    self.nag_active = false;
                    let mins = self.tuning.pomo_focus.round() as i32;
                    self.say(&format!("开始下一轮专注 {mins} 分钟"), 2.8);
                }
            }
        } else if self.tuning.break_after > 0.0 && !self.nag_active && t >= self.nag_snooze_until {
            if let Some(ws) = self.work_start {
                if t - self.work_last > 180.0 {
                    self.work_start = None;
                } else if t - ws >= self.tuning.break_after * 60.0 {
                    self.nag_active = true;
                    self.life.add_breaks(1.0);
                    self.say("工作很久了，休息一下吧", 3.5);
                }
            }
        }
        // Leaving the PC idle for 3 minutes clears break nag automatically.
        if self.nag_active && !self.pomo_is_break && idle > 180.0 {
            self.nag_active = false;
            self.work_start = None;
        }

        // Late-night bedtime nudge
        let late_night = self.is_late_night();
        if late_night
            && !self.asleep
            && !self.cross
            && !(self.pomodoro && !self.pomo_is_break)
            && t - self.last_night_hint > 900.0
            && t - self.last_input < 15.0
        {
            self.last_night_hint = t;
            self.act = Some(Fidget { kind: FidgetKind::Yawn, t0: t, dur: 2.3, side: 1.0 });
            self.say("很晚了，早点休息吧", 3.2);
        }

        // Needy behaviour: using mouse only for `needy_after` minutes without typing
        if self.tuning.needy_after > 0.0
            && !self.asleep
            && !self.cross
            && !self.nag_active
            && !(self.pomodoro && !self.pomo_is_break)
            && t - self.last_mouse < 15.0
            && t - self.last_key > self.tuning.needy_after * 60.0
            && t - self.last_needy > self.tuning.needy_after * 60.0
        {
            self.last_needy = t;
            let r = self.rand01();
            if r < 0.35 {
                if let Some(sp) = self.keyboard.resolve("Space") {
                    self.tail_tap.from = self.tail_tap.reach;
                    self.tail_tap.dur = 0.16;
                    self.tail_tap.key = Some(sp);
                    self.tail_tap.t0 = t;
                    self.tail_tap.down = false;
                }
            } else {
                let r_key = self.rand01();
                if let Some(k_idx) = self.keyboard.random_letter(r_key) {
                    let top = self.keyboard.key_top(k_idx);
                    self.paw_r.from = self.paw_r.pos;
                    self.paw_r.key = Some(k_idx);
                    self.paw_r.t0 = t;
                    self.paw_r.dur = clamp(self.paw_r.pos.distance(top) * 0.09, 0.06, 0.11);
                    self.paw_r.down = false;
                    self.keyboard.set_down(k_idx, true);
                    self.keyboard.set_down(k_idx, false);
                }
            }
            self.busy_until = self.busy_until.max(t + 1.5);
        }

        // Sprawling onto the keyboard (deep sleep > sprawl_after minutes OR break nag)
        let deep_sleep_sprawl = self.asleep
            && self.tuning.sprawl_after > 0.0
            && self.sleep_start.is_some_and(|st| t - st >= self.tuning.sprawl_after * 60.0);
        let want_sprawl = deep_sleep_sprawl || self.nag_active;
        self.sprawl += ((if want_sprawl { 1.0 } else { 0.0 }) - self.sprawl)
            * approach(if want_sprawl { 1.6 } else { 4.2 }, dt);
        if self.sprawl > 0.2 {
            self.keyboard.weigh(crate::cat::HOME.x + 0.05, 0.65 * self.sprawl);
        } else {
            self.keyboard.weigh(0.0, 0.0);
        }

        self.keyboard.update(dt);
        self.mouse.update(dt, self.mouse_down);
        self.update_cup(t, dt);

        // Idle fidgets lifecycle
        if let Some(a) = self.act {
            if t > a.t0 + a.dur {
                self.act = None;
            }
        }
        let pomo_quiet = self.pomodoro && !self.pomo_is_break;
        if self.act.is_none()
            && t > self.next_act
            && idle > 2.5
            && !self.asleep
            && !pet
            && !self.cross
            && !self.nag_active
            && !pomo_quiet
        {
            let can_push = self.cup.enabled && matches!(self.cup_anim, CupAnim::Idle);
            let mut pool: Vec<(FidgetKind, f32)> = vec![
                (FidgetKind::Tilt, 1.8),
                (FidgetKind::Ear, 0.45),
                (FidgetKind::SlowBlink, 1.4),
                (FidgetKind::Look, 2.2),
                (FidgetKind::Tail, 1.0),
                (FidgetKind::Yawn, 2.2),
                (FidgetKind::Stretch, 2.4),
                (FidgetKind::Lick, 2.4),
            ];
            if late_night {
                pool.push((FidgetKind::Yawn, 2.3));
                pool.push((FidgetKind::SlowBlink, 1.6));
            }
            if can_push {
                pool.push((FidgetKind::PushCup, 1.6));
            }
            let pick = ((self.rand01() * pool.len() as f32) as usize).min(pool.len() - 1);
            let (kind, dur) = pool[pick];
            let side = if self.rand01() < 0.5 { -1.0 } else { 1.0 };
            if kind == FidgetKind::PushCup {
                self.cup_anim = CupAnim::Pushing { t0: t };
            }
            self.act = Some(Fidget { kind, t0: t, dur, side });
            self.next_act = t + 4.5 + self.rand01() * 5.5;
            self.busy_until = self.busy_until.max(t + dur + 0.8);
        }
        let (act_u, act_env) = if let Some(a) = self.act {
            let u = clamp((t - a.t0) / a.dur, 0.0, 1.0);
            (u, smooth(0.0, 0.25, u) * (1.0 - smooth(0.7, 1.0, u)))
        } else {
            (0.0, 0.0)
        };

        let is_yawn = self.act.is_some_and(|a| a.kind == FidgetKind::Yawn);
        let is_stretch = self.act.is_some_and(|a| a.kind == FidgetKind::Stretch);
        let is_lick = self.act.is_some_and(|a| a.kind == FidgetKind::Lick);
        let is_push = self.act.is_some_and(|a| a.kind == FidgetKind::PushCup);
        let stopping = self.nag_active && t < self.nag_stop_until;

        // Mouth yawn & tongue lick animation
        self.cat.yawn_open = if is_yawn {
            smooth(0.12, 0.42, act_u) * (1.0 - smooth(0.68, 0.96, act_u))
        } else {
            0.0
        };
        self.cat.lick_out = if is_lick {
            act_env * (0.5 + 0.5 * (act_u * PI * 6.0).sin().max(0.0))
        } else {
            0.0
        };

        // Update paws (supports typing, mousing, kneading, grooming, stretching, pushing cup, blocking)
        let mousing = self.mousing();
        let paw_override_l = if is_push {
            Some(self.cup.pos + Vec3::new(0.12, 0.20, 0.06))
        } else if is_lick {
            Some(self.cat.head_world().translation() + Vec3::new(-0.18, -0.24, 0.42))
        } else if is_stretch {
            Some(Vec3::new(-0.48, 0.22, 1.05))
        } else if stopping {
            Some(Vec3::new(-0.34, 0.82, 0.82))
        } else if self.purr > 0.15 {
            let k = (t * 3.8).sin();
            Some(Vec3::new(-0.34, 0.27 + 0.08 * k.max(0.0) * self.purr, 0.72 + 0.04 * k * self.purr))
        } else {
            None
        };
        let paw_override_r = if is_stretch {
            Some(Vec3::new(0.48, 0.22, 1.05))
        } else if stopping {
            Some(Vec3::new(0.36, 0.86, 0.82))
        } else if self.purr > 0.15 {
            let k = (-t * 3.8).sin();
            Some(Vec3::new(0.34, 0.27 + 0.08 * k.max(0.0) * self.purr, 0.72 + 0.04 * k * self.purr))
        } else {
            None
        };

        update_paw_state(
            true,
            &mut self.paw_l,
            &self.keyboard,
            &self.mouse,
            &self.cat,
            mousing,
            self.last_mouse,
            self.mouse_down,
            self.last_key,
            paw_override_l,
            t,
            dt,
        );
        update_paw_state(
            false,
            &mut self.paw_r,
            &self.keyboard,
            &self.mouse,
            &self.cat,
            mousing,
            self.last_mouse,
            self.mouse_down,
            self.last_key,
            paw_override_r,
            t,
            dt,
        );

        // Update tail tap (Space bar easter egg)
        self.update_tail_tap(t, dt);

        // Excitement
        while self.strokes.front().is_some_and(|&s| t - s > 2.0) {
            self.strokes.pop_front();
        }
        let busyness = clamp(self.strokes.len() as f32 / 8.0, 0.0, 1.0);
        self.excite = (self.excite * (-dt / 1.5).exp())
            .max(busyness)
            .max(if mousing { 0.6 } else { 0.0 });

        // Irritation (bond increases pet tolerance; purring suppresses pet irritation completely)
        let rate = self.strokes.len() as f32 * 0.5;
        if rate > self.tuning.type_rate {
            self.poke((rate - self.tuning.type_rate) * 0.05 * dt, false);
        }
        let (bond_lvl, _) = self.life.level();
        let pet_limit = self.tuning.pet_limit * (1.0 + 0.25 * bond_lvl.saturating_sub(1) as f32);
        if self.petting && self.purr < 0.15 && self.pet_start >= 0.0 && t - self.pet_start > pet_limit {
            self.poke(0.3 * dt, false);
        }
        if t - self.last_poke > 2.0 {
            self.annoy = (self.annoy - self.tuning.calm_rate / 100.0 * dt).max(0.0);
        }
        let cross_at = self.tuning.cross_at / 100.0;
        if !self.cross && self.annoy >= cross_at {
            self.cross = true;
            self.cross_since = t;
            self.thump_phase = 0.0;
            self.act = None;
        }
        if self.cross && self.annoy < cross_at * 0.42 && t - self.cross_since > 3.0 {
            self.cross = false;
        }
        if self.asleep {
            self.cross = false;
        }

        // Curiosity
        while self.moves.front().is_some_and(|&(mt, _)| t - mt > 0.2) {
            self.moves.pop_front();
        }
        let mouse_speed: f32 = self.moves.iter().map(|&(_, d)| d).sum::<f32>() / 0.2;
        if self.tuning.curious_on && !self.asleep && mouse_speed > self.tuning.curious_speed {
            self.curious_until = t + self.tuning.curious_hold;
        }
        let curious_now = t < self.curious_until && !self.asleep;
        let curious = clamp(self.s_curious.step(if curious_now { 1.0 } else { 0.0 }, dt), 0.0, 1.0);
        let cross = clamp(
            self.s_cross.step(if self.cross && !curious_now { 1.0 } else { 0.0 }, dt),
            0.0,
            1.0,
        );
        if self.cross || curious_now {
            self.busy_until = self.busy_until.max(t + 1.0);
        }

        // Body bend & breathing
        let striking_l = self.paw_l.key.filter(|_| self.paw_l.down || t - self.paw_l.t0 < 0.3);
        let striking_r = self.paw_r.key.filter(|_| self.paw_r.down || t - self.paw_r.t0 < 0.3);
        let striking = striking_l.map(|k| (0usize, k)).or_else(|| striking_r.map(|k| (1usize, k)));
        let mut shift = 0.0f32;
        let mut hunch = 0.0f32;
        let mut squash = 0.0f32;
        if mousing {
            shift = (self.paw_l.pos.x - crate::cat::HOME.x + 0.3) * 0.45;
            hunch = 0.1;
            squash = 0.07;
        } else if let Some((arm_idx, k_idx)) = striking {
            shift = (self.keyboard.key_top(k_idx).x
                - crate::cat::HOME.x
                - self.cat.arms[arm_idx].shoulder_local.x)
                * 0.32;
            hunch = 0.16;
        }
        if t - self.last_key < 1.2 {
            hunch = hunch.max(0.12 + 0.06 * busyness);
            squash = squash.max(0.09 + 0.04 * busyness);
        }
        hunch += 0.1 * self.sleep + 0.24 * self.sprawl;
        squash += 0.08 * self.sleep + 0.14 * self.sprawl;
        hunch += 0.06 * curious;
        squash -= 0.04 * curious;
        if is_stretch {
            hunch += 0.22 * act_env;
            squash += 0.16 * act_env;
        }
        if is_push {
            shift -= 0.26 * act_env;
        }
        shift += 0.06 * smooth(0.0, 1.0, self.tail_tap.reach);
        self.s_shift.step(shift, dt);
        self.s_hunch.step(hunch, dt);
        self.s_squash.step(squash, dt);

        let breath_rate = mix(2.1, 1.1, self.sleep);
        self.breath_phase = (self.breath_phase + dt * breath_rate / (2.0 * PI)) % 1.0;
        let breathe = (self.breath_phase * 2.0 * PI).sin() * mix(0.012, 0.02, self.sleep);
        self.cat.bend.x = self.s_shift.x;
        self.cat.bend.z = self.s_hunch.x;
        self.cat.bend.squash = self.s_squash.x;
        self.cat.body_pos = crate::cat::HOME + Vec3::new(0.0, -0.05 * self.sprawl, 0.34 * self.sprawl);
        self.cat.body_rot = Vec3::new(0.05 * self.sleep + 0.12 * self.sprawl, 0.0, -self.s_shift.x * 0.06);
        self.cat.body_scale = Vec3::new(1.0 + breathe * 0.4, 1.0 + breathe, 1.0 + breathe * 0.4);
        self.cat.place_head();

        // Gaze & head pose
        let head_screen = self.project_screen(self.cat.head_world().translation());
        let typing_key = if self.paw_l.down {
            self.paw_l.key
        } else if self.paw_r.down {
            self.paw_r.key
        } else {
            None
        };
        let focus = if is_push {
            Some(self.project_screen(self.cup.pos))
        } else if let Some(k) = typing_key.filter(|_| t - self.last_key < 0.6) {
            Some(self.project_screen(self.keyboard.key_top(k)))
        } else if self.tail_tap.key.is_some() && t - self.tail_tap.t0 < 0.6 {
            Some(self.project_screen(self.tail_tap.tip))
        } else {
            self.cursor
        };
        let (mut lx, mut ly) = if let Some(f) = focus {
            (
                clamp((f.x - head_screen.x) / 260.0, -1.0, 1.0),
                clamp((f.y - head_screen.y) / 220.0, -1.0, 1.0),
            )
        } else {
            (0.0, 0.0)
        };
        if let Some(a) = self.act.filter(|a| a.kind == FidgetKind::Look) {
            lx = mix(lx, a.side * 0.8, act_env);
            ly = mix(ly, -0.3, act_env);
        }
        if pet || self.sleep > 0.5 {
            lx *= 0.2;
            ly *= 0.2;
        }
        self.s_gaze_x.step(lx, dt);
        self.s_gaze_y.step(ly, dt);

        let shake = if t - self.wake_at < 0.6 {
            ((t - self.wake_at) * 34.0).sin() * 0.22 * (1.0 - (t - self.wake_at) / 0.6)
        } else {
            0.0
        };
        self.rub += ((if pet { 1.0 } else { 0.0 }) - self.rub) * approach(8.0, dt);
        let rub_wave = (t * 7.5).sin() * 0.09 * self.rub;
        let pet_side = clamp(self.pet_x, -1.0, 1.0);
        let huff = cross * (if lx >= 0.0 { -1.0 } else { 1.0 }) * 0.32;
        let follow = mix(0.3, 0.5, curious);
        let yaw_extra = if is_lick { -0.22 * act_env } else { 0.0 };
        self.s_yaw.step(mix(lx * follow, huff, cross) + pet_side * 0.18 * self.rub + yaw_extra, dt);
        let pitch_extra = if is_yawn {
            -0.24 * self.cat.yawn_open
        } else if is_lick {
            0.28 * act_env + 0.08 * self.cat.lick_out
        } else {
            0.0
        };
        self.s_pitch.step(
            ly * mix(0.12, 0.22, curious) * (1.0 - cross) - 0.28
                + 0.5 * self.s_hunch.x
                + if mousing { 0.06 } else { 0.0 }
                + 0.5 * self.sleep
                + 0.08 * cross
                + pitch_extra,
            dt,
        );
        let mut roll = 0.0f32;
        if striking.is_some() {
            roll = clamp(self.s_shift.x * 1.2, -0.18, 0.18);
        }
        if let Some(a) = self.act.filter(|a| a.kind == FidgetKind::Tilt) {
            roll += a.side * 0.26 * act_env;
        }
        if self.cursor.is_some()
            && striking.is_none()
            && !mousing
            && t - self.cursor_seen > 1.2
            && lx.abs() < 0.5
            && ly.abs() < 0.6
        {
            roll += 0.12 * if lx < 0.0 { -1.0 } else { 1.0 };
        }
        roll += 0.2 * curious * if lx >= 0.0 { -1.0 } else { 1.0 };
        self.s_roll.step(roll - pet_side * 0.12 * self.rub + 0.08 * self.sleep, dt);

        self.cat.head_pivot_rot = Vec3::new(
            self.s_pitch.x,
            -self.cat.body_rot.y + self.s_yaw.x + shake,
            -self.s_yaw.x * 0.2 + rub_wave,
        );
        self.cat.head_roll = self.s_roll.x - self.cat.body_rot.z;
        self.cat.head_pivot_pos.x += rub_wave * 0.4;
        self.cat.place_collar();

        // Ears
        self.s_ears.step(
            0.1 * self.excite * (1.0 - cross) + 0.12 * curious - 0.22 * self.rub - 0.55 * cross - 0.15 * self.cat.yawn_open,
            dt,
        );
        for i in 0..2 {
            let e = self.cat.ears[i];
            let flick = if let Some(a) = self.act.filter(|a| a.kind == FidgetKind::Ear && e.side == a.side) {
                let _ = a;
                (act_u * PI * 3.0).sin() * 0.3 * (1.0 - act_u)
            } else {
                0.0
            };
            self.cat.turn_ear(
                i,
                -e.side * (e.tilt - self.s_ears.x) + flick,
                -0.12 - 0.35 * cross + 0.12 * curious,
            );
        }

        // Eyes
        if t > self.next_blink + 0.15 {
            self.next_blink = t + 2.5 + self.rand01() * 4.0;
        }
        if curious_now && t > self.next_blink - 0.2 {
            self.next_blink = t + 0.5;
        }
        let mut blink = if t > self.next_blink {
            (clamp((t - self.next_blink) / 0.15, 0.0, 1.0) * PI).sin()
        } else {
            0.0
        };
        if self.act.is_some_and(|a| a.kind == FidgetKind::SlowBlink) {
            blink = blink.max(0.85 * (clamp(act_u, 0.0, 1.0) * PI).sin());
        }
        let closed = (pet && !self.cross) || self.sleep > 0.5 || self.cat.yawn_open > 0.35;
        let drowsy = if late_night && !self.cross && !curious_now { 0.24 } else { 0.0 };
        let dilate = (mix(1.0 + 0.25 * self.excite, self.tuning.pupil_size / 100.0, curious)
            * (1.0 - 0.5 * cross))
            .min(2.6);
        for i in 0..2 {
            let eye_world_pos = self.cat.eye_world(i).translation();
            let (px, py) = if let Some(f) = focus {
                let es = self.project_screen(eye_world_pos);
                (
                    clamp((f.x - es.x) / 170.0, -1.0, 1.0),
                    clamp((f.y - es.y) / 170.0, -1.0, 1.0),
                )
            } else {
                (self.s_gaze_x.x, self.s_gaze_y.x)
            };
            let e = &mut self.cat.eyes[i];
            e.closed = closed;
            e.scale_y = (1.0 - 0.9 * blink) * (1.0 - 0.42 * cross) * (1.0 - drowsy) * (1.0 + 0.1 * curious);
            e.scale_x = 1.0 + 0.06 * curious;
            e.pupil_scale.x += (0.044 * dilate - e.pupil_scale.x) * approach(10.0, dt);
            e.pupil_scale.y += (0.11 - 0.012 * (dilate - 1.5).max(0.0) - e.pupil_scale.y) * approach(10.0, dt);
            let room = (0.155 - e.pupil_scale.x - 0.012).max(0.0);
            e.pupil_pos.x += (clamp(px * 0.07 - e.side * 0.012, -room, room) - e.pupil_pos.x) * approach(22.0, dt);
            e.pupil_pos.x = clamp(e.pupil_pos.x, -room, room);
            e.pupil_pos.y += (-py * 0.02 - e.pupil_pos.y) * approach(22.0, dt);
        }
        self.cat.blush_opacity = (0.55 * self.rub)
            .max(0.45 * self.purr)
            .max(if self.cat.resolved.blush_always { 0.4 } else { 0.0 })
            * (1.0 - cross);

        // Tail & thump (fixed-length chain with Verlet inertia + winter curl)
        self.tail_phase += dt * (1.0 + 1.4 * self.excite + 2.5 * self.annoy + 3.0 * curious);
        let flick_tail = if self.act.is_some_and(|a| a.kind == FidgetKind::Tail) {
            (act_u * PI).sin() * 2.0
        } else {
            0.0
        };
        let (mut lift, mut lash) = (0.0f32, 0.0f32);
        let mut do_slam = false;
        if cross > 0.05 {
            let prev = self.thump_phase;
            self.thump_phase += dt * self.tuning.thump_rate;
            let beat = self.thump_phase.floor();
            let u = self.thump_phase - beat;
            let resting = (beat as i32) % 4 == 3;
            if !resting {
                lift = if u < 0.6 {
                    smooth(0.0, 0.6, u)
                } else if u < 0.72 {
                    1.0 - ((u - 0.6) / 0.12).powi(2)
                } else {
                    0.0
                };
            }
            lash = (self.thump_phase * PI * 0.5).sin() * 0.6;
            let slam_at = beat + 0.72;
            if !resting && self.cross && prev < slam_at && self.thump_phase >= slam_at {
                do_slam = true;
            }
        }
        self.thump_lift += (lift - self.thump_lift) * approach(40.0, dt);
        if let Some(k_idx) = self.tail_tap.key {
            self.tail_tap.tip = self.keyboard.key_top(k_idx)
                + Vec3::new(0.36, 0.075 + self.tail_tap.hover, -0.02);
        }
        let reach = smooth(0.0, 1.0, self.tail_tap.reach);
        let winter_wrap = if self.life.clock.is_winter() && idle > 8.0 && !self.cross && !curious_now {
            0.75
        } else {
            0.0
        };
        let sway = (1.0 - self.sleep)
            * (1.0 + 0.6 * self.excite + flick_tail + 1.5 * self.annoy)
            * (1.0 - cross)
            * (1.0 - 0.55 * curious)
            * (1.0 - reach);
        self.cat.update_tail(
            self.tail_phase,
            sway,
            self.sleep,
            cross * (1.0 - reach),
            self.thump_lift,
            lash,
            reach,
            Some(self.tail_tap.tip),
            winter_wrap,
            dt,
        );
        if do_slam {
            self.slam();
        }

        // Desk hop & arms
        self.s_desk.step(0.0, dt);
        self.keyboard.pos.y = 0.02 + self.s_desk.x;
        self.mouse.pos.y += self.s_desk.x * 1.4;
        self.cat.update_arm(0, self.paw_l.pos, self.sprawl);
        self.cat.update_arm(1, self.paw_r.pos, self.sprawl);

        // Sprites (anger mark, slam effects, hearts, key popups, sleep Zs, speech bubble)
        self.sprites.clear();
        if cross > 0.1 {
            let pos = self.cat.head_pivot_world().transform_point(Vec3::new(-0.42, 0.85, 0.2));
            let pulse = 1.0 + 0.25 * (1.0 - (t - self.last_slam) / 0.25).max(0.0);
            let s = 0.27 * cross * pulse;
            self.sprites.push(SpriteDraw {
                key: SpriteKey::Anger,
                pos,
                scale_x: s,
                scale_y: s,
                opacity: cross,
                render_order: 45,
            });
        }
        self.effects.retain(|fx| t - fx.t0 < fx.life);
        for fx in &self.effects {
            let age = t - fx.t0;
            let u = age / fx.life;
            let s = fx.size * if fx.kind == SpriteKey::Bang {
                0.5 + 0.7 * smooth(0.0, 0.3, u)
            } else if fx.kind == SpriteKey::Heart {
                0.75 + 0.4 * smooth(0.0, 0.35, u)
            } else {
                0.6 + 0.9 * u
            };
            let opacity = if fx.kind == SpriteKey::Bang {
                1.0 - smooth(0.5, 1.0, u)
            } else if fx.kind == SpriteKey::Heart {
                smooth(0.0, 0.18, u) * (1.0 - smooth(0.55, 1.0, u))
            } else {
                0.9 * (1.0 - u)
            };
            self.sprites.push(SpriteDraw {
                key: fx.kind.clone(),
                pos: fx.from + fx.vel * age,
                scale_x: s,
                scale_y: s,
                opacity,
                render_order: fx.order,
            });
        }

        // Speech bubble (break reminder / pomodoro / bedtime)
        if let Some((ref text, t0, dur)) = self.bubble.clone() {
            let age = t - t0;
            if age > dur {
                self.bubble = None;
            } else {
                let fade = clamp((age / 0.2).min((dur - age) / 0.35), 0.0, 1.0);
                let pos = self.cat.head_world().translation() + Vec3::new(0.05, 0.72, 0.15);
                let chars = text.chars().count().max(4) as f32;
                let h = 0.28;
                let w = clamp(h * (chars * 0.62 + 1.1), 0.85, 2.25);
                self.sprites.push(SpriteDraw {
                    key: SpriteKey::Bubble(text.clone()),
                    pos,
                    scale_x: w,
                    scale_y: h,
                    opacity: fade,
                    render_order: 48,
                });
            }
        }

        let keys = &self.keyboard.keys;
        let mut keep_popups = Vec::with_capacity(self.popups.len());
        for mut p in self.popups.drain(..) {
            let age = t - p.t0;
            if !keys[p.key].down && p.up_at.is_none() {
                p.up_at = Some(t);
            }
            let fade = match p.up_at {
                None => 1.0,
                Some(up) => clamp(1.0 - (t - up - 0.35) / 0.3, 0.0, 1.0),
            };
            if fade > 0.0 {
                let pop = 0.6 + 0.4 * (age / 0.12).min(1.0);
                let h = 0.27 * pop;
                self.sprites.push(SpriteDraw {
                    key: SpriteKey::Cap(p.label.clone()),
                    pos: p.base + Vec3::new(0.0, 0.22 + 0.14 * smooth(0.0, 0.25, age), 0.0),
                    scale_x: h * p.aspect,
                    scale_y: h,
                    opacity: fade,
                    render_order: 40,
                });
                keep_popups.push(p);
            }
        }
        self.popups = keep_popups;

        if self.sleep > 0.7 && t > self.next_z {
            self.next_z = t + 1.3;
            let base = self.cat.head_world().translation() + Vec3::new(0.45, 0.55, 0.0);
            self.zs.push(ZItem { base, t0: t });
        }
        self.zs.retain(|z| t - z.t0 <= 2.6);
        for z in &self.zs {
            let age = t - z.t0;
            let s = 0.16 + age * 0.08;
            let opacity = clamp((age / 0.3).min((2.6 - age) / 0.6), 0.0, 1.0);
            self.sprites.push(SpriteDraw {
                key: SpriteKey::SleepZ,
                pos: z.base + Vec3::new((age * 2.2).sin() * 0.08 + age * 0.12, age * 0.32, 0.0),
                scale_x: s,
                scale_y: s,
                opacity,
                render_order: 40,
            });
        }
        self.sprites.sort_by_key(|s| s.render_order);

        // Steady sleep Zs stay at idle/sleep cadence rather than forcing full FPS.
        if pet
            || self.purr > 0.02
            || self.bubble.is_some()
            || !matches!(self.cup_anim, CupAnim::Idle)
            || !self.popups.is_empty()
            || !self.effects.is_empty()
            || cross > 0.02
            || (self.sleep > 0.02 && self.sleep < 0.98)
            || (self.sprawl > 0.02 && self.sprawl < 0.98)
            || t - self.wake_at < 0.7
        {
            self.busy_until = self.busy_until.max(t + 0.5);
        }

        // Hit test with viewport early-exit & idle dedup
        if let Some(c) = self.cursor {
            if c.x >= 0.0 && c.x <= self.viewport_w && c.y >= 0.0 && c.y <= self.viewport_h {
                if self.cursor_dirty || t < self.busy_until {
                    self.hit = self.hit_test(c.x, c.y);
                    self.cursor_dirty = false;
                }
            } else {
                self.hit = HitZone::None;
                self.cursor_dirty = false;
            }
        }
    }

    fn update_tail_tap(&mut self, t: f32, dt: f32) {
        let Some(k_idx) = self.tail_tap.key else {
            self.tail_tap.reach += (0.0 - self.tail_tap.reach) * approach(10.0, dt);
            return;
        };
        let age = t - self.tail_tap.t0;
        let swinging = age < self.tail_tap.dur;
        if swinging {
            self.tail_tap.reach = mix(self.tail_tap.from, 1.0, smooth(0.0, 1.0, age / self.tail_tap.dur));
        } else if self.tail_tap.land_at < self.tail_tap.t0 {
            self.tail_tap.land_at = t;
            self.s_squash.kick(0.15);
            self.s_pitch.kick(0.4);
            if !self.privacy {
                let label = self.keyboard.keys[k_idx].label.to_string();
                self.spawn_popup(k_idx, 1.0, label);
            }
        }
        let pressing = !swinging && (self.tail_tap.down || t - self.tail_tap.land_at < 0.09);
        self.keyboard.set_down(k_idx, pressing);
        let hover = if swinging {
            0.12 + (age / self.tail_tap.dur * PI).sin() * 0.1 * (1.0 - self.tail_tap.from)
        } else if pressing {
            0.0
        } else {
            0.12
        };
        let rate = if swinging { 60.0 } else if pressing { 70.0 } else { 18.0 };
        self.tail_tap.hover += (hover - self.tail_tap.hover) * approach(rate, dt);
        if !pressing && !swinging && t - self.tail_tap.land_at > 0.8 {
            self.tail_tap.key = None;
            return;
        }
        self.busy_until = self.busy_until.max(t + 1.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_paw_state(
    is_left: bool,
    paw: &mut PawState,
    keyboard: &Keyboard,
    mouse: &MouseProp,
    cat: &Cat,
    mousing: bool,
    last_mouse: f32,
    mouse_down: bool,
    last_key: f32,
    override_target: Option<Vec3>,
    t: f32,
    dt: f32,
) {
    let strike_age = t - paw.t0;
    if let Some(k_idx) = paw.key {
        if paw.down || strike_age < paw.dur + 0.06 {
            let top = keyboard.key_top(k_idx);
            let hover = top + Vec3::new(0.0, 0.12, 0.0);
            let press = top + Vec3::new(0.0, 0.07, 0.0);
            if strike_age < paw.dur {
                let u = smooth(0.0, 1.0, strike_age / paw.dur);
                paw.pos = paw.from.lerp(hover, u);
                paw.pos.y += (u.sqrt() * PI).sin() * clamp(paw.from.distance(hover) * 0.3, 0.04, 0.2);
            } else if paw.down {
                paw.pos = hover.lerp(press, clamp((strike_age - paw.dur) / 0.035, 0.0, 1.0));
            } else {
                paw.pos = paw.pos.lerp(hover, approach(30.0, dt));
            }
            paw.spring.x = paw.pos;
            if dt > 0.0 {
                paw.spring.v = (paw.pos - paw.last) * (0.5 / dt);
            }
            paw.last = paw.pos;
            return;
        }
    }
    let (target, hz, damping) = if let Some(ov) = override_target {
        (ov, 4.2, 0.82)
    } else if is_left && mousing {
        (
            mouse.top(),
            if t - last_mouse < 0.05 || mouse_down { 9.0 } else { 6.0 },
            0.9,
        )
    } else if let Some(k_idx) = paw.key.filter(|_| t - paw.t0 < 0.45) {
        (keyboard.key_top(k_idx) + Vec3::new(0.0, 0.13, 0.0), 5.0, 0.78)
    } else if t - last_key < 1.2 {
        (cat.attach(Vec3::new(paw.side * 0.55, 0.6, 0.44)), 3.6, 0.78)
    } else {
        let arm = &cat.arms[if paw.side < 0.0 { 0 } else { 1 }];
        let sh = cat.attach(arm.shoulder_local);
        (Vec3::new(sh.x + paw.side * 0.02, 0.26, 0.68), 2.4, 0.78)
    };
    paw.spring.hz = hz;
    paw.spring.damping = damping;
    paw.pos = paw.spring.step(target, dt);
    paw.last = paw.pos;
}
