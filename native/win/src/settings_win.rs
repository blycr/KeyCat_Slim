//! Native Direct2D + DirectWrite "调节反应…" settings window (zero WebView2).
//! Mirrors `web/settings.html` styling (light/dark system theme, rounded cards, accent sliders,
//! toggle checkboxes, test buttons and reset) and releases all GPU/D2D resources on close.

#[cfg(windows)]
mod imp {
    use std::cell::RefCell;

    use keycat_core::tuning::{ParamKind, Tuning, GROUPS, TUNING_SCHEMA};
    use windows::core::{w, Result};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F, D2D_SIZE_U};
    use windows::Win32::Graphics::Direct2D::{
        D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, D2D1_DRAW_TEXT_OPTIONS_NONE,
        D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES,
        D2D1_RENDER_TARGET_PROPERTIES, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
    };
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
        DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    };
    use windows::Win32::Graphics::Gdi::{InvalidateRect, ValidateRect};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
    use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics,
        LoadCursorW, PostMessageW, RegisterClassW, SetForegroundWindow, ShowWindow, CS_HREDRAW,
        CS_VREDRAW, IDC_ARROW, SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNORMAL, WINDOW_EX_STYLE, WM_APP,
        WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
        WM_SIZE, WNDCLASSW, WS_CAPTION, WS_MINIMIZEBOX, WS_SYSMENU, WS_VISIBLE,
    };
    use windows_numerics::Vector2;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    pub const WM_SETTINGS_CHANGED: u32 = WM_APP + 10;
    pub const WM_SETTINGS_RESET: u32 = WM_APP + 11;
    pub const WM_SETTINGS_TEST_ANNOY: u32 = WM_APP + 12;
    pub const WM_SETTINGS_TEST_CURIOUS: u32 = WM_APP + 13;

    const LOGICAL_W: f32 = 420.0;
    const PAD_X: f32 = 20.0;

    #[derive(Clone, Copy)]
    struct Palette {
        bg: D2D1_COLOR_F,
        card: D2D1_COLOR_F,
        text: D2D1_COLOR_F,
        muted: D2D1_COLOR_F,
        line: D2D1_COLOR_F,
        accent: D2D1_COLOR_F,
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

    fn palette() -> Palette {
        if detect_dark_mode() {
            Palette {
                bg: hex(0x23211d, 1.0),
                card: hex(0x2d2a25, 1.0),
                text: hex(0xeee8da, 1.0),
                muted: hex(0xa59d8d, 1.0),
                line: hex(0x3d3932, 1.0),
                accent: hex(0xf0a35a, 1.0),
            }
        } else {
            Palette {
                bg: hex(0xf7f2e4, 1.0),
                card: hex(0xfffdf7, 1.0),
                text: hex(0x2a2722, 1.0),
                muted: hex(0x8a8274, 1.0),
                line: hex(0xe6dfcd, 1.0),
                accent: hex(0xe08a3c, 1.0),
            }
        }
    }

    #[derive(Clone, Copy)]
    struct RowLayout {
        index: usize,
        top: f32,
        height: f32,
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
        fmt_heading: IDWriteTextFormat,
        fmt_label: IDWriteTextFormat,
        fmt_value: IDWriteTextFormat,
        fmt_hint: IDWriteTextFormat,
        fmt_btn: IDWriteTextFormat,
        tuning: Tuning,
        scroll_y: f32,
        content_h: f32,
        rows: Vec<RowLayout>,
        btn_rects: [D2D_RECT_F; 3],
        dragging_row: Option<usize>,
    }

    thread_local! {
        static STATE: RefCell<Option<Inner>> = const { RefCell::new(None) };
    }

    pub fn open_or_focus(owner: HWND, tuning: Tuning) {
        let existing = STATE.with(|s| s.borrow().as_ref().map(|i| i.hwnd));
        if let Some(hwnd) = existing {
            STATE.with(|s| {
                if let Some(ref mut inner) = *s.borrow_mut() {
                    inner.tuning = tuning;
                }
            });
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
        STATE.with(|s| s.borrow().as_ref().map(|i| i.tuning))
    }

    fn create_window(owner: HWND, tuning: Tuning) -> Result<()> {
        unsafe {
            let hinst = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
            let cls = w!("KeyCatNativeSettings");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinst.unwrap_or_default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: cls,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let dpi = GetDpiForWindow(owner).max(96);
            let scale = dpi as f32 / 96.0;
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let logical_h = ((screen_h as f32 / scale) - 90.0).min(820.0).max(480.0);

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

            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

            let make_fmt = |size: f32, bold: bool, align| -> Result<IDWriteTextFormat> {
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
                f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
                Ok(f)
            };

            let fmt_heading = make_fmt(13.0, true, DWRITE_TEXT_ALIGNMENT_LEADING)?;
            let fmt_label = make_fmt(13.0, false, DWRITE_TEXT_ALIGNMENT_LEADING)?;
            let fmt_value = make_fmt(13.0, true, DWRITE_TEXT_ALIGNMENT_TRAILING)?;
            let fmt_hint = make_fmt(11.5, false, DWRITE_TEXT_ALIGNMENT_LEADING)?;
            let fmt_btn = make_fmt(13.0, false, DWRITE_TEXT_ALIGNMENT_CENTER)?;

            let mut inner = Inner {
                hwnd,
                owner,
                d2d,
                _dwrite: dwrite,
                rt: None,
                fmt_heading,
                fmt_label,
                fmt_value,
                fmt_hint,
                fmt_btn,
                tuning,
                scroll_y: 0.0,
                content_h: 0.0,
                rows: Vec::new(),
                btn_rects: [D2D_RECT_F::default(); 3],
                dragging_row: None,
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

        fn compute_layout(&mut self) {
            self.rows.clear();
            let mut y = 16.0f32;
            let card_left = PAD_X;
            let card_right = LOGICAL_W - PAD_X;
            let inner_l = card_left + 14.0;
            let inner_r = card_right - 14.0;

            for (gi, &group) in GROUPS.iter().enumerate() {
                if gi > 0 {
                    y += 14.0;
                }
                y += 24.0; // group title
                y += 4.0;  // card top pad
                for (idx, meta) in TUNING_SCHEMA.iter().enumerate() {
                    if meta.group != group {
                        continue;
                    }
                    let row_top = y;
                    let has_hint = !meta.hint.is_empty();
                    let h = match (meta.kind, has_hint) {
                        (ParamKind::Toggle, true) => 54.0,
                        (ParamKind::Toggle, false) => 38.0,
                        (ParamKind::Range, true) => 78.0,
                        (ParamKind::Range, false) => 60.0,
                    };
                    let toggle_rect = D2D_RECT_F {
                        left: inner_r - 36.0,
                        top: row_top + 9.0,
                        right: inner_r,
                        bottom: row_top + 29.0,
                    };
                    self.rows.push(RowLayout {
                        index: idx,
                        top: row_top,
                        height: h,
                        slider_y: row_top + 40.0,
                        slider_left: inner_l + 8.0,
                        slider_right: inner_r - 8.0,
                        toggle_rect,
                    });
                    y += h;
                }
                y += 4.0; // card bottom pad
            }

            y += 16.0;
            let gap = 8.0;
            let total_w = card_right - card_left;
            let reset_w = 88.0;
            let test_w = (total_w - reset_w - gap * 2.0) * 0.5;
            let bh = 34.0;
            self.btn_rects = [
                D2D_RECT_F { left: card_left, top: y, right: card_left + test_w, bottom: y + bh },
                D2D_RECT_F { left: card_left + test_w + gap, top: y, right: card_left + test_w * 2.0 + gap, bottom: y + bh },
                D2D_RECT_F { left: card_right - reset_w, top: y, right: card_right, bottom: y + bh },
            ];
            self.content_h = y + bh + 20.0;
        }

        fn max_scroll(&self) -> f32 {
            let mut rc = RECT::default();
            unsafe { let _ = GetClientRect(self.hwnd, &mut rc); }
            let dpi = unsafe { GetDpiForWindow(self.hwnd).max(96) } as f32;
            let view_h = (rc.bottom - rc.top) as f32 * 96.0 / dpi;
            (self.content_h - view_h).max(0.0)
        }

        fn paint(&mut self) {
            if self.ensure_rt().is_err() {
                return;
            }
            let pal = palette();
            let Some(rt) = self.rt.clone() else { return };
            unsafe {
                rt.BeginDraw();
                rt.Clear(Some(&pal.bg));
                let Ok(b_card) = rt.CreateSolidColorBrush(&pal.card, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_line) = rt.CreateSolidColorBrush(&pal.line, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_text) = rt.CreateSolidColorBrush(&pal.text, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_muted) = rt.CreateSolidColorBrush(&pal.muted, None) else { let _ = rt.EndDraw(None, None); return; };
                let Ok(b_accent) = rt.CreateSolidColorBrush(&pal.accent, None) else { let _ = rt.EndDraw(None, None); return; };

                let sy = self.scroll_y;
                let card_l = PAD_X;
                let card_r = LOGICAL_W - PAD_X;
                let inner_l = card_l + 14.0;
                let inner_r = card_r - 14.0;

                // Draw group headers and card backgrounds
                let mut cur_y = 16.0f32;
                for (gi, &group) in GROUPS.iter().enumerate() {
                    if gi > 0 {
                        cur_y += 14.0;
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

                // Draw rows
                for (ri, row) in self.rows.iter().enumerate() {
                    let meta = &TUNING_SCHEMA[row.index];
                    let group_on = if meta.group == GROUPS[0] {
                        self.tuning.annoy_on
                    } else {
                        self.tuning.curious_on
                    };
                    let dim = meta.kind == ParamKind::Range && !group_on;
                    let label_brush = if dim { &b_muted } else { &b_text };
                    let top = row.top - sy;

                    // Separator line except first row of each group
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
                        right: inner_r - 80.0,
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
                                left: inner_r - 90.0,
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
                        let hy = if meta.kind == ParamKind::Range { top + 52.0 } else { top + 30.0 };
                        let hint_rc = D2D_RECT_F { left: inner_l, top: hy, right: inner_r, bottom: hy + 20.0 };
                        draw_text(&rt, meta.hint, &self.fmt_hint, &hint_rc, &b_muted);
                    }
                }

                // Bottom action buttons
                let labels = ["让它生气试试", "让它好奇试试", "恢复默认"];
                for (i, r) in self.btn_rects.iter().enumerate() {
                    let dr = D2D_RECT_F { left: r.left, top: r.top - sy, right: r.right, bottom: r.bottom - sy };
                    let rr = D2D1_ROUNDED_RECT { rect: dr, radiusX: 7.0, radiusY: 7.0 };
                    rt.FillRoundedRectangle(&rr, &b_card);
                    rt.DrawRoundedRectangle(&rr, &b_line, 1.0, None);
                    draw_text(&rt, labels[i], &self.fmt_btn, &dr, if i == 2 { &b_muted } else { &b_text });
                }

                if rt.EndDraw(None, None).is_err() {
                    self.rt = None;
                }
            }
        }

        fn to_logical(&self, lparam: LPARAM) -> (f32, f32) {
            let px = (lparam.0 & 0xffff) as i16 as f32;
            let py = ((lparam.0 >> 16) & 0xffff) as i16 as f32;
            let dpi = unsafe { GetDpiForWindow(self.hwnd).max(96) } as f32;
            let scale = dpi / 96.0;
            (px / scale, py / scale + self.scroll_y)
        }

        fn update_slider(&mut self, row_idx: usize, x: f32) {
            let row = self.rows[row_idx];
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

        fn on_mouse_down(&mut self, lparam: LPARAM) {
            let (x, y) = self.to_logical(lparam);
            // Check buttons
            for (i, r) in self.btn_rects.iter().enumerate() {
                if x >= r.left && x <= r.right && y >= r.top && y <= r.bottom {
                    match i {
                        0 => unsafe { let _ = PostMessageW(Some(self.owner), WM_SETTINGS_TEST_ANNOY, WPARAM(0), LPARAM(0)); },
                        1 => unsafe { let _ = PostMessageW(Some(self.owner), WM_SETTINGS_TEST_CURIOUS, WPARAM(0), LPARAM(0)); },
                        2 => {
                            self.tuning = Tuning::default();
                            unsafe {
                                let _ = PostMessageW(Some(self.owner), WM_SETTINGS_RESET, WPARAM(0), LPARAM(0));
                                let _ = InvalidateRect(Some(self.hwnd), None, false);
                            }
                        }
                        _ => {}
                    }
                    return;
                }
            }
            // Check rows
            for ri in 0..self.rows.len() {
                let row = self.rows[ri];
                let meta = &TUNING_SCHEMA[row.index];
                if y >= row.top && y <= row.top + row.height {
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
                            if (y - row.slider_y).abs() <= 16.0 && x >= row.slider_left - 12.0 && x <= row.slider_right + 12.0 {
                                self.dragging_row = Some(ri);
                                unsafe { SetCapture(self.hwnd); }
                                self.update_slider(ri, x);
                            }
                        }
                    }
                    return;
                }
            }
        }

        fn on_mouse_move(&mut self, lparam: LPARAM) {
            if let Some(ri) = self.dragging_row {
                let (x, _) = self.to_logical(lparam);
                self.update_slider(ri, x);
            }
        }

        fn on_mouse_up(&mut self) {
            if self.dragging_row.take().is_some() {
                unsafe { let _ = ReleaseCapture(); }
            }
        }

        fn on_wheel(&mut self, wparam: WPARAM) {
            let delta = ((wparam.0 >> 16) as u16 as i16) as f32;
            self.scroll_y = (self.scroll_y - delta * 0.4).clamp(0.0, self.max_scroll());
            unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
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
            WM_LBUTTONUP => {
                STATE.with(|s| {
                    if let Ok(mut b) = s.try_borrow_mut() {
                        if let Some(ref mut inner) = *b {
                            inner.on_mouse_up();
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
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                // Reclaim Direct2D/DirectWrite resources completely on close.
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
