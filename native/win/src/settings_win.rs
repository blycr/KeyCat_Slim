//! Native Direct2D + DirectWrite "调节反应…" settings window (zero WebView2).
//! Warm cream / dark charcoal + caramel accent palette matching KeyCat's visual identity,
//! with sticky bottom action bar, hover/press/trigger feedback, dark titlebar support,
//! disabled group sliders, Esc-to-close, and hidden scrollbars.

#[cfg(windows)]
mod imp {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::time::{Duration, Instant};

    use keycat_core::tuning::{ParamKind, Tuning, GROUPS, TUNING_SCHEMA};
    use windows::core::{w, BOOL, PCWSTR, Result};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F, D2D_SIZE_U};
    use windows::Win32::Graphics::Direct2D::{
        D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
        D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
        D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_PROPERTIES, D2D1_ROUNDED_RECT,
        D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
    };
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
        DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
        DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_WORD_WRAPPING_WRAP,
    };
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
    use windows::Win32::Graphics::Gdi::{InvalidateRect, ValidateRect};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_DOWN,
        VK_ESCAPE, VK_NEXT, VK_PRIOR, VK_UP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics,
        KillTimer, LoadCursorW, LoadIconW, PostMessageW, RegisterClassW, SetCursor,
        SetForegroundWindow, SetTimer, ShowWindow, CS_HREDRAW, CS_VREDRAW, IDC_ARROW, IDC_HAND,
        SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNORMAL, WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_DESTROY,
        WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
        WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE, WM_TIMER, WNDCLASSW, WS_CAPTION, WS_MINIMIZEBOX,
        WS_SYSMENU, WS_VISIBLE,
    };
    use windows_numerics::Vector2;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    pub const WM_SETTINGS_CHANGED: u32 = WM_APP + 10;
    pub const WM_SETTINGS_RESET: u32 = WM_APP + 11;
    pub const WM_SETTINGS_TEST_ANNOY: u32 = WM_APP + 12;
    pub const WM_SETTINGS_TEST_CURIOUS: u32 = WM_APP + 13;

    const WM_MOUSELEAVE: u32 = 0x02A3;

    const LOGICAL_W: f32 = 424.0;
    const PAD_X: f32 = 20.0;
    const FOOTER_H: f32 = 58.0;
    const TIMER_FEEDBACK: usize = 1;
    const FEEDBACK_MS: u32 = 1200;

    #[derive(Clone, Copy)]
    struct Palette {
        bg: D2D1_COLOR_F,
        footer_bg: D2D1_COLOR_F,
        card: D2D1_COLOR_F,
        btn_hover: D2D1_COLOR_F,
        btn_pressed: D2D1_COLOR_F,
        text: D2D1_COLOR_F,
        muted: D2D1_COLOR_F,
        line: D2D1_COLOR_F,
        accent: D2D1_COLOR_F,
        accent_text: D2D1_COLOR_F,
    }

    fn hex(rgb: u32, a: f32) -> D2D1_COLOR_F {
        D2D1_COLOR_F {
            r: ((rgb >> 16) & 0xff) as f32 / 255.0,
            g: ((rgb >> 8) & 0xff) as f32 / 255.0,
            b: (rgb & 0xff) as f32 / 255.0,
            a,
        }
    }

    fn detect_dark_mode() -> bool {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
            .map(|v| v == 0)
            .unwrap_or(false)
    }

    fn apply_titlebar_theme(hwnd: HWND) {
        let dark = BOOL::from(detect_dark_mode());
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark as *const BOOL).cast::<c_void>(),
                std::mem::size_of::<BOOL>() as u32,
            );
        }
    }

    fn palette() -> Palette {
        if detect_dark_mode() {
            Palette {
                bg: hex(0x23211d, 1.0),
                footer_bg: hex(0x1f1d19, 1.0),
                card: hex(0x2d2a25, 1.0),
                btn_hover: hex(0x38342d, 1.0),
                btn_pressed: hex(0x454037, 1.0),
                text: hex(0xeee8da, 1.0),
                muted: hex(0xa59d8d, 1.0),
                line: hex(0x3d3932, 1.0),
                accent: hex(0xf0a35a, 1.0),
                accent_text: hex(0x1d1a16, 1.0),
            }
        } else {
            Palette {
                bg: hex(0xf7f2e4, 1.0),
                footer_bg: hex(0xf2ecdc, 1.0),
                card: hex(0xfffdf7, 1.0),
                btn_hover: hex(0xf8f3e6, 1.0),
                btn_pressed: hex(0xebe3cf, 1.0),
                text: hex(0x2a2722, 1.0),
                muted: hex(0x8a8274, 1.0),
                line: hex(0xe6dfcd, 1.0),
                accent: hex(0xe08a3c, 1.0),
                accent_text: hex(0xfffdf7, 1.0),
            }
        }
    }

    #[derive(Clone, Copy)]
    struct RowLayout {
        index: usize,
        top: f32,
        height: f32,
        hint_h: f32,
        slider_y: f32,
        slider_left: f32,
        slider_right: f32,
        toggle_rect: D2D_RECT_F,
    }

    struct Inner {
        hwnd: HWND,
        owner: HWND,
        d2d: ID2D1Factory,
        _dwrite: IDWriteFactory,
        rt: Option<ID2D1HwndRenderTarget>,
        fmt_subtitle: IDWriteTextFormat,
        fmt_heading: IDWriteTextFormat,
        fmt_label: IDWriteTextFormat,
        fmt_value: IDWriteTextFormat,
        fmt_hint: IDWriteTextFormat,
        fmt_btn: IDWriteTextFormat,
        tuning: Tuning,
        scroll_y: f32,
        content_h: f32,
        rows: Vec<RowLayout>,
        dragging_row: Option<usize>,
        hover_btn: Option<usize>,
        pressed_btn: Option<usize>,
        hover_interactive: bool,
        tracking_mouse: bool,
        feedback_btn: Option<(usize, Instant)>,
    }

    thread_local! {
        static STATE: RefCell<Option<Inner>> = const { RefCell::new(None) };
    }

    pub fn open_or_focus(owner: HWND, tuning: Tuning) {
        let existing = STATE.with(|s| s.try_borrow().ok().and_then(|b| b.as_ref().map(|i| i.hwnd)));
        if let Some(hwnd) = existing {
            STATE.with(|s| {
                if let Ok(mut b) = s.try_borrow_mut() {
                    if let Some(ref mut inner) = *b {
                        inner.tuning = tuning;
                    }
                }
            });
            apply_titlebar_theme(hwnd);
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
                let _ = SetForegroundWindow(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            return;
        }
        let _ = create_window(owner, tuning);
    }

    pub fn current_tuning() -> Option<Tuning> {
        STATE.with(|s| s.try_borrow().ok().and_then(|b| b.as_ref().map(|i| i.tuning)))
    }

    fn create_window(owner: HWND, tuning: Tuning) -> Result<()> {
        unsafe {
            let hinst = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
            let icon = LoadIconW(hinst, PCWSTR(1 as *const u16)).unwrap_or_default();
            let cls = w!("KeyCatNativeSettings");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinst.unwrap_or_default(),
                hIcon: icon,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: cls,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let dpi = GetDpiForWindow(owner).max(96);
            let scale = dpi as f32 / 96.0;
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let logical_h = ((screen_h as f32 / scale) - 96.0).min(780.0).max(480.0);

            let mut rc = RECT {
                left: 0,
                top: 0,
                right: (LOGICAL_W * scale).round() as i32,
                bottom: (logical_h * scale).round() as i32,
            };
            let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE;
            let _ = AdjustWindowRectExForDpi(&mut rc, style, false, WINDOW_EX_STYLE(0), dpi);
            let win_w = rc.right - rc.left;
            let win_h = rc.bottom - rc.top;
            let x = (screen_w - win_w) / 2;
            let y = (screen_h - win_h) / 2;

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                cls,
                w!("KeyCat 调节"),
                style,
                x,
                y,
                win_w,
                win_h,
                None,
                None,
                hinst,
                None,
            )?;
            apply_titlebar_theme(hwnd);

            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

            let make_fmt = |size: f32, bold: bool, align, para_align| -> Result<IDWriteTextFormat> {
                let f = dwrite.CreateTextFormat(
                    w!("Microsoft YaHei UI"),
                    None,
                    if bold { DWRITE_FONT_WEIGHT_SEMI_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL },
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    w!("zh-CN"),
                )?;
                f.SetTextAlignment(align)?;
                f.SetParagraphAlignment(para_align)?;
                f.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
                Ok(f)
            };

            let fmt_subtitle = make_fmt(12.0, false, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let fmt_heading = make_fmt(13.0, true, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let fmt_label = make_fmt(13.0, false, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let fmt_value = make_fmt(13.0, true, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let fmt_hint = make_fmt(11.5, false, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
            let fmt_btn = make_fmt(13.0, true, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

            let mut inner = Inner {
                hwnd,
                owner,
                d2d,
                _dwrite: dwrite,
                rt: None,
                fmt_subtitle,
                fmt_heading,
                fmt_label,
                fmt_value,
                fmt_hint,
                fmt_btn,
                tuning,
                scroll_y: 0.0,
                content_h: 0.0,
                rows: Vec::new(),
                dragging_row: None,
                hover_btn: None,
                pressed_btn: None,
                hover_interactive: false,
                tracking_mouse: false,
                feedback_btn: None,
            };
            inner.compute_layout();
            inner.ensure_rt()?;
            STATE.with(|s| *s.borrow_mut() = Some(inner));
            let _ = InvalidateRect(Some(hwnd), None, false);
            Ok(())
        }
    }

    impl Inner {
        fn ensure_rt(&mut self) -> Result<()> {
            if self.rt.is_some() {
                return Ok(());
            }
            unsafe {
                let mut rc = RECT::default();
                let _ = GetClientRect(self.hwnd, &mut rc);
                let size = D2D_SIZE_U {
                    width: (rc.right - rc.left).max(1) as u32,
                    height: (rc.bottom - rc.top).max(1) as u32,
                };
                let dpi = GetDpiForWindow(self.hwnd).max(96) as f32;
                let props = D2D1_RENDER_TARGET_PROPERTIES {
                    dpiX: dpi,
                    dpiY: dpi,
                    ..Default::default()
                };
                let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                    hwnd: self.hwnd,
                    pixelSize: size,
                    ..Default::default()
                };
                let rt = self.d2d.CreateHwndRenderTarget(&props, &hwnd_props)?;
                rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
                self.rt = Some(rt);
                Ok(())
            }
        }

        fn view_height(&self) -> f32 {
            let mut rc = RECT::default();
            unsafe { let _ = GetClientRect(self.hwnd, &mut rc); }
            let dpi = unsafe { GetDpiForWindow(self.hwnd).max(96) } as f32;
            (rc.bottom - rc.top) as f32 * 96.0 / dpi
        }

        fn scroll_view_height(&self) -> f32 {
            (self.view_height() - FOOTER_H).max(1.0)
        }

        fn footer_btn_rects(&self) -> [D2D_RECT_F; 3] {
            let card_left = PAD_X;
            let card_right = LOGICAL_W - PAD_X;
            let gap = 8.0;
            let total_w = card_right - card_left;
            let reset_w = 88.0;
            let test_w = (total_w - reset_w - gap * 2.0) * 0.5;
            let bh = 34.0;
            let footer_top = self.view_height() - FOOTER_H;
            let y = footer_top + (FOOTER_H - bh) * 0.5;
            [
                D2D_RECT_F { left: card_left, top: y, right: card_left + test_w, bottom: y + bh },
                D2D_RECT_F {
                    left: card_left + test_w + gap,
                    top: y,
                    right: card_left + test_w * 2.0 + gap,
                    bottom: y + bh,
                },
                D2D_RECT_F { left: card_right - reset_w, top: y, right: card_right, bottom: y + bh },
            ]
        }

        fn row_enabled(&self, meta_idx: usize) -> bool {
            let meta = &TUNING_SCHEMA[meta_idx];
            if meta.kind == ParamKind::Toggle {
                return true;
            }
            if meta.group == GROUPS[0] {
                self.tuning.annoy_on
            } else {
                self.tuning.curious_on
            }
        }

        fn compute_layout(&mut self) {
            self.rows.clear();
            let mut y = 42.0f32; // room for top subtitle
            let card_left = PAD_X;
            let card_right = LOGICAL_W - PAD_X;
            let inner_l = card_left + 14.0;
            let inner_r = card_right - 14.0;

            for (gi, &group) in GROUPS.iter().enumerate() {
                if gi > 0 {
                    y += 16.0;
                }
                y += 24.0; // group title
                y += 4.0;  // card top pad
                for (idx, meta) in TUNING_SCHEMA.iter().enumerate() {
                    if meta.group != group {
                        continue;
                    }
                    let row_top = y;
                    let hint_chars = meta.hint.chars().count();
                    let hint_h = if hint_chars == 0 {
                        0.0
                    } else if hint_chars > 24 {
                        34.0
                    } else {
                        20.0
                    };
                    let h = match meta.kind {
                        ParamKind::Toggle => if hint_h > 0.0 { 36.0 + hint_h } else { 40.0 },
                        ParamKind::Range => if hint_h > 0.0 { 58.0 + hint_h } else { 60.0 },
                    };
                    let toggle_rect = D2D_RECT_F {
                        left: inner_r - 36.0,
                        top: row_top + 10.0,
                        right: inner_r,
                        bottom: row_top + 30.0,
                    };
                    self.rows.push(RowLayout {
                        index: idx,
                        top: row_top,
                        height: h,
                        hint_h,
                        slider_y: row_top + 40.0,
                        slider_left: inner_l + 8.0,
                        slider_right: inner_r - 8.0,
                        toggle_rect,
                    });
                    y += h;
                }
                y += 4.0; // card bottom pad
            }
            self.content_h = y + 16.0;
        }

        fn max_scroll(&self) -> f32 {
            (self.content_h - self.scroll_view_height()).max(0.0)
        }

        fn active_feedback(&mut self) -> Option<usize> {
            if let Some((idx, until)) = self.feedback_btn {
                if Instant::now() < until {
                    return Some(idx);
                }
                self.feedback_btn = None;
            }
            None
        }

        fn paint(&mut self) {
            if self.ensure_rt().is_err() {
                return;
            }
            let feedback_idx = self.active_feedback();
            let pal = palette();
            let view_h = self.view_height();
            let scroll_h = self.scroll_view_height();
            let btns = self.footer_btn_rects();
            let Some(rt) = self.rt.clone() else { return };
            unsafe {
                rt.BeginDraw();
                rt.Clear(Some(&pal.bg));
                let Ok(b_footer) = rt.CreateSolidColorBrush(&pal.footer_bg, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_card) = rt.CreateSolidColorBrush(&pal.card, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_hover) = rt.CreateSolidColorBrush(&pal.btn_hover, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_pressed) = rt.CreateSolidColorBrush(&pal.btn_pressed, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_line) = rt.CreateSolidColorBrush(&pal.line, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_text) = rt.CreateSolidColorBrush(&pal.text, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_muted) = rt.CreateSolidColorBrush(&pal.muted, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_accent) = rt.CreateSolidColorBrush(&pal.accent, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_accent_txt) = rt.CreateSolidColorBrush(&pal.accent_text, None) else { let _ = rt.EndDraw(None, None); return; };

                let sy = self.scroll_y;
                let card_l = PAD_X;
                let card_r = LOGICAL_W - PAD_X;
                let inner_l = card_l + 14.0;
                let inner_r = card_r - 14.0;

                // Clip scrollable content strictly above the sticky footer (hidden scrollbar)
                let clip_rc = D2D_RECT_F { left: 0.0, top: 0.0, right: LOGICAL_W, bottom: scroll_h };
                rt.PushAxisAlignedClip(&clip_rc, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);

                // Top subtitle
                let sub_rc = D2D_RECT_F {
                    left: card_l + 4.0,
                    top: 12.0 - sy,
                    right: card_r,
                    bottom: 34.0 - sy,
                };
                draw_text(&rt, "改动实时生效，可随时点下方按钮看小猫反应", &self.fmt_subtitle, &sub_rc, &b_muted);

                // Group headers and cards
                let mut cur_y = 42.0f32;
                for (gi, &group) in GROUPS.iter().enumerate() {
                    if gi > 0 {
                        cur_y += 16.0;
                    }
                    let title_rc = D2D_RECT_F {
                        left: card_l + 4.0,
                        top: cur_y - sy,
                        right: card_r,
                        bottom: cur_y + 22.0 - sy,
                    };
                    draw_text(&rt, group, &self.fmt_heading, &title_rc, &b_muted);
                    cur_y += 24.0;

                    let card_top = cur_y;
                    let group_h: f32 = self
                        .rows
                        .iter()
                        .filter(|r| TUNING_SCHEMA[r.index].group == group)
                        .map(|r| r.height)
                        .sum();
                    let card_bot = card_top + group_h + 8.0;
                    let rr = D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: card_l,
                            top: card_top - sy,
                            right: card_r,
                            bottom: card_bot - sy,
                        },
                        radiusX: 10.0,
                        radiusY: 10.0,
                    };
                    rt.FillRoundedRectangle(&rr, &b_card);
                    rt.DrawRoundedRectangle(&rr, &b_line, 1.0, None);
                    cur_y = card_bot;
                }

                // Rows
                for (ri, row) in self.rows.iter().enumerate() {
                    let meta = &TUNING_SCHEMA[row.index];
                    let enabled = self.row_enabled(row.index);
                    let dim = meta.kind == ParamKind::Range && !enabled;
                    let label_brush = if dim { &b_muted } else { &b_text };
                    let top = row.top - sy;

                    let is_first_in_group = ri == 0 || TUNING_SCHEMA[self.rows[ri - 1].index].group != meta.group;
                    if !is_first_in_group {
                        rt.DrawLine(
                            Vector2 { X: inner_l, Y: top },
                            Vector2 { X: inner_r, Y: top },
                            &b_line,
                            1.0,
                            None,
                        );
                    }

                    let head_rc = D2D_RECT_F {
                        left: inner_l,
                        top: top + 6.0,
                        right: inner_r - 86.0,
                        bottom: top + 28.0,
                    };
                    draw_text(&rt, meta.label, &self.fmt_label, &head_rc, label_brush);

                    let val = self.tuning.get_value(meta.key);
                    match meta.kind {
                        ParamKind::Toggle => {
                            let on = val >= 0.5;
                            let tr = D2D_RECT_F {
                                left: row.toggle_rect.left,
                                top: row.toggle_rect.top - sy,
                                right: row.toggle_rect.right,
                                bottom: row.toggle_rect.bottom - sy,
                            };
                            let rr = D2D1_ROUNDED_RECT { rect: tr, radiusX: 10.0, radiusY: 10.0 };
                            rt.FillRoundedRectangle(&rr, if on { &b_accent } else { &b_line });
                            let kx = if on { tr.right - 10.0 } else { tr.left + 10.0 };
                            let ky = (tr.top + tr.bottom) * 0.5;
                            rt.FillEllipse(&D2D1_ELLIPSE { point: Vector2 { X: kx, Y: ky }, radiusX: 7.5, radiusY: 7.5 }, &b_card);
                        }
                        ParamKind::Range => {
                            let txt = if meta.step < 1.0 {
                                format!("{val:.1} {}", meta.unit)
                            } else {
                                format!("{} {}", val.round() as i32, meta.unit)
                            };
                            let val_rc = D2D_RECT_F {
                                left: inner_r - 92.0,
                                top: top + 6.0,
                                right: inner_r,
                                bottom: top + 28.0,
                            };
                            draw_text(&rt, &txt, &self.fmt_value, &val_rc, if dim { &b_muted } else { &b_accent });

                            let track_y = row.slider_y - sy;
                            let t = ((val - meta.min) / (meta.max - meta.min)).clamp(0.0, 1.0);
                            let kx = row.slider_left + t * (row.slider_right - row.slider_left);
                            let track_bg = D2D1_ROUNDED_RECT {
                                rect: D2D_RECT_F { left: row.slider_left, top: track_y - 2.0, right: row.slider_right, bottom: track_y + 2.0 },
                                radiusX: 2.0, radiusY: 2.0,
                            };
                            rt.FillRoundedRectangle(&track_bg, &b_line);
                            let track_fg = D2D1_ROUNDED_RECT {
                                rect: D2D_RECT_F { left: row.slider_left, top: track_y - 2.0, right: kx, bottom: track_y + 2.0 },
                                radiusX: 2.0, radiusY: 2.0,
                            };
                            rt.FillRoundedRectangle(&track_fg, if dim { &b_muted } else { &b_accent });
                            let knob = D2D1_ELLIPSE { point: Vector2 { X: kx, Y: track_y }, radiusX: 7.5, radiusY: 7.5 };
                            rt.FillEllipse(&knob, if dim { &b_muted } else { &b_accent });
                            rt.DrawEllipse(&knob, &b_card, 2.0, None);
                        }
                    }

                    if !meta.hint.is_empty() {
                        let hy = if meta.kind == ParamKind::Range { top + 52.0 } else { top + 32.0 };
                        let hint_rc = D2D_RECT_F { left: inner_l, top: hy, right: inner_r, bottom: hy + row.hint_h };
                        draw_text(&rt, meta.hint, &self.fmt_hint, &hint_rc, &b_muted);
                    }
                }

                rt.PopAxisAlignedClip();

                // Sticky bottom action bar
                let footer_rc = D2D_RECT_F { left: 0.0, top: scroll_h, right: LOGICAL_W, bottom: view_h };
                rt.FillRectangle(&footer_rc, &b_footer);
                rt.DrawLine(
                    Vector2 { X: 0.0, Y: scroll_h },
                    Vector2 { X: LOGICAL_W, Y: scroll_h },
                    &b_line,
                    1.0,
                    None,
                );

                let base_labels = ["让它生气试试", "让它好奇试试", "恢复默认"];
                let done_labels = ["已触发", "已触发", "已恢复"];
                for (i, r) in btns.iter().enumerate() {
                    let is_feedback = feedback_idx == Some(i);
                    let is_pressed = self.pressed_btn == Some(i);
                    let is_hover = self.hover_btn == Some(i);

                    let rr = D2D1_ROUNDED_RECT { rect: *r, radiusX: 8.0, radiusY: 8.0 };
                    let fill_brush = if is_feedback {
                        &b_accent
                    } else if is_pressed {
                        &b_pressed
                    } else if is_hover {
                        &b_hover
                    } else {
                        &b_card
                    };
                    let border_brush = if is_feedback || is_hover { &b_accent } else { &b_line };
                    let text_brush = if is_feedback {
                        &b_accent_txt
                    } else if i == 2 {
                        &b_muted
                    } else {
                        &b_text
                    };
                    let label = if is_feedback { done_labels[i] } else { base_labels[i] };

                    rt.FillRoundedRectangle(&rr, fill_brush);
                    rt.DrawRoundedRectangle(&rr, border_brush, 1.0, None);
                    draw_text(&rt, label, &self.fmt_btn, r, text_brush);
                }

                if rt.EndDraw(None, None).is_err() {
                    self.rt = None;
                }
            }
        }

        fn client_logical(&self, lparam: LPARAM) -> (f32, f32) {
            let px = (lparam.0 & 0xffff) as i16 as f32;
            let py = ((lparam.0 >> 16) & 0xffff) as i16 as f32;
            let dpi = unsafe { GetDpiForWindow(self.hwnd).max(96) } as f32;
            let scale = dpi / 96.0;
            (px / scale, py / scale)
        }

        fn hit_footer_btn(&self, cx: f32, cy: f32) -> Option<usize> {
            let btns = self.footer_btn_rects();
            for (i, r) in btns.iter().enumerate() {
                if cx >= r.left && cx <= r.right && cy >= r.top && cy <= r.bottom {
                    return Some(i);
                }
            }
            None
        }

        fn hit_interactive_row(&self, cx: f32, cy: f32) -> bool {
            if cy >= self.scroll_view_height() {
                return false;
            }
            let y = cy + self.scroll_y;
            for row in &self.rows {
                if y >= row.top && y <= row.top + row.height {
                    let meta = &TUNING_SCHEMA[row.index];
                    return match meta.kind {
                        ParamKind::Toggle => cx >= PAD_X && cx <= LOGICAL_W - PAD_X,
                        ParamKind::Range => {
                            self.row_enabled(row.index)
                                && (y - row.slider_y).abs() <= 16.0
                                && cx >= row.slider_left - 12.0
                                && cx <= row.slider_right + 12.0
                        }
                    };
                }
            }
            false
        }

        fn update_slider(&mut self, row_idx: usize, x: f32) {
            let row = self.rows[row_idx];
            if !self.row_enabled(row.index) {
                return;
            }
            let meta = &TUNING_SCHEMA[row.index];
            let t = ((x - row.slider_left) / (row.slider_right - row.slider_left)).clamp(0.0, 1.0);
            let raw = meta.min + t * (meta.max - meta.min);
            let snapped = ((raw - meta.min) / meta.step).round() * meta.step + meta.min;
            let clamped = snapped.clamp(meta.min, meta.max);
            if (self.tuning.get_value(meta.key) - clamped).abs() > 1e-4 {
                self.tuning.set_value(meta.key, clamped);
                unsafe {
                    let _ = PostMessageW(Some(self.owner), WM_SETTINGS_CHANGED, WPARAM(row.index), LPARAM(0));
                    let _ = InvalidateRect(Some(self.hwnd), None, false);
                }
            }
        }

        fn trigger_button(&mut self, idx: usize) {
            match idx {
                0 => unsafe { let _ = PostMessageW(Some(self.owner), WM_SETTINGS_TEST_ANNOY, WPARAM(0), LPARAM(0)); },
                1 => unsafe { let _ = PostMessageW(Some(self.owner), WM_SETTINGS_TEST_CURIOUS, WPARAM(0), LPARAM(0)); },
                2 => {
                    self.tuning = Tuning::default();
                    unsafe { let _ = PostMessageW(Some(self.owner), WM_SETTINGS_RESET, WPARAM(0), LPARAM(0)); }
                }
                _ => return,
            }
            self.feedback_btn = Some((idx, Instant::now() + Duration::from_millis(u64::from(FEEDBACK_MS))));
            unsafe {
                let _ = SetTimer(Some(self.hwnd), TIMER_FEEDBACK, FEEDBACK_MS, None);
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }

        fn on_mouse_down(&mut self, lparam: LPARAM) {
            let (cx, cy) = self.client_logical(lparam);
            if let Some(btn) = self.hit_footer_btn(cx, cy) {
                self.pressed_btn = Some(btn);
                unsafe {
                    SetCapture(self.hwnd);
                    let _ = InvalidateRect(Some(self.hwnd), None, false);
                }
                return;
            }
            if cy >= self.scroll_view_height() {
                return;
            }
            let y = cy + self.scroll_y;
            for ri in 0..self.rows.len() {
                let row = self.rows[ri];
                if y >= row.top && y <= row.top + row.height {
                    let meta = &TUNING_SCHEMA[row.index];
                    match meta.kind {
                        ParamKind::Toggle => {
                            let cur = self.tuning.get_value(meta.key);
                            self.tuning.set_value(meta.key, if cur >= 0.5 { 0.0 } else { 1.0 });
                            unsafe {
                                let _ = PostMessageW(Some(self.owner), WM_SETTINGS_CHANGED, WPARAM(row.index), LPARAM(0));
                                let _ = InvalidateRect(Some(self.hwnd), None, false);
                            }
                        }
                        ParamKind::Range => {
                            if self.row_enabled(row.index)
                                && (y - row.slider_y).abs() <= 16.0
                                && cx >= row.slider_left - 12.0
                                && cx <= row.slider_right + 12.0
                            {
                                self.dragging_row = Some(ri);
                                unsafe { SetCapture(self.hwnd); }
                                self.update_slider(ri, cx);
                            }
                        }
                    }
                    return;
                }
            }
        }

        fn on_mouse_move(&mut self, lparam: LPARAM) {
            if !self.tracking_mouse {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: self.hwnd,
                    dwHoverTime: 0,
                };
                unsafe { let _ = TrackMouseEvent(&mut tme); }
                self.tracking_mouse = true;
            }
            let (cx, cy) = self.client_logical(lparam);
            if let Some(ri) = self.dragging_row {
                self.update_slider(ri, cx);
                return;
            }
            let next_btn = self.hit_footer_btn(cx, cy);
            let next_interactive = next_btn.is_some() || self.hit_interactive_row(cx, cy);
            self.hover_interactive = next_interactive;
            if next_btn != self.hover_btn {
                self.hover_btn = next_btn;
                unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
            }
        }

        fn on_mouse_leave(&mut self) {
            self.tracking_mouse = false;
            self.hover_interactive = false;
            if self.hover_btn.take().is_some() {
                unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
            }
        }

        fn on_mouse_up(&mut self, lparam: LPARAM) {
            if self.dragging_row.take().is_some() {
                unsafe { let _ = ReleaseCapture(); }
            }
            if let Some(pressed) = self.pressed_btn.take() {
                unsafe { let _ = ReleaseCapture(); }
                let (cx, cy) = self.client_logical(lparam);
                if self.hit_footer_btn(cx, cy) == Some(pressed) {
                    self.trigger_button(pressed);
                } else {
                    unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                }
            }
        }

        fn scroll_by(&mut self, dy: f32) {
            let next = (self.scroll_y + dy).clamp(0.0, self.max_scroll());
            if (next - self.scroll_y).abs() > 0.1 {
                self.scroll_y = next;
                unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
            }
        }

        fn on_wheel(&mut self, wparam: WPARAM) {
            let delta = ((wparam.0 >> 16) as u16 as i16) as f32;
            self.scroll_by(-delta * 0.45);
        }

        fn on_key_down(&mut self, wparam: WPARAM) {
            let vk = wparam.0 as u16;
            if vk == VK_ESCAPE.0 {
                unsafe { let _ = DestroyWindow(self.hwnd); }
            } else if vk == VK_DOWN.0 {
                self.scroll_by(48.0);
            } else if vk == VK_UP.0 {
                self.scroll_by(-48.0);
            } else if vk == VK_NEXT.0 {
                self.scroll_by(self.scroll_view_height() * 0.8);
            } else if vk == VK_PRIOR.0 {
                self.scroll_by(-self.scroll_view_height() * 0.8);
            }
        }
    }

    unsafe fn draw_text(
        rt: &ID2D1HwndRenderTarget,
        text: &str,
        fmt: &IDWriteTextFormat,
        rect: &D2D_RECT_F,
        brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
    ) {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        rt.DrawText(
            &utf16,
            fmt,
            rect,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.paint();
                        }
                    }
                });
                let _ = ValidateRect(Some(hwnd), None);
                LRESULT(0)
            }
            WM_SIZE => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            let w = (lparam.0 & 0xffff) as u32;
                            let h = ((lparam.0 >> 16) & 0xffff) as u32;
                            if let Some(ref rt) = inner.rt {
                                let _ = rt.Resize(&D2D_SIZE_U { width: w.max(1), height: h.max(1) });
                            }
                            inner.scroll_y = inner.scroll_y.clamp(0.0, inner.max_scroll());
                        }
                    }
                });
                LRESULT(0)
            }
            WM_SETCURSOR => {
                let hit_client = (lparam.0 & 0xffff) == 1;
                if hit_client {
                    let hand = STATE.with(|s| {
                        s.try_borrow()
                            .ok()
                            .and_then(|b| b.as_ref().map(|i| i.hover_interactive || i.dragging_row.is_some()))
                            .unwrap_or(false)
                    });
                    if let Ok(cur) = LoadCursorW(None, if hand { IDC_HAND } else { IDC_ARROW }) {
                        SetCursor(Some(cur));
                        return LRESULT(1);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_SETTINGCHANGE => {
                apply_titlebar_theme(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_mouse_down(lparam);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_mouse_move(lparam);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_MOUSELEAVE => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_mouse_leave();
                        }
                    }
                });
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_mouse_up(lparam);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_wheel(wparam);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_KEYDOWN => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_key_down(wparam);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_TIMER => {
                if wparam.0 == TIMER_FEEDBACK {
                    let _ = KillTimer(Some(hwnd), TIMER_FEEDBACK);
                    STATE.with(|s| {
                        if let Ok(mut b) = s.try_borrow_mut() {
                            if let Some(ref mut inner) = *b {
                                inner.feedback_btn = None;
                            }
                        }
                    });
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = KillTimer(Some(hwnd), TIMER_FEEDBACK);
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        *b = None;
                    }
                });
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(windows)]
pub use imp::*;
