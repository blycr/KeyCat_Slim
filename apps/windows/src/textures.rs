//! Direct2D + DirectWrite offscreen texture baker for keycap labels, sleep "Z" and FX stickers.
//! Bakes directly onto D3D11 BGRA textures (premultiplied alpha) and caches them forever by key.

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;
    use std::f32::consts::TAU;

    use keycat_core::behavior::SpriteKey;
    use windows::core::{w, Interface, Result};
    use windows::Win32::Graphics::Direct2D::Common::{
        D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
        D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN,
        D2D1_PIXEL_FORMAT, D2D_RECT_F,
    };
    use windows::Win32::Graphics::Direct2D::{
        D2D1CreateFactory, ID2D1Factory, ID2D1RenderTarget, D2D1_CAP_STYLE_ROUND,
        D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
        D2D1_LINE_JOIN_ROUND, D2D1_QUADRATIC_BEZIER_SEGMENT, D2D1_RENDER_TARGET_PROPERTIES,
        D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_ROUNDED_RECT, D2D1_STROKE_STYLE_PROPERTIES,
        D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
    };
    use windows::Win32::Graphics::Direct3D11::{
        ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_RENDER_TARGET,
        D3D11_BIND_SHADER_RESOURCE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    };
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
        DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
        DWRITE_TEXT_ALIGNMENT_CENTER,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
    use windows::Win32::Graphics::Dxgi::IDXGISurface;
    use windows_numerics::Vector2;

    pub struct SpriteAtlas {
        d2d: ID2D1Factory,
        dwrite: IDWriteFactory,
        cache: HashMap<SpriteKey, ID3D11ShaderResourceView>,
    }

    fn rgba(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
        D2D1_COLOR_F {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    fn pt(x: f32, y: f32) -> Vector2 {
        Vector2 { X: x, Y: y }
    }

    impl SpriteAtlas {
        pub fn new() -> Result<Self> {
            let d2d: ID2D1Factory =
                unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
            let dwrite: IDWriteFactory =
                unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
            Ok(Self { d2d, dwrite, cache: HashMap::new() })
        }

        pub fn get_or_bake(
            &mut self,
            device: &ID3D11Device,
            key: &SpriteKey,
        ) -> Option<&ID3D11ShaderResourceView> {
            if !self.cache.contains_key(key) {
                if let Ok(srv) = self.bake(device, key) {
                    self.cache.insert(key.clone(), srv);
                }
            }
            self.cache.get(key)
        }

        fn bake(&self, device: &ID3D11Device, key: &SpriteKey) -> Result<ID3D11ShaderResourceView> {
            let wide = matches!(key, SpriteKey::Cap(t) if t.chars().count() > 1);
            let w: u32 = if wide { 256 } else { 128 };
            let h: u32 = 128;

            let desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut tex: Option<ID3D11Texture2D> = None;
            unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex))? };
            let tex = tex.unwrap();
            let surface: IDXGISurface = tex.cast()?;

            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            };
            let rt: ID2D1RenderTarget =
                unsafe { self.d2d.CreateDxgiSurfaceRenderTarget(&surface, &props)? };

            unsafe {
                rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
                rt.BeginDraw();
                rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
                match key {
                    SpriteKey::Cap(label) => self.draw_cap(&rt, w as f32, label)?,
                    SpriteKey::SleepZ => self.draw_sleep_z(&rt)?,
                    SpriteKey::Anger => self.draw_anger(&rt)?,
                    SpriteKey::Bang => self.draw_bang(&rt)?,
                    SpriteKey::Dust => self.draw_dust(&rt)?,
                }
                rt.EndDraw(None, None)?;
            }

            let mut srv: Option<ID3D11ShaderResourceView> = None;
            unsafe { device.CreateShaderResourceView(&tex, None, Some(&mut srv))? };
            Ok(srv.unwrap())
        }

        unsafe fn draw_cap(&self, rt: &ID2D1RenderTarget, w: f32, label: &str) -> Result<()> {
            let shadow = rt.CreateSolidColorBrush(&rgba(40, 40, 40, 0.55), None)?;
            let fill = rt.CreateSolidColorBrush(&rgba(0xfb, 0xfa, 0xf3, 1.0), None)?;
            let stroke = rt.CreateSolidColorBrush(&rgba(0x2a, 0x2a, 0x2a, 1.0), None)?;
            let ink = rt.CreateSolidColorBrush(&rgba(0x1b, 0x1b, 0x1b, 1.0), None)?;

            let x = 8.0f32;
            let y = 8.0f32;
            let rw = w - 16.0;
            let rh = 96.0f32;
            let r = 26.0f32;

            let rr_shadow = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: x, top: y + 12.0, right: x + rw, bottom: y + 12.0 + rh },
                radiusX: r,
                radiusY: r,
            };
            rt.FillRoundedRectangle(&rr_shadow, &shadow);

            let rr = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: x, top: y, right: x + rw, bottom: y + rh },
                radiusX: r,
                radiusY: r,
            };
            rt.FillRoundedRectangle(&rr, &fill);
            rt.DrawRoundedRectangle(&rr, &stroke, 5.0, None);

            let wide = label.chars().count() > 1;
            let font_size = if wide { 44.0 } else { 58.0 };
            let fmt = self.dwrite.CreateTextFormat(
                w!("Segoe UI Symbol"),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_size,
                w!("en-US"),
            )?;
            fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let utf16: Vec<u16> = label.encode_utf16().collect();
            let rect = D2D_RECT_F { left: x, top: y + 2.0, right: x + rw, bottom: y + rh + 2.0 };
            rt.DrawText(
                &utf16,
                &fmt,
                &rect,
                &ink,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
            Ok(())
        }

        unsafe fn draw_sleep_z(&self, rt: &ID2D1RenderTarget) -> Result<()> {
            let stroke = rt.CreateSolidColorBrush(&rgba(0x1b, 0x1b, 0x1b, 1.0), None)?;
            let fill = rt.CreateSolidColorBrush(&rgba(0xfb, 0xfa, 0xf3, 1.0), None)?;
            let fmt = self.dwrite.CreateTextFormat(
                w!("Comic Sans MS"),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                80.0,
                w!("en-US"),
            )?;
            fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let text: [u16; 1] = ['Z' as u16];
            // Thick crisp outline via 12-direction offset halo, then cream fill.
            let r = 6.0f32;
            for i in 0..12 {
                let a = (i as f32 / 12.0) * TAU;
                let (sa, ca) = a.sin_cos();
                let rect = D2D_RECT_F {
                    left: ca * r,
                    top: 2.0 + sa * r,
                    right: 128.0 + ca * r,
                    bottom: 130.0 + sa * r,
                };
                rt.DrawText(
                    &text,
                    &fmt,
                    &rect,
                    &stroke,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            let rect = D2D_RECT_F { left: 0.0, top: 2.0, right: 128.0, bottom: 130.0 };
            rt.DrawText(
                &text,
                &fmt,
                &rect,
                &fill,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
            Ok(())
        }

        unsafe fn draw_anger(&self, rt: &ID2D1RenderTarget) -> Result<()> {
            let white = rt.CreateSolidColorBrush(&rgba(0xfb, 0xfa, 0xf3, 1.0), None)?;
            let red = rt.CreateSolidColorBrush(&rgba(0xe2, 0x48, 0x3d, 1.0), None)?;
            let style = self.d2d.CreateStrokeStyle(
                &D2D1_STROKE_STYLE_PROPERTIES {
                    startCap: D2D1_CAP_STYLE_ROUND,
                    endCap: D2D1_CAP_STYLE_ROUND,
                    lineJoin: D2D1_LINE_JOIN_ROUND,
                    ..Default::default()
                },
                None,
            )?;
            let geo = self.d2d.CreatePathGeometry()?;
            let sink = geo.Open()?;
            for &(sx, sy) in &[(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                sink.BeginFigure(pt(64.0 + sx * 14.0, 64.0 + sy * 46.0), D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                    point1: pt(64.0 + sx * 14.0, 64.0 + sy * 14.0),
                    point2: pt(64.0 + sx * 46.0, 64.0 + sy * 14.0),
                });
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
            }
            sink.Close()?;
            rt.DrawGeometry(&geo, &white, 30.0, &style);
            rt.DrawGeometry(&geo, &red, 15.0, &style);
            Ok(())
        }

        unsafe fn draw_bang(&self, rt: &ID2D1RenderTarget) -> Result<()> {
            let fill = rt.CreateSolidColorBrush(&rgba(0xfb, 0xfa, 0xf3, 1.0), None)?;
            let stroke = rt.CreateSolidColorBrush(&rgba(0x1d, 0x1d, 0x1b, 1.0), None)?;
            let style = self.d2d.CreateStrokeStyle(
                &D2D1_STROKE_STYLE_PROPERTIES {
                    startCap: D2D1_CAP_STYLE_ROUND,
                    endCap: D2D1_CAP_STYLE_ROUND,
                    lineJoin: D2D1_LINE_JOIN_ROUND,
                    ..Default::default()
                },
                None,
            )?;
            let geo = self.d2d.CreatePathGeometry()?;
            let sink = geo.Open()?;
            for i in 0..20 {
                let a = (i as f32 / 20.0) * TAU;
                let r = if i % 2 == 1 { 26.0 } else { 58.0 };
                let p = pt(64.0 + a.cos() * r, 64.0 + a.sin() * r);
                if i == 0 {
                    sink.BeginFigure(p, D2D1_FIGURE_BEGIN_FILLED);
                } else {
                    sink.AddLine(p);
                }
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            rt.FillGeometry(&geo, &fill, None);
            rt.DrawGeometry(&geo, &stroke, 7.0, &style);
            Ok(())
        }

        unsafe fn draw_dust(&self, rt: &ID2D1RenderTarget) -> Result<()> {
            let fill = rt.CreateSolidColorBrush(&rgba(0xf4, 0xf1, 0xe6, 1.0), None)?;
            let stroke = rt.CreateSolidColorBrush(&rgba(0xc9, 0xc4, 0xb4, 1.0), None)?;
            let e = D2D1_ELLIPSE { point: pt(64.0, 64.0), radiusX: 52.0, radiusY: 52.0 };
            rt.FillEllipse(&e, &fill);
            rt.DrawEllipse(&e, &stroke, 6.0, None);
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use imp::SpriteAtlas;
