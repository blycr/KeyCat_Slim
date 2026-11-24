//! Native Win32 notification area icon (`Shell_NotifyIconW`) and dynamic popup menu.

#[cfg(windows)]
mod imp {
    use keycat_core::coats::COATS;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, SetForegroundWindow,
        TrackPopupMenuEx, HMENU, IDI_APPLICATION, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
        MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP,
        WM_NULL,
    };

    use crate::autostart::Mode;

    pub const WM_TRAY: u32 = WM_APP + 1;

    pub const CMD_TOGGLE_3D: u32 = 1001;
    pub const CMD_COAT_BASE: u32 = 1100; // + coat index
    pub const CMD_PATTERN_SOLID: u32 = 1200;
    pub const CMD_PATTERN_TABBY: u32 = 1201;
    pub const CMD_EYE_BASE: u32 = 1300;
    pub const CMD_SHINE_BASE: u32 = 1320;
    pub const CMD_COLLAR_BASE: u32 = 1340;
    pub const CMD_TOGGLE_BLUSH: u32 = 1360;
    pub const CMD_TOGGLE_WHISKERS: u32 = 1361;
    pub const CMD_SCALE_BASE: u32 = 1400;
    pub const CMD_FPS_AUTO: u32 = 1449;
    pub const CMD_FPS_30: u32 = 1450;
    pub const CMD_FPS_60: u32 = 1451;
    pub const CMD_FPS_HIGH: u32 = 1452;
    pub const CMD_FPS_UNLIM: u32 = 1453;
    pub const CMD_SLEEP_BASE: u32 = 1500;
    pub const CMD_TOGGLE_PRIVACY: u32 = 1600;
    pub const CMD_TOGGLE_CLICK_THROUGH: u32 = 1601;
    pub const CMD_OPEN_SETTINGS: u32 = 1602;
    pub const CMD_TOGGLE_AUTOSTART: u32 = 1700;
    pub const CMD_TOGGLE_ADMIN: u32 = 1701;
    pub const CMD_HOME: u32 = 1702;
    pub const CMD_VISIBILITY: u32 = 1703;
    pub const CMD_QUIT: u32 = 1900;

    pub const EYE_OPTIONS: &[(&str, &str)] = &[
        ("跟随毛色", "auto"),
        ("金色", "gold"),
        ("绿色", "green"),
        ("蓝色", "blue"),
        ("琥珀色", "amber"),
        ("鸳鸯眼（一蓝一金）", "odd"),
    ];

    pub const SHINE_OPTIONS: &[(&str, &str)] = &[
        ("无", "none"),
        ("经典", "classic"),
        ("闪亮", "sparkle"),
    ];

    pub const COLLAR_OPTIONS: &[(&str, &str)] = &[
        ("无", "none"),
        ("红色铃铛", "red"),
        ("蓝色铃铛", "blue"),
        ("粉色铃铛", "pink"),
        ("绿色铃铛", "green"),
    ];

    pub const SCALE_OPTIONS: &[(&str, f64)] = &[
        ("特小", 0.5),
        ("小", 0.7),
        ("中", 0.9),
        ("大", 1.1),
        ("特大", 1.35),
    ];

    pub const SLEEP_OPTIONS: &[(&str, f64)] = &[
        ("30 秒", 30.0),
        ("1 分钟", 60.0),
        ("3 分钟", 180.0),
        ("10 分钟", 600.0),
        ("不睡", 0.0),
    ];

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum FpsMode {
        Auto,
        Fixed(u32),
        Unlimited,
    }

    pub struct MenuState<'a> {
        pub style_3d: bool,
        pub coat: &'a str,
        pub pattern: &'a str,
        pub eye: &'a str,
        pub shine: &'a str,
        pub collar: &'a str,
        pub blush: bool,
        pub whiskers: bool,
        pub scale: f64,
        pub fps: FpsMode,
        pub high_fps: Option<u32>,
        pub sleep_after: f64,
        pub privacy: bool,
        pub click_through: bool,
        pub autostart: Mode,
        pub run_as_admin: bool,
        pub visible: bool,
        pub hook_ready: bool,
    }

    pub struct TrayIcon {
        hwnd: HWND,
        id: u32,
    }

    impl TrayIcon {
        pub fn new(hwnd: HWND) -> Self {
            unsafe {
                let module = GetModuleHandleW(None).ok();
                let icon = LoadIconW(module.map(Into::into), PCWSTR(1 as *const u16))
                    .or_else(|_| LoadIconW(None, IDI_APPLICATION))
                    .unwrap_or_default();
                let mut nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: 1,
                    uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                    uCallbackMessage: WM_TRAY,
                    hIcon: icon,
                    ..Default::default()
                };
                let tip: Vec<u16> = "KeyCat".encode_utf16().collect();
                for (dst, &src) in nid.szTip.iter_mut().zip(tip.iter()) {
                    *dst = src;
                }
                let _ = Shell_NotifyIconW(NIM_ADD, &nid);
            }
            Self { hwnd, id: 1 }
        }
    }

    impl Drop for TrayIcon {
        fn drop(&mut self) {
            unsafe {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: self.hwnd,
                    uID: self.id,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    unsafe fn add_item(menu: HMENU, id: u32, text: &str, checked: bool) {
        let w = wide(text);
        let flags = MF_STRING | if checked { MF_CHECKED } else { Default::default() };
        let _ = AppendMenuW(menu, flags, id as usize, PCWSTR(w.as_ptr()));
    }

    unsafe fn add_note(menu: HMENU, text: &str) {
        let w = wide(text);
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(w.as_ptr()));
    }

    unsafe fn add_sep(menu: HMENU) {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    }

    unsafe fn add_sub(parent: HMENU, title: &str, sub: HMENU) {
        let w = wide(title);
        let _ = AppendMenuW(parent, MF_POPUP, sub.0 as usize, PCWSTR(w.as_ptr()));
    }

    /// Displays the native popup menu at the cursor and returns the chosen command ID (or 0).
    pub fn show_menu(hwnd: HWND, st: &MenuState<'_>) -> u32 {
        unsafe {
            let Ok(root) = CreatePopupMenu() else { return 0 };

            add_item(root, CMD_TOGGLE_3D, "立体效果（3D）", st.style_3d);

            if let Ok(sub) = CreatePopupMenu() {
                for (i, c) in COATS.iter().enumerate() {
                    add_item(sub, CMD_COAT_BASE + i as u32, c.label, st.coat == c.id);
                }
                add_sub(root, "毛色", sub);
            }

            if st.coat == "orange" || st.coat == "grey" {
                if let Ok(sub) = CreatePopupMenu() {
                    let striped = st.pattern == "tabby"
                        || (st.pattern == "auto" && st.coat == "orange");
                    add_item(sub, CMD_PATTERN_SOLID, "纯色", !striped);
                    add_item(sub, CMD_PATTERN_TABBY, "虎斑条纹", striped);
                    add_sub(root, "花纹", sub);
                }
            }

            if let Ok(look) = CreatePopupMenu() {
                if let Ok(sub) = CreatePopupMenu() {
                    for (i, &(label, id)) in EYE_OPTIONS.iter().enumerate() {
                        add_item(sub, CMD_EYE_BASE + i as u32, label, st.eye == id);
                    }
                    add_sub(look, "眼睛颜色", sub);
                }
                if let Ok(sub) = CreatePopupMenu() {
                    for (i, &(label, id)) in SHINE_OPTIONS.iter().enumerate() {
                        add_item(sub, CMD_SHINE_BASE + i as u32, label, st.shine == id);
                    }
                    add_sub(look, "眼睛高光", sub);
                }
                if let Ok(sub) = CreatePopupMenu() {
                    for (i, &(label, id)) in COLLAR_OPTIONS.iter().enumerate() {
                        add_item(sub, CMD_COLLAR_BASE + i as u32, label, st.collar == id);
                    }
                    add_sub(look, "项圈", sub);
                }
                add_item(look, CMD_TOGGLE_BLUSH, "腮红", st.blush);
                add_item(look, CMD_TOGGLE_WHISKERS, "胡须", st.whiskers);
                add_sub(root, "外观", look);
            }

            if let Ok(sub) = CreatePopupMenu() {
                for (i, &(label, val)) in SCALE_OPTIONS.iter().enumerate() {
                    add_item(sub, CMD_SCALE_BASE + i as u32, label, (st.scale - val).abs() < 0.01);
                }
                add_sub(root, "大小", sub);
            }

            if let Ok(sub) = CreatePopupMenu() {
                add_item(sub, CMD_FPS_AUTO, "自动", st.fps == FpsMode::Auto);
                add_item(sub, CMD_FPS_30, "30 fps", st.fps == FpsMode::Fixed(30));
                add_item(sub, CMD_FPS_60, "60 fps", st.fps == FpsMode::Fixed(60));
                if let Some(hz) = st.high_fps {
                    add_item(sub, CMD_FPS_HIGH, &format!("{hz} fps"), st.fps == FpsMode::Fixed(hz));
                }
                add_item(sub, CMD_FPS_UNLIM, "不限制", st.fps == FpsMode::Unlimited);
                add_sub(root, "帧率", sub);
            }

            if let Ok(sub) = CreatePopupMenu() {
                for (i, &(label, val)) in SLEEP_OPTIONS.iter().enumerate() {
                    add_item(sub, CMD_SLEEP_BASE + i as u32, label, (st.sleep_after - val).abs() < 0.5);
                }
                add_sub(root, "多久睡着", sub);
            }

            add_item(root, CMD_TOGGLE_PRIVACY, "隐私模式（不显示键名）", st.privacy);
            add_item(root, CMD_TOGGLE_CLICK_THROUGH, "鼠标穿透（点击直接落到下层窗口）", st.click_through);
            add_item(root, CMD_OPEN_SETTINGS, "调节反应…", false);
            add_sep(root);
            add_item(root, CMD_TOGGLE_AUTOSTART, "开机自启", st.autostart != Mode::Off);
            add_item(root, CMD_TOGGLE_ADMIN, "以管理员身份运行（游戏里也有按键反馈）", st.run_as_admin);
            add_item(root, CMD_HOME, "回到右下角", false);
            add_item(root, CMD_VISIBILITY, if st.visible { "隐藏" } else { "显示" }, false);
            add_sep(root);
            add_note(root, "KeyCat v2.1.1");
            if !st.hook_ready {
                add_note(root, "键盘监听未启动");
            }
            add_sep(root);
            add_item(root, CMD_QUIT, "退出 KeyCat", false);

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(hwnd);
            let cmd = TrackPopupMenuEx(
                root,
                (TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD).0,
                pt.x,
                pt.y,
                hwnd,
                None,
            );
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(Some(hwnd), WM_NULL, Default::default(), Default::default());
            let _ = DestroyMenu(root);
            let _ = PWSTR::null();
            cmd.0 as u32
        }
    }
}

#[cfg(windows)]
pub use imp::*;
