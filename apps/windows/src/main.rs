//! KeyCat Native (Windows) — zero-WebView2 desktop cat rendered with Direct3D 11 (4x MSAA),
//! DirectComposition (`WS_EX_NOREDIRECTIONBITMAP`) and Direct2D/DirectWrite.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg_attr(not(windows), allow(dead_code))]
mod audio;
mod autostart;
#[cfg_attr(not(windows), allow(dead_code))]
mod display;
mod elevate;
#[cfg_attr(not(windows), allow(dead_code))]
mod hook;
#[cfg_attr(not(windows), allow(dead_code))]
mod keymap;
#[cfg_attr(not(windows), allow(dead_code))]
mod renderer;
#[cfg_attr(not(windows), allow(dead_code))]
mod settings_win;
mod store;
#[cfg_attr(not(windows), allow(dead_code))]
mod textures;
#[cfg_attr(not(windows), allow(dead_code))]
mod tray;

#[cfg(not(windows))]
fn main() {
    eprintln!("keycat-native is Windows-only");
}

#[cfg(windows)]
fn main() {
    win_app::run();
}

#[cfg(windows)]
mod win_app {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Receiver};
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use keycat_core::behavior::{HitZone, Scene, CROP_H, CROP_W};
    use keycat_core::coats::{Look, COATS};
    use keycat_core::tuning::{Tuning, TUNING_SCHEMA};
    use serde_json::{json, Value};
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT,
        RECT, WPARAM,
    };
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFO,
        MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::SystemInformation::GetLocalTime;
    use windows::Win32::System::Threading::{CreateMutexW, INFINITE};
    use windows::Win32::UI::HiDpi::{
        GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetCursorPos,
        GetWindowLongW, GetWindowRect, LoadCursorW, MsgWaitForMultipleObjectsEx, PeekMessageW,
        PostMessageW, PostQuitMessage, RegisterClassW, SetCursor, SetWindowLongW, SetWindowPos,
        ShowWindow, TranslateMessage, GWL_EXSTYLE, HWND_TOPMOST, IDC_ARROW, IDC_HAND, MSG,
        MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT, SWP_NOACTIVATE, SWP_NOSIZE, SW_HIDE,
        SW_SHOWNOACTIVATE, WM_APP, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_LBUTTONUP,
        WM_RBUTTONUP, WM_SETCURSOR, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    use crate::audio::PurrAudio;
    use crate::autostart::{self, Mode};
    use crate::display;
    use crate::elevate;
    use crate::hook::{self, Button, Input};
    use crate::renderer::Renderer;
    use crate::settings_win::{
        self, WM_SETTINGS_CHANGED, WM_SETTINGS_PRESET, WM_SETTINGS_RESET, WM_SETTINGS_TEST_ANNOY,
        WM_SETTINGS_TEST_CURIOUS,
    };
    use crate::store::Store;
    use crate::tray::{
        self, FpsMode, MenuState, TrayIcon, CMD_COAT_BASE, CMD_COLLAR_BASE, CMD_EYE_BASE,
        CMD_FPS_30, CMD_FPS_60, CMD_FPS_AUTO, CMD_FPS_HIGH, CMD_FPS_UNLIM, CMD_GLOVE_BASE,
        CMD_HOME, CMD_KB_COLOR_BASE, CMD_OPACITY_BASE, CMD_OPEN_SETTINGS, CMD_PATTERN_SOLID,
        CMD_PATTERN_TABBY, CMD_QUIT, CMD_SCALE_BASE, CMD_SHINE_BASE, CMD_SLEEP_BASE,
        CMD_TOGGLE_3D, CMD_TOGGLE_ADMIN, CMD_TOGGLE_AUTOSTART, CMD_TOGGLE_BLUSH,
        CMD_TOGGLE_CABLE, CMD_TOGGLE_CLICK_THROUGH, CMD_TOGGLE_CUP, CMD_TOGGLE_FESTIVAL,
        CMD_TOGGLE_POMODORO, CMD_TOGGLE_PRIVACY, CMD_TOGGLE_SOUND, CMD_TOGGLE_WHISKERS,
        CMD_VISIBILITY, COLLAR_OPTIONS, EYE_OPTIONS, GLOVE_OPTIONS, KB_COLOR_OPTIONS,
        OPACITY_OPTIONS, SCALE_OPTIONS, SHINE_OPTIONS, SLEEP_OPTIONS, WM_TRAY,
    };

    const IDENTIFIER: &str = "com.ellobok.keycat";
    const WM_APP_INPUT: u32 = WM_APP + 2;
    const WM_APP_SHOW: u32 = WM_APP + 3;
    const WM_APP_MENU: u32 = WM_APP + 4;

    struct Drag {
        cursor: (i32, i32),
        origin: (i32, i32),
        last: (i32, i32),
    }

    struct App {
        hwnd: HWND,
        store: Store,
        scene: Scene,
        renderer: Renderer,
        audio: PurrAudio,
        _tray: TrayIcon,
        visible: bool,
        click_through: bool,
        run_as_admin: bool,
        autostart: Mode,
        scale: f64,
        fps: FpsMode,
        display_hz: u32,
        hook_ready: bool,
        held: HashMap<u32, (u32, Option<&'static str>)>,
        last_key_audit: Instant,
        last_clock_sync: Instant,
        last_pet_click: Option<Instant>,
        petting: bool,
        drag: Option<Drag>,
        last_pointer: Option<(i32, i32)>,
        last_cursor: (i32, i32),
        pending_delta: (f32, f32),
        click_ignored: bool,
        save_deadline: Option<Instant>,
        quit: bool,
    }

    thread_local! {
        static APP: RefCell<Option<App>> = const { RefCell::new(None) };
        static MENU_OPEN: Cell<bool> = const { Cell::new(false) };
    }

    struct OwnedMenuState {
        pomodoro: bool,
        style_3d: bool,
        coat: String,
        pattern: String,
        eye: String,
        shine: String,
        gloves: String,
        collar: String,
        blush: bool,
        whiskers: bool,
        festival: bool,
        scale: f64,
        keyboard_color: String,
        mouse_cable: bool,
        cup: bool,
        opacity: f64,
        fps: FpsMode,
        high_fps: Option<u32>,
        sleep_after: f64,
        sound: bool,
        privacy: bool,
        click_through: bool,
        autostart: Mode,
        run_as_admin: bool,
        visible: bool,
        hook_ready: bool,
    }

    impl OwnedMenuState {
        fn as_menu_state(&self) -> MenuState<'_> {
            MenuState {
                pomodoro: self.pomodoro,
                style_3d: self.style_3d,
                coat: &self.coat,
                pattern: &self.pattern,
                eye: &self.eye,
                shine: &self.shine,
                gloves: &self.gloves,
                collar: &self.collar,
                blush: self.blush,
                whiskers: self.whiskers,
                festival: self.festival,
                scale: self.scale,
                keyboard_color: &self.keyboard_color,
                mouse_cable: self.mouse_cable,
                cup: self.cup,
                opacity: self.opacity,
                fps: self.fps,
                high_fps: self.high_fps,
                sleep_after: self.sleep_after,
                sound: self.sound,
                privacy: self.privacy,
                click_through: self.click_through,
                autostart: self.autostart,
                run_as_admin: self.run_as_admin,
                visible: self.visible,
                hook_ready: self.hook_ready,
            }
        }
    }

    fn settings_path() -> PathBuf {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join(IDENTIFIER).join("settings.json")
    }

    fn prepare_launch() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if let Some(pid) = args
            .iter()
            .position(|arg| arg == "--after")
            .and_then(|i| args.get(i + 1)?.parse().ok())
        {
            elevate::wait_for_exit(pid);
            return true;
        }
        if cfg!(debug_assertions) || elevate::is_elevated() {
            return true;
        }
        let store = Store::open(settings_path());
        store.get("runAsAdmin") != Some(&Value::Bool(true))
            || (!autostart::run_task() && elevate::relaunch("").is_err())
    }

    fn read_scale(store: &Store) -> f64 {
        store
            .get("catScale")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.3)
            .unwrap_or(0.9)
    }

    fn read_fps(store: &Store) -> FpsMode {
        match store.get("fps") {
            Some(Value::String(s)) if s == "auto" => FpsMode::Auto,
            Some(v) if v.as_u64() == Some(0) => FpsMode::Unlimited,
            Some(v) if v.as_u64().is_some_and(|n| (15..=1000).contains(&n)) => {
                FpsMode::Fixed(v.as_u64().unwrap() as u32)
            }
            _ => FpsMode::Auto,
        }
    }

    fn read_look(store: &Store) -> Look {
        let text = |key: &str, fb: &str| {
            store.get(key).and_then(Value::as_str).unwrap_or(fb).to_string()
        };
        let coat = text("coat", "black");
        let opacity = store
            .get("opacity")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .map(|v| (v as f32).clamp(0.3, 1.0))
            .unwrap_or(1.0);
        Look {
            style_3d: text("style", "2d") == "3d",
            pattern: text(&format!("pattern.{coat}"), "auto"),
            eye: text("eyeColor", "auto"),
            shine: text("eyeShine", "classic"),
            collar: text("collar", "none"),
            blush: store.get("blush") == Some(&Value::Bool(true)),
            whiskers: store.get("whiskers") != Some(&Value::Bool(false)),
            gloves: text("gloves", "auto"),
            keyboard_color: text("keyboardColor", "cream"),
            mouse_cable: store.get("mouseCable") != Some(&Value::Bool(false)),
            opacity,
            cup: store.get("cup") != Some(&Value::Bool(false)),
            festival: store.get("festival") != Some(&Value::Bool(false)),
            sound: store.get("sound") == Some(&Value::Bool(true)),
            coat,
        }
    }

    fn sync_local_clock(scene: &mut Scene) {
        let st = unsafe { GetLocalTime() };
        let hour = st.wHour as f32 + st.wMinute as f32 / 60.0;
        scene.set_local_clock(st.wYear as i32, st.wMonth as u32, st.wDay as u32, hour);
    }

    fn read_tuning(store: &Store) -> Tuning {
        store
            .get("tune")
            .and_then(Value::as_object)
            .map(Tuning::from_json)
            .unwrap_or_default()
    }

    fn primary_work_area() -> RECT {
        unsafe {
            let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(mon, &mut info).as_bool() {
                info.rcWork
            } else {
                RECT { left: 0, top: 0, right: 1920, bottom: 1040 }
            }
        }
    }

    fn point_on_any_monitor(x: i32, y: i32) -> bool {
        struct Probe {
            x: i32,
            y: i32,
            hit: bool,
        }
        unsafe extern "system" fn cb(
            _mon: HMONITOR,
            _hdc: HDC,
            rc: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let p = &mut *(lparam.0 as *mut Probe);
            let r = *rc;
            if p.x < r.right && r.left < p.x + 80 && p.y < r.bottom && r.top < p.y + 80 {
                p.hit = true;
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut probe = Probe { x, y, hit: false };
        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(cb),
                LPARAM(&mut probe as *mut Probe as isize),
            );
        }
        probe.hit
    }

    fn dpi_scale(hwnd: HWND) -> f64 {
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi >= 96 { dpi as f64 / 96.0 } else { 1.0 }
    }

    fn window_phys_size(scale: f64, factor: f64) -> (i32, i32) {
        let lw = (f64::from(CROP_W) * scale).round();
        let lh = (f64::from(CROP_H) * scale).round();
        ((lw * factor).round() as i32, (lh * factor).round() as i32)
    }

    fn default_origin(w: i32, h: i32, factor: f64) -> (i32, i32) {
        let area = primary_work_area();
        (area.right - w - (20.0 * factor).round() as i32, area.bottom - h)
    }

    fn saved_origin(store: &Store) -> Option<(i32, i32)> {
        let x = store.get("originX")?.as_f64()? as i32;
        let y = store.get("originY")?.as_f64()? as i32;
        point_on_any_monitor(x, y).then_some((x, y))
    }

    impl App {
        fn apply_scene_options(&mut self) {
            self.scene.set_look(read_look(&self.store));
            self.scene.tuning = read_tuning(&self.store);
            self.scene.privacy = self.store.get("privacy") == Some(&Value::Bool(true));
            let sleep = self
                .store
                .get("sleepAfter")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
                .unwrap_or(60.0);
            self.scene.sleep_after = if sleep > 0.0 { sleep as f32 } else { 0.0 };
            self.click_through = self.store.get("clickThrough") == Some(&Value::Bool(true));
            self.run_as_admin = self.store.get("runAsAdmin") == Some(&Value::Bool(true));
            self.fps = read_fps(&self.store);
            self.update_click_target();
        }

        fn apply_scale(&mut self, value: f64, keep_origin: bool) {
            self.scale = value.clamp(0.4, 2.0);
            self.store.set("catScale", json!(self.scale));
            let factor = dpi_scale(self.hwnd);
            let (pw, ph) = window_phys_size(self.scale, factor);
            let (x, y) = if keep_origin {
                let mut rc = RECT::default();
                unsafe { let _ = GetWindowRect(self.hwnd, &mut rc); }
                (rc.left, rc.top)
            } else {
                saved_origin(&self.store).unwrap_or_else(|| default_origin(pw, ph, factor))
            };
            unsafe {
                let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, pw, ph, SWP_NOACTIVATE);
            }
            self.scene.set_viewport((pw as f64 / factor) as f32, (ph as f64 / factor) as f32);
            let _ = self.renderer.resize(pw as u32, ph as u32);
        }

        fn set_visible(&mut self, vis: bool) {
            if self.visible == vis {
                return;
            }
            self.visible = vis;
            if vis {
                unsafe { let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); }
                self.update_click_target();
            } else {
                self.petting = false;
                self.drag = None;
                self.pending_delta = (0.0, 0.0);
                for (_, code) in self.held.drain().map(|(_, v)| v) {
                    if let Some(code) = code {
                        self.scene.on_key(code, false);
                    }
                }
                self.scene.hit = HitZone::None;
                self.update_click_target();
                self.audio.sync(false, 0.0, 0.0);
                unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
                self.renderer.release_targets_and_trim();
            }
        }

        fn update_click_target(&mut self) {
            let ignore = !self.visible
                || (self.drag.is_none()
                    && (self.click_through || self.scene.hit == HitZone::None));
            if ignore == self.click_ignored {
                return;
            }
            self.click_ignored = ignore;
            unsafe {
                let mut ex = GetWindowLongW(self.hwnd, GWL_EXSTYLE) as u32;
                if ignore {
                    ex |= WS_EX_TRANSPARENT.0;
                } else {
                    ex &= !WS_EX_TRANSPARENT.0;
                }
                SetWindowLongW(self.hwnd, GWL_EXSTYLE, ex as i32);
            }
        }

        fn pointer_in_panel(&self) -> Option<(i32, i32)> {
            if !self.visible {
                return None;
            }
            let mut pt = POINT::default();
            let mut rc = RECT::default();
            unsafe {
                GetCursorPos(&mut pt).ok()?;
                GetWindowRect(self.hwnd, &mut rc).ok()?;
            }
            if pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }

        fn sync_pointer(&mut self) {
            if !self.visible {
                return;
            }
            let mut pt = POINT::default();
            if unsafe { GetCursorPos(&mut pt) }.is_err() {
                return;
            }
            let origin = if let Some(ref mut drag) = self.drag {
                let nx = drag.origin.0 + (pt.x - drag.cursor.0);
                let ny = drag.origin.1 + (pt.y - drag.cursor.1);
                if (nx, ny) != drag.last {
                    drag.last = (nx, ny);
                    unsafe {
                        let _ = SetWindowPos(
                            self.hwnd,
                            Some(HWND_TOPMOST),
                            nx,
                            ny,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                drag.last
            } else {
                let mut rc = RECT::default();
                if unsafe { GetWindowRect(self.hwnd, &mut rc) }.is_err() {
                    return;
                }
                (rc.left, rc.top)
            };

            let factor = dpi_scale(self.hwnd);
            if let Some(last) = self.last_pointer.replace((pt.x, pt.y)) {
                self.pending_delta.0 += ((pt.x - last.0) as f64 / factor) as f32;
                self.pending_delta.1 += ((pt.y - last.1) as f64 / factor) as f32;
            }
            let cx = ((pt.x - origin.0) as f64 / factor).round() as i32;
            let cy = ((pt.y - origin.1) as f64 / factor).round() as i32;
            if (cx, cy) != self.last_cursor {
                self.last_cursor = (cx, cy);
                self.scene.on_cursor(cx as f32, cy as f32);
            }
            if self.pending_delta.0.abs() > 0.01 || self.pending_delta.1.abs() > 0.01 {
                self.scene.on_mouse_move(self.pending_delta.0, self.pending_delta.1);
                self.pending_delta = (0.0, 0.0);
            }
        }

        fn reconcile_held_keys(&mut self, now: Instant) {
            if self.held.is_empty() || now.saturating_duration_since(self.last_key_audit) < Duration::from_millis(200) {
                return;
            }
            self.last_key_audit = now;
            let mut released = Vec::new();
            self.held.retain(|_, &mut (vk, code)| {
                if vk > 0 && vk < 256 && !hook::is_vk_down(vk) {
                    if let Some(c) = code {
                        released.push(c);
                    }
                    false
                } else {
                    true
                }
            });
            for code in released {
                self.scene.on_key(code, false);
            }
        }

        fn on_input(&mut self, input: Input) {
            match input {
                Input::Key { id, vk, code, down } => {
                    if down {
                        if self.held.contains_key(&id) {
                            return;
                        }
                        self.held.insert(id, (vk, code));
                    } else {
                        self.held.remove(&id);
                    }
                    if self.visible {
                        if let Some(code) = code {
                            self.scene.on_key(code, down);
                        }
                    }
                }
                Input::Button { button, down } => self.on_button(button, down),
                Input::Wheel(notches) => {
                    if self.visible {
                        self.pending_delta.1 += (-notches * 0.4) as f32;
                    }
                }
                Input::PointerWake => {}
            }
        }

        fn on_button(&mut self, button: Button, down: bool) {
            if !self.visible {
                return;
            }
            if down {
                if self.scene.hit != HitZone::None && !self.click_through {
                    if let Some((px, py)) = self.pointer_in_panel() {
                        match button {
                            Button::Right => {
                                // Right-click opens menu on release so button-up doesn't dismiss it.
                                return;
                            }
                            Button::Left if self.scene.hit == HitZone::Pet => {
                                if !self.petting {
                                    let now = Instant::now();
                                    let is_double = self
                                        .last_pet_click
                                        .is_some_and(|prev| now.saturating_duration_since(prev) <= Duration::from_millis(420));
                                    self.last_pet_click = Some(now);
                                    if is_double {
                                        self.last_pet_click = None;
                                        self.scene.on_head_double_click();
                                    }
                                    self.petting = true;
                                    let mut rc = RECT::default();
                                    unsafe { let _ = GetWindowRect(self.hwnd, &mut rc); }
                                    let factor = dpi_scale(self.hwnd);
                                    let lx = ((px - rc.left) as f64 / factor) as f32;
                                    self.scene.on_pet(true, lx);
                                }
                            }
                            Button::Left => {
                                if self.drag.is_none() {
                                    let mut rc = RECT::default();
                                    unsafe { let _ = GetWindowRect(self.hwnd, &mut rc); }
                                    self.drag = Some(Drag {
                                        cursor: (px, py),
                                        origin: (rc.left, rc.top),
                                        last: (rc.left, rc.top),
                                    });
                                    self.update_click_target();
                                }
                            }
                            Button::Other => {}
                        }
                    }
                }
            } else {
                if button == Button::Right
                    && self.scene.hit != HitZone::None
                    && !self.click_through
                    && self.pointer_in_panel().is_some()
                {
                    unsafe {
                        let _ = PostMessageW(Some(self.hwnd), WM_APP_MENU, WPARAM(0), LPARAM(0));
                    }
                    return;
                }
                if self.petting {
                    self.petting = false;
                    self.scene.on_pet(false, 0.0);
                }
                if let Some(drag) = self.drag.take() {
                    self.store.set("originX", json!(drag.last.0));
                    self.store.set("originY", json!(drag.last.1));
                    self.update_click_target();
                }
            }
            self.scene.on_mouse_button(down);
        }

        fn snapshot_menu(&mut self) -> OwnedMenuState {
            self.display_hz = display::max_refresh_hz();
            let look = read_look(&self.store);
            let sleep_after = self
                .store
                .get("sleepAfter")
                .and_then(Value::as_f64)
                .unwrap_or(60.0);
            OwnedMenuState {
                pomodoro: self.scene.pomodoro,
                style_3d: look.style_3d,
                coat: look.coat,
                pattern: look.pattern,
                eye: look.eye,
                shine: look.shine,
                gloves: look.gloves,
                collar: look.collar,
                blush: look.blush,
                whiskers: look.whiskers,
                festival: look.festival,
                scale: self.scale,
                keyboard_color: look.keyboard_color,
                mouse_cable: look.mouse_cable,
                cup: look.cup,
                opacity: f64::from(look.opacity),
                fps: self.fps,
                high_fps: (self.display_hz > 60).then_some(self.display_hz),
                sleep_after,
                sound: look.sound,
                privacy: self.scene.privacy,
                click_through: self.click_through,
                autostart: self.autostart,
                run_as_admin: self.run_as_admin,
                visible: self.visible,
                hook_ready: self.hook_ready,
            }
        }

        fn handle_menu_cmd(&mut self, cmd: u32, high_fps: Option<u32>) {
            match cmd {
                CMD_TOGGLE_POMODORO => {
                    let next = !self.scene.pomodoro;
                    self.scene.set_pomodoro(next);
                }
                CMD_TOGGLE_3D => {
                    let next = if self.scene.cat.look.style_3d { "2d" } else { "3d" };
                    self.store.set("style", json!(next));
                    self.apply_scene_options();
                }
                c if (CMD_COAT_BASE..CMD_COAT_BASE + COATS.len() as u32).contains(&c) => {
                    let coat = COATS[(c - CMD_COAT_BASE) as usize].id;
                    self.store.set("coat", json!(coat));
                    self.apply_scene_options();
                }
                CMD_PATTERN_SOLID | CMD_PATTERN_TABBY => {
                    let coat = self.scene.cat.look.coat.clone();
                    let val = if cmd == CMD_PATTERN_TABBY { "tabby" } else { "solid" };
                    self.store.set(&format!("pattern.{coat}"), json!(val));
                    self.apply_scene_options();
                }
                c if (CMD_EYE_BASE..CMD_EYE_BASE + EYE_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("eyeColor", json!(EYE_OPTIONS[(c - CMD_EYE_BASE) as usize].1));
                    self.apply_scene_options();
                }
                c if (CMD_SHINE_BASE..CMD_SHINE_BASE + SHINE_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("eyeShine", json!(SHINE_OPTIONS[(c - CMD_SHINE_BASE) as usize].1));
                    self.apply_scene_options();
                }
                c if (CMD_GLOVE_BASE..CMD_GLOVE_BASE + GLOVE_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("gloves", json!(GLOVE_OPTIONS[(c - CMD_GLOVE_BASE) as usize].1));
                    self.apply_scene_options();
                }
                c if (CMD_COLLAR_BASE..CMD_COLLAR_BASE + COLLAR_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("collar", json!(COLLAR_OPTIONS[(c - CMD_COLLAR_BASE) as usize].1));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_BLUSH => {
                    let v = !self.scene.cat.look.blush;
                    self.store.set("blush", json!(v));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_WHISKERS => {
                    let v = !self.scene.cat.look.whiskers;
                    self.store.set("whiskers", json!(v));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_FESTIVAL => {
                    let v = !self.scene.festival_enabled;
                    self.store.set("festival", json!(v));
                    self.apply_scene_options();
                }
                c if (CMD_SCALE_BASE..CMD_SCALE_BASE + SCALE_OPTIONS.len() as u32).contains(&c) => {
                    self.apply_scale(SCALE_OPTIONS[(c - CMD_SCALE_BASE) as usize].1, true);
                }
                c if (CMD_KB_COLOR_BASE..CMD_KB_COLOR_BASE + KB_COLOR_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("keyboardColor", json!(KB_COLOR_OPTIONS[(c - CMD_KB_COLOR_BASE) as usize].1));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_CABLE => {
                    let v = !self.scene.mouse.cable_visible;
                    self.store.set("mouseCable", json!(v));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_CUP => {
                    let v = !self.scene.cup.enabled;
                    self.store.set("cup", json!(v));
                    self.apply_scene_options();
                }
                c if (CMD_OPACITY_BASE..CMD_OPACITY_BASE + OPACITY_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("opacity", json!(OPACITY_OPTIONS[(c - CMD_OPACITY_BASE) as usize].1));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_SOUND => {
                    let v = !self.scene.sound_enabled;
                    self.store.set("sound", json!(v));
                    self.apply_scene_options();
                }
                CMD_FPS_AUTO => {
                    self.store.set("fps", json!("auto"));
                    self.apply_scene_options();
                }
                CMD_FPS_30 => {
                    self.store.set("fps", json!(30));
                    self.apply_scene_options();
                }
                CMD_FPS_60 => {
                    self.store.set("fps", json!(60));
                    self.apply_scene_options();
                }
                CMD_FPS_HIGH => {
                    if let Some(hz) = high_fps {
                        self.store.set("fps", json!(hz));
                        self.apply_scene_options();
                    }
                }
                CMD_FPS_UNLIM => {
                    self.store.set("fps", json!(0));
                    self.apply_scene_options();
                }
                c if (CMD_SLEEP_BASE..CMD_SLEEP_BASE + SLEEP_OPTIONS.len() as u32).contains(&c) => {
                    self.store.set("sleepAfter", json!(SLEEP_OPTIONS[(c - CMD_SLEEP_BASE) as usize].1));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_PRIVACY => {
                    let v = !self.scene.privacy;
                    self.store.set("privacy", json!(v));
                    self.apply_scene_options();
                }
                CMD_TOGGLE_CLICK_THROUGH => {
                    let v = !self.click_through;
                    self.store.set("clickThrough", json!(v));
                    self.apply_scene_options();
                }
                CMD_OPEN_SETTINGS => {
                    settings_win::open_or_focus(self.hwnd, self.scene.tuning);
                }
                CMD_TOGGLE_AUTOSTART => {
                    let want = if self.autostart == Mode::Off {
                        if self.run_as_admin && elevate::is_elevated() && autostart::task_allowed() {
                            Mode::Task
                        } else {
                            Mode::RunKey
                        }
                    } else {
                        Mode::Off
                    };
                    if autostart::set(want).is_ok() {
                        self.autostart = want;
                    }
                }
                CMD_TOGGLE_ADMIN => {
                    let on = !self.run_as_admin;
                    self.store.set("runAsAdmin", json!(on));
                    self.run_as_admin = on;
                    if on && !elevate::is_elevated() {
                        if elevate::relaunch(&format!("--after {}", std::process::id())).is_ok() {
                            self.store.flush_if_dirty();
                            self.quit = true;
                            unsafe { PostQuitMessage(0); }
                            return;
                        } else {
                            self.store.set("runAsAdmin", json!(false));
                            self.run_as_admin = false;
                        }
                    }
                    self.sync_autostart();
                }
                CMD_HOME => {
                    self.store.remove("originX");
                    self.store.remove("originY");
                    self.apply_scale(self.scale, false);
                }
                CMD_VISIBILITY => {
                    self.set_visible(!self.visible);
                }
                CMD_QUIT => {
                    self.store.flush_if_dirty();
                    self.quit = true;
                    unsafe { PostQuitMessage(0); }
                }
                _ => {}
            }
        }

        fn sync_autostart(&mut self) {
            if cfg!(debug_assertions) || self.autostart == Mode::Off {
                return;
            }
            let want = if self.run_as_admin && elevate::is_elevated() && autostart::task_allowed() {
                Mode::Task
            } else {
                Mode::RunKey
            };
            if self.autostart == Mode::Task && want == Mode::Task && !autostart::task_runs_this() {
                if autostart::set(Mode::Task).is_ok() {
                    self.autostart = Mode::Task;
                }
                return;
            }
            if self.autostart != want && (want == Mode::Task || elevate::is_elevated()) {
                if autostart::set(want).is_ok() {
                    self.autostart = want;
                }
            }
        }

        fn on_settings_changed(&mut self, schema_idx: usize) {
            if let Some(t) = settings_win::current_tuning() {
                self.scene.tuning = t;
                if let Some(meta) = TUNING_SCHEMA.get(schema_idx) {
                    let mut obj = self
                        .store
                        .get("tune")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    let mut tmp = t;
                    let json_val = tmp.set_value(meta.key, t.get_value(meta.key));
                    obj.insert(meta.key.to_string(), json_val);
                    self.store.set_deferred("tune", Value::Object(obj));
                    self.save_deadline = Some(Instant::now() + Duration::from_millis(250));
                }
            }
        }

        fn frame_duration(&self) -> Duration {
            let hz = match self.fps {
                FpsMode::Auto => {
                    let cap = self.display_hz.min(60).max(15);
                    if self.scene.is_busy() {
                        cap
                    } else {
                        cap.min(30)
                    }
                }
                FpsMode::Fixed(n) => n.clamp(15, 240),
                FpsMode::Unlimited => self.display_hz.max(60),
            };
            Duration::from_secs_f64(1.0 / f64::from(hz))
        }
    }

    /// Opens the popup menu WITHOUT holding any RefCell borrow on APP, so nested Win32 messages
    /// dispatched during TrackPopupMenuEx's modal loop can never trigger a re-entrant borrow panic.
    fn open_menu_safely(hwnd: HWND) {
        if MENU_OPEN.with(|m| m.replace(true)) {
            return;
        }
        let snapshot = APP.with(|cell| {
            let mut borrow = cell.try_borrow_mut().ok()?;
            borrow.as_mut().map(App::snapshot_menu)
        });
        let cmd_and_hz = snapshot.map(|owned| {
            let st = owned.as_menu_state();
            let cmd = tray::show_menu(hwnd, &st);
            (cmd, owned.high_fps)
        });
        MENU_OPEN.with(|m| m.set(false));
        if let Some((cmd, high_fps)) = cmd_and_hz {
            if cmd != 0 {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.handle_menu_cmd(cmd, high_fps);
                        }
                    }
                });
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_TRAY => {
                let ev = (lparam.0 & 0xffff) as u32;
                if ev == WM_LBUTTONUP || ev == WM_RBUTTONUP {
                    let _ = PostMessageW(Some(hwnd), WM_APP_MENU, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }
            WM_APP_MENU => {
                open_menu_safely(hwnd);
                LRESULT(0)
            }
            WM_APP_SHOW => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.set_visible(true);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETCURSOR => {
                let menu_open = MENU_OPEN.with(|m| m.get());
                let show_hand = !menu_open
                    && APP.with(|cell| {
                        cell.try_borrow().ok().and_then(|b| {
                            b.as_ref().map(|a| {
                                a.visible && !a.click_through && (a.drag.is_some() || a.scene.hit != HitZone::None)
                            })
                        }).unwrap_or(false)
                    });
                let cursor_id = if show_hand { IDC_HAND } else { IDC_ARROW };
                if let Ok(cur) = LoadCursorW(None, cursor_id) {
                    SetCursor(Some(cur));
                    return LRESULT(1);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_DPICHANGED => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.display_hz = display::max_refresh_hz();
                            app.apply_scale(app.scale, true);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_DISPLAYCHANGE => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.display_hz = display::max_refresh_hz();
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETTINGS_CHANGED => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.on_settings_changed(wparam.0);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETTINGS_RESET => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.store.set("tune", json!({}));
                            app.scene.tuning = Tuning::default();
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETTINGS_PRESET => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            app.scene.tuning.apply_preset(wparam.0);
                            app.store.set("tune", Value::Object(app.scene.tuning.to_diff_json()));
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETTINGS_TEST_ANNOY => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            if !app.visible {
                                app.set_visible(true);
                            }
                            app.scene.annoy_now();
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETTINGS_TEST_CURIOUS => {
                APP.with(|cell| {
                    if let Ok(mut borrow) = cell.try_borrow_mut() {
                        if let Some(ref mut app) = *borrow {
                            if !app.visible {
                                app.set_visible(true);
                            }
                            app.scene.curious_now(3.0);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Bridges the bounded hook channel to the Win32 main loop using an atomic coalescing flag
    /// (`notify_pending`) so bursts of key/mouse input wake the UI thread in <1 ms with a single
    /// PostMessageW instead of flooding the message queue.
    fn spawn_wake_bridge(
        rx: Receiver<Input>,
        main_hwnd: isize,
        notify_pending: Arc<AtomicBool>,
    ) -> Receiver<Input> {
        let (tx2, rx2) = mpsc::sync_channel::<Input>(256);
        thread::Builder::new()
            .name("input-wake".into())
            .spawn(move || {
                while let Ok(ev) = rx.recv() {
                    if tx2.try_send(ev).is_ok()
                        && notify_pending
                            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                            .is_ok()
                    {
                        unsafe {
                            let _ = PostMessageW(
                                Some(HWND(main_hwnd as *mut _)),
                                WM_APP_INPUT,
                                WPARAM(0),
                                LPARAM(0),
                            );
                        }
                    }
                }
            })
            .expect("spawn input-wake");
        rx2
    }

    pub fn run() {
        if !prepare_launch() {
            return;
        }
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        // Single-instance guard: focus/show existing copy if already running.
        let mutex = unsafe { CreateMutexW(None, false, w!("Local\\KeyCatNativeSingleInstance")).ok() };
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            if let Ok(existing) = unsafe { FindWindowW(w!("KeyCatNativePanel"), None) } {
                unsafe {
                    let _ = PostMessageW(Some(existing), WM_APP_SHOW, WPARAM(0), LPARAM(0));
                }
            }
            if let Some(h) = mutex {
                unsafe { let _ = CloseHandle(h); }
            }
            unsafe { CoUninitialize(); }
            return;
        }

        let store = Store::open(settings_path());
        let scale = read_scale(&store);
        let hinst = unsafe { GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0)) };
        let cls = w!("KeyCatNativePanel");
        unsafe {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinst.unwrap_or_default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: cls,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);
        }

        let ex_style = WS_EX_NOREDIRECTIONBITMAP
            | WS_EX_TOPMOST
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE
            | WS_EX_TRANSPARENT;
        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                cls,
                w!("KeyCat"),
                WS_POPUP,
                0,
                0,
                360,
                368,
                None,
                None,
                hinst,
                None,
            )
            .expect("create panel window")
        };

        let factor = dpi_scale(hwnd);
        let (pw, ph) = window_phys_size(scale, factor);
        let (ox, oy) = saved_origin(&store).unwrap_or_else(|| default_origin(pw, ph, factor));
        unsafe {
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), ox, oy, pw, ph, SWP_NOACTIVATE);
        }

        let mut scene = Scene::new();
        sync_local_clock(&mut scene);
        if let Some(mem_val) = store.get("memory") {
            if let Ok(s) = serde_json::to_string(mem_val) {
                scene.life.load_json(&s);
            }
        }
        scene.set_viewport((pw as f64 / factor) as f32, (ph as f64 / factor) as f32);
        let renderer = Renderer::new(hwnd, pw as u32, ph as u32, &scene)
            .expect("init D3D11 + DirectComposition renderer");
        let audio = PurrAudio::new();

        let tray = TrayIcon::new(hwnd);

        let (sink, raw_rx) = mpsc::sync_channel::<Input>(256);
        let pointer_dirty = Arc::new(AtomicBool::new(false));
        let notify_pending = Arc::new(AtomicBool::new(false));
        let hook_guard = hook::start(sink, pointer_dirty.clone()).ok();
        let hook_ready = hook_guard.is_some();
        let inputs = spawn_wake_bridge(raw_rx, hwnd.0 as isize, notify_pending.clone());

        let mut app = App {
            hwnd,
            store,
            scene,
            renderer,
            audio,
            _tray: tray,
            visible: true,
            click_through: false,
            run_as_admin: false,
            autostart: if cfg!(debug_assertions) { Mode::Off } else { autostart::current() },
            scale,
            fps: FpsMode::Auto,
            display_hz: display::max_refresh_hz(),
            hook_ready,
            held: HashMap::new(),
            last_key_audit: Instant::now(),
            last_clock_sync: Instant::now(),
            last_pet_click: None,
            petting: false,
            drag: None,
            last_pointer: None,
            last_cursor: (-9999, -9999),
            pending_delta: (0.0, 0.0),
            click_ignored: true,
            save_deadline: None,
            quit: false,
        };
        app.apply_scene_options();
        app.sync_autostart();
        app.renderer.render(&mut app.scene);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }

        APP.with(|cell| *cell.borrow_mut() = Some(app));

        // Main event + frame loop
        let mut last_frame = Instant::now();
        let mut next_frame = last_frame;
        let mut msg = MSG::default();

        loop {
            // Drain all pending Win32 messages
            let mut quitting = false;
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                        quitting = true;
                        break;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if quitting {
                break;
            }

            // Drain hook inputs & flush debounced settings
            let now = Instant::now();
            let (visible, quit_flag, frame_dur, should_render) = APP.with(|cell| {
                let mut borrow = cell.borrow_mut();
                let Some(ref mut app) = *borrow else {
                    return (false, true, Duration::from_millis(16), false);
                };
                // Clear coalescing flag before draining so concurrent input never misses a wakeup.
                notify_pending.store(false, Ordering::Release);
                let mut woke_pointer = pointer_dirty.swap(false, Ordering::AcqRel);
                while let Ok(ev) = inputs.try_recv() {
                    if matches!(ev, Input::PointerWake | Input::Wheel(_)) {
                        woke_pointer = true;
                    }
                    app.on_input(ev);
                }
                if woke_pointer || app.drag.is_some() {
                    app.sync_pointer();
                }
                if let Some(dl) = app.save_deadline {
                    if now >= dl {
                        app.save_deadline = None;
                        app.store.flush_if_dirty();
                    }
                }
                let due = app.visible && now >= next_frame;
                if due {
                    app.reconcile_held_keys(now);
                    if now.saturating_duration_since(app.last_clock_sync) >= Duration::from_secs(30) {
                        app.last_clock_sync = now;
                        sync_local_clock(&mut app.scene);
                    }
                    let dt = now.saturating_duration_since(last_frame).as_secs_f32().min(0.1);
                    last_frame = now;
                    app.scene.step(dt);
                    let (a_on, a_lvl, a_br) = app.scene.purr_audio_state();
                    app.audio.sync(a_on, a_lvl, a_br);
                    if let Some(mem_json) = app.scene.life.take_dirty_json(app.scene.time, false) {
                        if let Ok(v) = serde_json::from_str::<Value>(&mem_json) {
                            app.store.set_deferred("memory", v);
                            app.save_deadline = Some(now + Duration::from_millis(500));
                        }
                    }
                    app.update_click_target();
                    app.renderer.render(&mut app.scene);
                }
                (app.visible, app.quit, app.frame_duration(), due)
            });

            if quit_flag {
                break;
            }
            if should_render {
                next_frame = Instant::now() + frame_dur;
            } else if !visible {
                last_frame = Instant::now();
                next_frame = last_frame;
            }

            // Sleep efficiently in kernel until next frame or next Win32/input message
            let wait_ms = if !visible {
                INFINITE
            } else {
                let rem = next_frame.saturating_duration_since(Instant::now());
                let ms = rem.as_millis().min(50) as u32;
                if ms == 0 { 1 } else { ms }
            };
            unsafe {
                let _ = MsgWaitForMultipleObjectsEx(None, wait_ms, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
            }
        }

        drop(hook_guard);
        APP.with(|cell| {
            if let Some(mut app) = cell.borrow_mut().take() {
                if let Some(mem_json) = app.scene.life.take_dirty_json(app.scene.time, true) {
                    if let Ok(v) = serde_json::from_str::<Value>(&mem_json) {
                        app.store.set("memory", v);
                    }
                }
                app.store.flush_if_dirty();
            }
        });
        if let Some(h) = mutex {
            unsafe { let _ = CloseHandle(h); }
        }
        unsafe { CoUninitialize(); }
    }
}
