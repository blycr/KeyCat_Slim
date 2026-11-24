//! Direct3D 11 (4x MSAA) + DXGI Premultiplied Alpha + DirectComposition renderer.
//! Renders the entire keycat-core 3D scene directly onto a WS_EX_NOREDIRECTIONBITMAP window
//! with zero WebView2 / Chromium overhead.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::mem;
    use std::ptr;

    use keycat_core::behavior::Scene;
    use keycat_core::cat::{COLLAR_LIFT, HEAD_R};
    use keycat_core::geometry::{MeshData, Vertex};
    use keycat_core::keyboard::KeyInstance;
    use keycat_core::life::Festival;
    use keycat_core::math::{hex_color, Mat4, Vec3};
    use windows::core::{s, w, Error, Interface, Result, BOOL, HRESULT, PCSTR};
    use windows::Win32::Foundation::{FreeLibrary, HMODULE, HWND};
    use windows::Win32::Graphics::Direct3D::{
        ID3DBlob, D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0,
        D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
    };
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11BlendState, ID3D11Buffer, ID3D11DepthStencilState,
        ID3D11DepthStencilView, ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout,
        ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState,
        ID3D11Texture2D, ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_DEPTH_STENCIL,
        D3D11_BIND_INDEX_BUFFER, D3D11_BIND_RENDER_TARGET, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
        D3D11_BUFFER_DESC, D3D11_CLEAR_DEPTH, D3D11_COLOR_WRITE_ENABLE_ALL,
        D3D11_COMPARISON_LESS_EQUAL, D3D11_CPU_ACCESS_WRITE, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_CULL_BACK, D3D11_CULL_FRONT, D3D11_CULL_NONE, D3D11_DEPTH_STENCIL_DESC,
        D3D11_DEPTH_WRITE_MASK_ALL, D3D11_DEPTH_WRITE_MASK_ZERO, D3D11_FILL_SOLID,
        D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_INSTANCE_DATA,
        D3D11_INPUT_PER_VERTEX_DATA, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_WRITE_DISCARD,
        D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC,
        D3D11_SDK_VERSION, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC,
        D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC,
        D3D11_USAGE_IMMUTABLE, D3D11_VIEWPORT,
    };
    use windows::Win32::Graphics::DirectComposition::{
        DCompositionCreateDevice, IDCompositionDevice, IDCompositionEffectGroup,
        IDCompositionTarget, IDCompositionVisual,
    };
    use windows::Win32::Graphics::Dxgi::Common::{
        DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_D24_UNORM_S8_UINT,
        DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32_FLOAT,
        DXGI_FORMAT_R32_UINT, DXGI_SAMPLE_DESC,
    };
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory2, IDXGIDevice, IDXGIDevice3, IDXGIFactory2, IDXGISwapChain1,
        DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
        DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
    };

    use crate::textures::SpriteAtlas;

    const SHADERS_SRC: &str = include_str!("shaders.hlsl");

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FrameCB {
        view: [[f32; 4]; 4],
        proj: [[f32; 4]; 4],
        bend: [f32; 4],
        cam_right: [f32; 4],
        cam_up: [f32; 4],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FurCB {
        fur: [f32; 4],
        light: [f32; 4],
        dark: [f32; 4],
        patch_a: [f32; 4],
        patch_b: [f32; 4],
        point: [f32; 4],
        inner: [f32; 4],
        w0: [f32; 4],
        w1: [f32; 4],
        w2: [f32; 4],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ObjCB {
        model: [[f32; 4]; 4],
        normal_mat: [[f32; 4]; 4],
        color: [f32; 4],
        params: [f32; 4],
    }

    struct GpuMesh {
        vb: ID3D11Buffer,
        ib: ID3D11Buffer,
        index_count: u32,
    }

    struct DynamicMesh {
        vb: ID3D11Buffer,
        ib: ID3D11Buffer,
        index_count: u32,
    }

    struct EarGpu {
        key: u32,
        fill: GpuMesh,
        rim: GpuMesh,
    }

    struct CupGpu {
        body: GpuMesh,
        rim: GpuMesh,
        handle: GpuMesh,
        coffee: GpuMesh,
        steam_a: DynamicMesh,
        steam_b: DynamicMesh,
    }

    struct HatGpu {
        cone: GpuMesh,
        brim: GpuMesh,
        ring: GpuMesh,
    }

    pub struct Renderer {
        device: ID3D11Device,
        ctx: ID3D11DeviceContext,
        swap_chain: IDXGISwapChain1,
        dcomp_device: IDCompositionDevice,
        _dcomp_target: IDCompositionTarget,
        dcomp_visual: IDCompositionVisual,
        dcomp_effect: Option<IDCompositionEffectGroup>,
        last_opacity: f32,
        sample_count: u32,
        width: u32,
        height: u32,
        msaa_tex: Option<ID3D11Texture2D>,
        msaa_rtv: Option<ID3D11RenderTargetView>,
        dsv: Option<ID3D11DepthStencilView>,

        // Shaders & layouts
        il_vertex: ID3D11InputLayout,
        il_instanced: ID3D11InputLayout,
        vs_basic: ID3D11VertexShader,
        ps_basic: ID3D11PixelShader,
        vs_inst: ID3D11VertexShader,
        ps_inst: ID3D11PixelShader,
        vs_fur: ID3D11VertexShader,
        ps_fur: ID3D11PixelShader,
        vs_sprite: ID3D11VertexShader,
        ps_sprite: ID3D11PixelShader,

        // States
        rs_front: ID3D11RasterizerState,
        rs_back: ID3D11RasterizerState,
        rs_none: ID3D11RasterizerState,
        rs_decal: ID3D11RasterizerState,
        ds_rw: ID3D11DepthStencilState,
        ds_ro: ID3D11DepthStencilState,
        ds_none: ID3D11DepthStencilState,
        blend_opaque: ID3D11BlendState,
        blend_premul: ID3D11BlendState,
        sampler: ID3D11SamplerState,

        // Constant buffers
        cb_frame: ID3D11Buffer,
        cb_fur: ID3D11Buffer,
        cb_obj: ID3D11Buffer,

        // Static scene meshes
        body_gpu: GpuMesh,
        head_gpu: GpuMesh,
        paw_gpu: GpuMesh,
        toe_gpu: GpuMesh,
        disc_gpu: GpuMesh,
        cap_start_gpu: GpuMesh,
        cap_end_gpu: GpuMesh,
        arc_gpu: GpuMesh,
        whiskers_gpu: Vec<GpuMesh>,
        nose_gpu: GpuMesh,
        mouth_gpu: GpuMesh,
        ear_gpus: Vec<EarGpu>,
        collar_fill_gpu: GpuMesh,
        collar_rim_gpu: GpuMesh,
        bell_ball_gpu: GpuMesh,
        bell_rim_gpu: GpuMesh,
        bell_slit_gpu: GpuMesh,
        bell_glint_gpu: GpuMesh,
        bell_loop_gpu: GpuMesh,
        kb_case_gpu: GpuMesh,
        kb_rim_gpu: GpuMesh,
        kb_plate_gpu: GpuMesh,
        kb_side_gpu: GpuMesh,
        kb_top_gpu: GpuMesh,
        kb_dot_gpu: GpuMesh,
        kb_side_inst: ID3D11Buffer,
        kb_top_inst: ID3D11Buffer,
        kb_dot_inst: ID3D11Buffer,
        mouse_dome_gpu: GpuMesh,
        mouse_wheel_gpu: GpuMesh,
        mouse_seam_gpu: GpuMesh,
        mouse_btn_gpu: GpuMesh,
        cup_gpu: Option<CupGpu>,
        hat_gpu: Option<HatGpu>,

        // Dynamic vertex buffers (updated in place)
        tail_fill_dyn: DynamicMesh,
        tail_rim_dyn: DynamicMesh,
        arm_fill_dyn: [DynamicMesh; 2],
        arm_rim_dyn: [DynamicMesh; 2],
        cable_dyn: DynamicMesh,

        atlas: Option<SpriteAtlas>,
    }

    impl Renderer {
        pub fn new(hwnd: HWND, width: u32, height: u32, scene: &Scene) -> Result<Self> {
            let w = width.max(1);
            let h = height.max(1);

            let feature_levels = [D3D_FEATURE_LEVEL_11_0];
            let mut device = None;
            let mut ctx = None;
            unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    Default::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&feature_levels),
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut ctx),
                )?;
            }
            let device = device.unwrap();
            let ctx = ctx.unwrap();
            let dxgi_device: IDXGIDevice = device.cast()?;
            let factory: IDXGIFactory2 =
                unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))? };

            let sc_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: w,
                Height: h,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                Stereo: BOOL(0),
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                Flags: 0,
            };
            let swap_chain = unsafe {
                factory.CreateSwapChainForComposition(&dxgi_device, &sc_desc, None)?
            };

            let dcomp_device: IDCompositionDevice =
                unsafe { DCompositionCreateDevice(&dxgi_device)? };
            let dcomp_target = unsafe { dcomp_device.CreateTargetForHwnd(hwnd, true)? };
            let dcomp_visual = unsafe { dcomp_device.CreateVisual()? };
            // Only attach IDCompositionEffectGroup when semi-transparent (< 1.0);
            // attaching it at 100% opacity forces DWM to allocate an extra offscreen surface.
            let dcomp_effect = if scene.opacity < 0.999 {
                let eff = unsafe { dcomp_device.CreateEffectGroup()? };
                unsafe {
                    eff.SetOpacity2(scene.opacity)?;
                    dcomp_visual.SetEffect(&eff)?;
                }
                Some(eff)
            } else {
                None
            };
            unsafe {
                dcomp_visual.SetContent(&swap_chain)?;
                dcomp_target.SetRoot(&dcomp_visual)?;
                dcomp_device.Commit()?;
            }

            let msaa4 = unsafe {
                device.CheckMultisampleQualityLevels(DXGI_FORMAT_B8G8R8A8_UNORM, 4).unwrap_or(0)
            };
            let sample_count = if msaa4 > 0 { 4 } else { 1 };

            // Compile shaders via dynamically loaded d3dcompiler_47.dll, copy bytecode out,
            // release all ID3DBlob objects, and immediately FreeLibrary so the 4.5 MB compiler DLL
            // never stays resident in memory.
            let (
                vs_basic_bc,
                ps_basic_bc,
                vs_inst_bc,
                ps_inst_bc,
                vs_fur_bc,
                ps_fur_bc,
                vs_sprite_bc,
                ps_sprite_bc,
            ) = {
                let compiler = ShaderCompiler::load()?;
                (
                    compiler.compile("vs_basic\0", "vs_5_0\0")?,
                    compiler.compile("ps_basic\0", "ps_5_0\0")?,
                    compiler.compile("vs_instanced\0", "vs_5_0\0")?,
                    compiler.compile("ps_instanced\0", "ps_5_0\0")?,
                    compiler.compile("vs_fur\0", "vs_5_0\0")?,
                    compiler.compile("ps_fur\0", "ps_5_0\0")?,
                    compiler.compile("vs_sprite\0", "vs_5_0\0")?,
                    compiler.compile("ps_sprite\0", "ps_5_0\0")?,
                )
            };

            let vs_basic = create_vs(&device, &vs_basic_bc)?;
            let ps_basic = create_ps(&device, &ps_basic_bc)?;
            let vs_inst = create_vs(&device, &vs_inst_bc)?;
            let ps_inst = create_ps(&device, &ps_inst_bc)?;
            let vs_fur = create_vs(&device, &vs_fur_bc)?;
            let ps_fur = create_ps(&device, &ps_fur_bc)?;
            let vs_sprite = create_vs(&device, &vs_sprite_bc)?;
            let ps_sprite = create_ps(&device, &ps_sprite_bc)?;

            // Input layouts
            let vert_elems = [
                elem(b"POSITION\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, false),
                elem(b"NORMAL\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 12, false),
                elem(b"TEXCOORD\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 24, false),
                elem(b"COLOR\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 36, false),
                elem(b"TEXCOORD\0", 1, DXGI_FORMAT_R32_FLOAT, 0, 48, false),
            ];
            let mut il_vertex = None;
            unsafe {
                device.CreateInputLayout(&vert_elems, &vs_basic_bc, Some(&mut il_vertex))?;
            }
            let il_vertex = il_vertex.unwrap();

            let inst_elems = [
                elem(b"POSITION\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, false),
                elem(b"NORMAL\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 12, false),
                elem(b"TEXCOORD\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 24, false),
                elem(b"COLOR\0", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 36, false),
                elem(b"TEXCOORD\0", 1, DXGI_FORMAT_R32_FLOAT, 0, 48, false),
                elem(b"TEXCOORD\0", 2, DXGI_FORMAT_R32G32B32A32_FLOAT, 1, 0, true),
                elem(b"TEXCOORD\0", 3, DXGI_FORMAT_R32G32B32A32_FLOAT, 1, 16, true),
                elem(b"TEXCOORD\0", 4, DXGI_FORMAT_R32G32B32A32_FLOAT, 1, 32, true),
                elem(b"TEXCOORD\0", 5, DXGI_FORMAT_R32G32B32A32_FLOAT, 1, 48, true),
                elem(b"COLOR\0", 1, DXGI_FORMAT_R32G32B32A32_FLOAT, 1, 64, true),
            ];
            let mut il_instanced = None;
            unsafe {
                device.CreateInputLayout(&inst_elems, &vs_inst_bc, Some(&mut il_instanced))?;
            }
            let il_instanced = il_instanced.unwrap();

            // Pipeline states
            let rs_front = make_rs(&device, D3D11_CULL_BACK, 0, 0.0)?;
            let rs_back = make_rs(&device, D3D11_CULL_FRONT, 0, 0.0)?;
            let rs_none = make_rs(&device, D3D11_CULL_NONE, 0, 0.0)?;
            let rs_decal = make_rs(&device, D3D11_CULL_BACK, -200, -2.0)?;
            let ds_rw = make_ds(&device, true, true)?;
            let ds_ro = make_ds(&device, true, false)?;
            let ds_none = make_ds(&device, false, false)?;
            let blend_opaque = make_blend(&device, false)?;
            let blend_premul = make_blend(&device, true)?;
            let sampler = make_sampler(&device)?;

            let cb_frame = make_cb(&device, mem::size_of::<FrameCB>())?;
            let cb_fur = make_cb(&device, mem::size_of::<FurCB>())?;
            let cb_obj = make_cb(&device, mem::size_of::<ObjCB>())?;

            // Upload static meshes
            let c = &scene.cat;
            let kb = &scene.keyboard;
            let mp = &scene.mouse;

            let body_gpu = upload_static(&device, &c.body_geo)?;
            let head_gpu = upload_static(&device, &c.head_geo)?;
            let paw_gpu = upload_static(&device, &c.paw_geo)?;
            let toe_gpu = upload_static(&device, &c.toe_geo)?;
            let disc_gpu = upload_static(&device, &c.disc_geo)?;
            let cap_start_gpu = upload_static(&device, &c.cap_start_geo)?;
            let cap_end_gpu = upload_static(&device, &c.cap_end_geo)?;
            let arc_gpu = upload_static(&device, &c.arc_mesh)?;
            let mut whiskers_gpu = Vec::with_capacity(c.whiskers.len());
            for w_m in &c.whiskers {
                whiskers_gpu.push(upload_static(&device, w_m)?);
            }
            let nose_gpu = upload_static(&device, &c.nose_geo)?;
            let mouth_gpu = upload_static(&device, &c.mouth_mesh)?;
            let mut ear_gpus = Vec::new();
            for eg in &c.ear_geos {
                ear_gpus.push(EarGpu {
                    key: eg.key,
                    fill: upload_static(&device, &eg.fill)?,
                    rim: upload_static(&device, &eg.rim)?,
                });
            }
            let collar_fill_gpu = upload_static(&device, &c.collar_band.body.mesh)?;
            let collar_rim_gpu = upload_static(&device, &c.collar_band.rim.mesh)?;
            let bell_ball_gpu = upload_static(&device, &c.bell_ball_geo)?;
            let bell_rim_gpu = upload_static(&device, &c.bell_rim_geo)?;
            let bell_slit_gpu = upload_static(&device, &c.bell_slit_geo)?;
            let bell_glint_gpu = upload_static(&device, &c.bell_glint_geo)?;
            let bell_loop_gpu = upload_static(&device, &c.bell_loop_geo)?;

            let kb_case_gpu = upload_static(&device, &kb.case_mesh)?;
            let kb_rim_gpu = upload_static(&device, &kb.case_rim)?;
            let kb_plate_gpu = upload_static(&device, &kb.plate_mesh)?;
            let kb_side_gpu = upload_static(&device, &kb.side_geo)?;
            let kb_top_gpu = upload_static(&device, &kb.top_geo)?;
            let kb_dot_gpu = upload_static(&device, &kb.dot_geo)?;
            let kb_side_inst = make_dynamic_buf(&device, kb.side_instances.len() * mem::size_of::<KeyInstance>(), D3D11_BIND_VERTEX_BUFFER)?;
            let kb_top_inst = make_dynamic_buf(&device, kb.top_instances.len() * mem::size_of::<KeyInstance>(), D3D11_BIND_VERTEX_BUFFER)?;
            let kb_dot_inst = make_dynamic_buf(&device, kb.dot_instances.len() * mem::size_of::<KeyInstance>(), D3D11_BIND_VERTEX_BUFFER)?;

            let mouse_dome_gpu = upload_static(&device, &mp.dome_geo)?;
            let mouse_wheel_gpu = upload_static(&device, &mp.wheel_geo)?;
            let mouse_seam_gpu = upload_static(&device, &mp.seam.mesh)?;
            let mouse_btn_gpu = upload_static(&device, &mp.button_geo)?;

            let tail_fill_dyn = upload_dynamic(&device, &c.tail.body.mesh)?;
            let tail_rim_dyn = upload_dynamic(&device, &c.tail.rim.mesh)?;
            let arm_fill_dyn = [
                upload_dynamic(&device, &c.arms[0].limb.body.mesh)?,
                upload_dynamic(&device, &c.arms[1].limb.body.mesh)?,
            ];
            let arm_rim_dyn = [
                upload_dynamic(&device, &c.arms[0].limb.rim.mesh)?,
                upload_dynamic(&device, &c.arms[1].limb.rim.mesh)?,
            ];
            let cable_dyn = upload_dynamic(&device, &mp.cable.mesh)?;

            let mut r = Self {
                device,
                ctx,
                swap_chain,
                dcomp_device,
                _dcomp_target: dcomp_target,
                dcomp_visual,
                dcomp_effect,
                last_opacity: scene.opacity,
                sample_count,
                width: w,
                height: h,
                msaa_tex: None,
                msaa_rtv: None,
                dsv: None,
                il_vertex,
                il_instanced,
                vs_basic,
                ps_basic,
                vs_inst,
                ps_inst,
                vs_fur,
                ps_fur,
                vs_sprite,
                ps_sprite,
                rs_front,
                rs_back,
                rs_none,
                rs_decal,
                ds_rw,
                ds_ro,
                ds_none,
                blend_opaque,
                blend_premul,
                sampler,
                cb_frame,
                cb_fur,
                cb_obj,
                body_gpu,
                head_gpu,
                paw_gpu,
                toe_gpu,
                disc_gpu,
                cap_start_gpu,
                cap_end_gpu,
                arc_gpu,
                whiskers_gpu,
                nose_gpu,
                mouth_gpu,
                ear_gpus,
                collar_fill_gpu,
                collar_rim_gpu,
                bell_ball_gpu,
                bell_rim_gpu,
                bell_slit_gpu,
                bell_glint_gpu,
                bell_loop_gpu,
                kb_case_gpu,
                kb_rim_gpu,
                kb_plate_gpu,
                kb_side_gpu,
                kb_top_gpu,
                kb_dot_gpu,
                kb_side_inst,
                kb_top_inst,
                kb_dot_inst,
                mouse_dome_gpu,
                mouse_wheel_gpu,
                mouse_seam_gpu,
                mouse_btn_gpu,
                cup_gpu: None,
                hat_gpu: None,
                tail_fill_dyn,
                tail_rim_dyn,
                arm_fill_dyn,
                arm_rim_dyn,
                cable_dyn,
                atlas: None,
            };
            r.create_targets()?;
            r.trim_driver();
            Ok(r)
        }

        fn trim_driver(&self) {
            unsafe {
                if let Ok(dxgi3) = self.device.cast::<IDXGIDevice3>() {
                    dxgi3.Trim();
                }
            }
        }

        pub fn release_targets_and_trim(&mut self) {
            unsafe {
                self.ctx.OMSetRenderTargets(None, None);
                self.ctx.ClearState();
                self.ctx.Flush();
            }
            self.msaa_rtv = None;
            self.msaa_tex = None;
            self.dsv = None;
            self.trim_driver();
        }

        fn create_targets(&mut self) -> Result<()> {
            let color_desc = D3D11_TEXTURE2D_DESC {
                Width: self.width,
                Height: self.height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: self.sample_count, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut msaa_tex = None;
            unsafe { self.device.CreateTexture2D(&color_desc, None, Some(&mut msaa_tex))? };
            let msaa_tex = msaa_tex.unwrap();
            let mut msaa_rtv = None;
            unsafe { self.device.CreateRenderTargetView(&msaa_tex, None, Some(&mut msaa_rtv))? };

            let depth_desc = D3D11_TEXTURE2D_DESC {
                Width: self.width,
                Height: self.height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_D24_UNORM_S8_UINT,
                SampleDesc: DXGI_SAMPLE_DESC { Count: self.sample_count, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_DEPTH_STENCIL.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut depth_tex = None;
            unsafe { self.device.CreateTexture2D(&depth_desc, None, Some(&mut depth_tex))? };
            let depth_tex = depth_tex.unwrap();
            let mut dsv = None;
            unsafe { self.device.CreateDepthStencilView(&depth_tex, None, Some(&mut dsv))? };

            self.msaa_tex = Some(msaa_tex);
            self.msaa_rtv = msaa_rtv;
            self.dsv = dsv;
            Ok(())
        }

        pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
            let w = width.max(1);
            let h = height.max(1);
            if w == self.width && h == self.height {
                return Ok(());
            }
            self.width = w;
            self.height = h;
            unsafe {
                self.ctx.OMSetRenderTargets(None, None);
            }
            self.msaa_rtv = None;
            self.msaa_tex = None;
            self.dsv = None;
            unsafe {
                self.swap_chain.ResizeBuffers(2, w, h, DXGI_FORMAT_B8G8R8A8_UNORM, Default::default())?;
            }
            self.create_targets()
        }

        fn ensure_ear_gpu(&mut self, scene: &Scene) -> usize {
            let eg = scene.cat.active_ear();
            if let Some(pos) = self.ear_gpus.iter().position(|e| e.key == eg.key) {
                return pos;
            }
            if let (Ok(fill), Ok(rim)) = (
                upload_static(&self.device, &eg.fill),
                upload_static(&self.device, &eg.rim),
            ) {
                self.ear_gpus.push(EarGpu { key: eg.key, fill, rim });
                return self.ear_gpus.len() - 1;
            }
            0
        }

        fn ensure_cup_gpu(&mut self, scene: &Scene) -> bool {
            if self.cup_gpu.is_some() {
                return true;
            }
            let cup = &scene.cup;
            if let (Ok(body), Ok(rim), Ok(handle), Ok(coffee), Ok(steam_a), Ok(steam_b)) = (
                upload_static(&self.device, &cup.body_geo),
                upload_static(&self.device, &cup.rim_geo),
                upload_static(&self.device, &cup.handle_geo),
                upload_static(&self.device, &cup.coffee_geo),
                upload_dynamic(&self.device, &cup.steam_a.mesh),
                upload_dynamic(&self.device, &cup.steam_b.mesh),
            ) {
                write_buf(&self.ctx, &steam_a.vb, &cup.steam_a.mesh.vertices);
                write_buf(&self.ctx, &steam_b.vb, &cup.steam_b.mesh.vertices);
                self.cup_gpu = Some(CupGpu { body, rim, handle, coffee, steam_a, steam_b });
                return true;
            }
            false
        }

        fn ensure_hat_gpu(&mut self, scene: &Scene) -> bool {
            if self.hat_gpu.is_some() {
                return true;
            }
            let c = &scene.cat;
            if let (Ok(cone), Ok(brim), Ok(ring)) = (
                upload_static(&self.device, &c.hat_cone_geo),
                upload_static(&self.device, &c.hat_brim_geo),
                upload_static(&self.device, &c.hat_ring_geo),
            ) {
                self.hat_gpu = Some(HatGpu { cone, brim, ring });
                return true;
            }
            false
        }

        pub fn render(&mut self, scene: &mut Scene) {
            if self.msaa_rtv.is_none() && self.create_targets().is_err() {
                return;
            }
            let ear_idx = self.ensure_ear_gpu(scene);

            // Upload dirty dynamic meshes & keyboard instances (per-part to minimize WRITE_DISCARD)
            if scene.keyboard.instances_dirty {
                write_buf(&self.ctx, &self.kb_side_inst, &scene.keyboard.side_instances);
                write_buf(&self.ctx, &self.kb_top_inst, &scene.keyboard.top_instances);
                write_buf(&self.ctx, &self.kb_dot_inst, &scene.keyboard.dot_instances);
                scene.keyboard.instances_dirty = false;
            }
            if scene.mouse.cable_dirty {
                write_buf(&self.ctx, &self.cable_dyn.vb, &scene.mouse.cable.mesh.vertices);
                scene.mouse.cable_dirty = false;
            }
            if scene.cat.tail_dirty {
                write_buf(&self.ctx, &self.tail_fill_dyn.vb, &scene.cat.tail.body.mesh.vertices);
                write_buf(&self.ctx, &self.tail_rim_dyn.vb, &scene.cat.tail.rim.mesh.vertices);
                scene.cat.tail_dirty = false;
            }
            for i in 0..2 {
                if scene.cat.arms_dirty[i] {
                    write_buf(&self.ctx, &self.arm_fill_dyn[i].vb, &scene.cat.arms[i].limb.body.mesh.vertices);
                    write_buf(&self.ctx, &self.arm_rim_dyn[i].vb, &scene.cat.arms[i].limb.rim.mesh.vertices);
                    scene.cat.arms_dirty[i] = false;
                }
            }
            scene.cat.dynamic_dirty = false;

            if scene.cup.visible() && self.ensure_cup_gpu(scene) && scene.cup.steam_dirty {
                if let Some(ref cg) = self.cup_gpu {
                    write_buf(&self.ctx, &cg.steam_a.vb, &scene.cup.steam_a.mesh.vertices);
                    write_buf(&self.ctx, &cg.steam_b.vb, &scene.cup.steam_b.mesh.vertices);
                }
                scene.cup.steam_dirty = false;
            } else if !scene.cup.enabled && self.cup_gpu.is_some() {
                self.cup_gpu = None;
            }

            if (scene.opacity - self.last_opacity).abs() > 1e-3 {
                self.last_opacity = scene.opacity;
                unsafe {
                    if scene.opacity >= 0.999 {
                        if self.dcomp_effect.take().is_some() {
                            let _ = self.dcomp_visual.SetEffect(None);
                            let _ = self.dcomp_device.Commit();
                        }
                    } else {
                        if self.dcomp_effect.is_none() {
                            if let Ok(eff) = self.dcomp_device.CreateEffectGroup() {
                                let _ = self.dcomp_visual.SetEffect(&eff);
                                self.dcomp_effect = Some(eff);
                            }
                        }
                        if let Some(ref eff) = self.dcomp_effect {
                            let _ = eff.SetOpacity2(scene.opacity);
                            let _ = self.dcomp_device.Commit();
                        }
                    }
                }
            }

            let Some(rtv) = self.msaa_rtv.clone() else { return };
            let Some(dsv) = self.dsv.clone() else { return };

            // Upload Frame & Fur CBs
            let frame_cb = FrameCB {
                view: scene.view.to_cols_array_2d(),
                proj: scene.proj.to_cols_array_2d(),
                bend: [scene.cat.bend.x, scene.cat.bend.squash, scene.cat.bend.z, 0.0],
                cam_right: [scene.cam_right.x, scene.cam_right.y, scene.cam_right.z, 0.0],
                cam_up: [scene.cam_up.x, scene.cam_up.y, scene.cam_up.z, 0.0],
            };
            write_struct(&self.ctx, &self.cb_frame, &frame_cb);

            let rl = &scene.cat.resolved;
            let fur_cb = FurCB {
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
            write_struct(&self.ctx, &self.cb_fur, &fur_cb);

            unsafe {
                let vp = D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: self.width as f32,
                    Height: self.height as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                };
                self.ctx.RSSetViewports(Some(&[vp]));
                self.ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), Some(&dsv));
                self.ctx.ClearRenderTargetView(&rtv, &[0.0, 0.0, 0.0, 0.0]);
                self.ctx.ClearDepthStencilView(&dsv, D3D11_CLEAR_DEPTH.0 as u32, 1.0, 0);

                let cbs = [Some(self.cb_frame.clone()), Some(self.cb_fur.clone()), Some(self.cb_obj.clone())];
                self.ctx.VSSetConstantBuffers(0, Some(&cbs));
                self.ctx.PSSetConstantBuffers(0, Some(&cbs));
                self.ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            }

            let view = scene.view;
            let body_w = scene.cat.body_world();
            let head_w = scene.cat.head_world();
            let board_w = scene.keyboard.world_matrix();
            let paw_rim_col = hex_color("#8f8f8a");

            // ===== Order -2: desk cup, mouse outer shell + hind paw rims =====
            if scene.cup.visible() {
                if let Some(ref cg) = self.cup_gpu {
                    let cw = scene.cup.world_matrix();
                    let hw = scene.cup.handle_world();
                    let cfw = scene.cup.coffee_world();
                    let alpha = scene.cup.opacity;
                    let blend = if alpha < 0.99 { &self.blend_premul } else { &self.blend_opaque };
                    self.set_basic_pipe(&self.rs_back, &self.ds_rw, blend);
                    self.draw_static(&cg.rim, cw, view, hex_color("#2a241f"), alpha, [0.0; 4]);
                    self.set_basic_pipe(&self.rs_front, &self.ds_rw, blend);
                    self.draw_static(&cg.body, cw, view, hex_color("#f7f2e7"), alpha, [0.0; 4]);
                    self.draw_static(&cg.handle, hw, view, hex_color("#efe7d7"), alpha, [0.0; 4]);
                    self.draw_static(&cg.coffee, cfw, view, hex_color("#5b3a24"), alpha, [0.0; 4]);
                    if scene.cup.steam_opacity_a > 0.01 || scene.cup.steam_opacity_b > 0.01 {
                        self.set_basic_pipe(&self.rs_none, &self.ds_ro, &self.blend_premul);
                        if scene.cup.steam_opacity_a > 0.01 {
                            self.draw_dynamic(&cg.steam_a, Mat4::IDENTITY, view, hex_color("#fbfaf3"), scene.cup.steam_opacity_a, [0.0; 4]);
                        }
                        if scene.cup.steam_opacity_b > 0.01 {
                            self.draw_dynamic(&cg.steam_b, Mat4::IDENTITY, view, hex_color("#fbfaf3"), scene.cup.steam_opacity_b, [0.0; 4]);
                        }
                    }
                }
            }

            self.set_basic_pipe(&self.rs_back, &self.ds_rw, &self.blend_opaque);
            self.draw_static(&self.mouse_dome_gpu, scene.mouse.shell_world(0.03), view, hex_color("#f4f1e6"), 1.0, [0.0; 4]);

            self.set_basic_pipe(&self.rs_back, &self.ds_ro, &self.blend_opaque);
            for &s in &[-1.0f32, 1.0] {
                self.draw_static(&self.paw_gpu, scene.cat.hind_paw_world(s, true), view, paw_rim_col, 1.0, [0.0; 4]);
            }

            // ===== Order -1: mouse inner shell + body/head/ear/tail rims =====
            self.set_basic_pipe(&self.rs_back, &self.ds_rw, &self.blend_opaque);
            self.draw_static(&self.mouse_dome_gpu, scene.mouse.shell_world(0.013), view, hex_color("#1d1d1b"), 1.0, [0.0; 4]);

            self.set_basic_pipe(&self.rs_back, &self.ds_ro, &self.blend_opaque);
            let body_rim_m = body_w.mul(Mat4::from_scale(Vec3::new(1.045, 1.018, 0.86)));
            self.draw_static(&self.body_gpu, body_rim_m, view, rl.rim_color, 1.0, [2.0, 1.0 / 1.045, 1.0 / 0.86, 0.0]);
            let head_rim_m = head_w.mul(Mat4::from_scale(Vec3::new(HEAD_R.x + 0.024, HEAD_R.y + 0.024, HEAD_R.z + 0.02)));
            self.draw_static(&self.head_gpu, head_rim_m, view, rl.rim_color, 1.0, [0.0; 4]);
            for i in 0..2 {
                let ear_rim_m = scene.cat.ear_world(i).mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.56)));
                self.draw_static(&self.ear_gpus[ear_idx].rim, ear_rim_m, view, rl.rim_color, 1.0, [0.0; 4]);
            }
            self.draw_dynamic(&self.tail_rim_dyn, Mat4::IDENTITY, view, rl.rim_color, 1.0, [0.0; 4]);
            let tail_cap_rim_m = Mat4::from_trs_euler(scene.cat.tail.end_pos, Vec3::ZERO, Vec3::splat(scene.cat.tail.end_scale_rim));
            self.draw_static(&self.cap_end_gpu, tail_cap_rim_m, view, rl.rim_color, 1.0, [0.0; 4]);

            // ===== Order 0: keyboard case/plate/keys, mouse dome, cat fur, chinLine =====
            self.set_basic_pipe(&self.rs_back, &self.ds_rw, &self.blend_opaque);
            self.draw_static(&self.kb_rim_gpu, board_w, view, hex_color("#f4f1e6"), 1.0, [0.0; 4]);
            if rl.chin_visible {
                let chin_m = head_w.mul(Mat4::from_scale(HEAD_R));
                self.draw_static(&self.head_gpu, chin_m, view, rl.chin_color, 1.0, [3.0, 0.05, 0.0, 0.0]);
            }

            self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
            self.draw_static(&self.kb_case_gpu, board_w, view, scene.keyboard.case_color, 1.0, [0.0; 4]);
            self.draw_static(&self.kb_plate_gpu, board_w, view, scene.keyboard.plate_color, 1.0, [0.0; 4]);
            if rl.style_3d {
                self.draw_static(&self.mouse_dome_gpu, scene.mouse.dome_world(), view, [1.0, 1.0, 1.0], 1.0, [1.0, 0.0, 0.0, 0.0]);
            } else {
                self.draw_static(&self.mouse_dome_gpu, scene.mouse.dome_world(), view, hex_color("#f8f7f0"), 1.0, [0.0; 4]);
            }

            // Instanced keyboard (3 draw calls for all 61 keys!)
            self.draw_keyboard_instanced(board_w, scene.keyboard.keys.len() as u32);

            // Fur meshes (body, head, ears, tail)
            self.set_fur_pipe(&self.rs_front, &self.ds_rw);
            let body_m = body_w.mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.82)));
            self.draw_static(&self.body_gpu, body_m, view, [1.0; 3], 1.0, [0.0, 1.0, 1.0 / 0.82, 1.0]);
            let head_m = head_w.mul(Mat4::from_scale(HEAD_R));
            self.draw_static(&self.head_gpu, head_m, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 0.0]);
            for i in 0..2 {
                let ear_m = scene.cat.ear_world(i).mul(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.5)));
                let part = if i == 0 { 2.0 } else { 3.0 };
                self.draw_static(&self.ear_gpus[ear_idx].fill, ear_m, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, part]);
            }
            self.draw_dynamic(&self.tail_fill_dyn, Mat4::IDENTITY, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 4.0]);
            let tail_cap_m = Mat4::from_trs_euler(scene.cat.tail.end_pos, Vec3::ZERO, Vec3::splat(scene.cat.tail.end_scale_fill));
            self.draw_static(&self.cap_end_gpu, tail_cap_m, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 4.0]);

            // ===== Order 0~1: collar / CNY scarf (with towardEye = 0.07) & mouse cable =====
            let outfit = scene.cat.outfit;
            if outfit == Festival::Cny {
                let cw = scene.cat.collar_world().mul(Mat4::from_scale(Vec3::new(1.05, 1.45, 1.05)));
                self.set_basic_pipe(&self.rs_back, &self.ds_rw, &self.blend_opaque);
                self.draw_static(&self.collar_rim_gpu, cw, view, hex_color("#7a1d17"), 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
                self.draw_static(&self.collar_fill_gpu, cw, view, hex_color("#d93829"), 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
            } else if rl.collar_visible {
                let cw = scene.cat.collar_world();
                let bh = scene.cat.bell_hang_world();
                let r_bell: f32 = 0.064;
                let ink = hex_color("#1d1d1b");
                let gold = hex_color("#e8b93a");

                self.set_basic_pipe(&self.rs_back, &self.ds_rw, &self.blend_opaque);
                self.draw_static(&self.collar_rim_gpu, cw, view, ink, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                let cs_r = cw.mul(Mat4::from_trs_euler(scene.cat.collar_band.start_pos, Vec3::ZERO, Vec3::splat(scene.cat.collar_band.start_scale_rim)));
                let ce_r = cw.mul(Mat4::from_trs_euler(scene.cat.collar_band.end_pos, Vec3::ZERO, Vec3::splat(scene.cat.collar_band.end_scale_rim)));
                self.draw_static(&self.cap_start_gpu, cs_r, view, ink, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                self.draw_static(&self.cap_end_gpu, ce_r, view, ink, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                self.draw_static(&self.bell_rim_gpu, bh, view, ink, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);

                self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
                self.draw_static(&self.collar_fill_gpu, cw, view, rl.collar_color, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                let cs_f = cw.mul(Mat4::from_trs_euler(scene.cat.collar_band.start_pos, Vec3::ZERO, Vec3::splat(scene.cat.collar_band.start_scale_fill)));
                let ce_f = cw.mul(Mat4::from_trs_euler(scene.cat.collar_band.end_pos, Vec3::ZERO, Vec3::splat(scene.cat.collar_band.end_scale_fill)));
                self.draw_static(&self.cap_start_gpu, cs_f, view, rl.collar_color, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                self.draw_static(&self.cap_end_gpu, ce_f, view, rl.collar_color, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                self.draw_static(&self.bell_ball_gpu, bh, view, gold, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                let loop_m = bh.mul(Mat4::from_translation(Vec3::new(0.0, r_bell + 0.01, 0.0)));
                self.draw_static(&self.bell_loop_gpu, loop_m, view, ink, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);

                self.set_basic_pipe(&self.rs_decal, &self.ds_rw, &self.blend_opaque);
                let slit_z = (r_bell * r_bell - 0.022 * 0.022).sqrt() + 0.002;
                let slit_m = bh.mul(Mat4::from_trs_euler(Vec3::new(0.0, -0.022, slit_z), Vec3::new(-0.33, 0.0, 0.0), Vec3::ONE));
                self.draw_static(&self.bell_slit_gpu, slit_m, view, ink, 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
                let glint_z = (r_bell * r_bell - 0.022 * 0.022 - 0.024 * 0.024).sqrt() + 0.002;
                let glint_m = bh.mul(Mat4::from_trs_euler(Vec3::new(-0.022, 0.024, glint_z), Vec3::new(-0.35, -0.35, 0.0), Vec3::ONE));
                self.draw_static(&self.bell_glint_gpu, glint_m, view, hex_color("#fff6d8"), 1.0, [0.0, 0.0, COLLAR_LIFT, 0.0]);
            }

            if scene.mouse.cable_visible {
                self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
                self.draw_dynamic(&self.cable_dyn, Mat4::IDENTITY, view, hex_color("#2a2a27"), 1.0, [0.0; 4]);
            }

            // ===== Order 2~3: hind paws + toes =====
            for &s in &[-1.0f32, 1.0] {
                let pm = scene.cat.hind_paw_world(s, false);
                self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
                self.draw_static(&self.paw_gpu, pm, view, rl.paw_color, 1.0, [0.0; 4]);
                self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                for &tx in &[-0.3f32, 0.3] {
                    let tm = pm.mul(Mat4::from_translation(Vec3::new(tx, 0.66, 0.72)));
                    self.draw_static(&self.toe_gpu, tm, view, rl.toe_color, 1.0, [0.0; 4]);
                }
            }

            // ===== Order 5~7: mouse overlays (depthTest=false) =====
            if scene.mouse.pressed > 0.005 {
                self.set_basic_pipe(&self.rs_none, &self.ds_none, &self.blend_premul);
                self.draw_static(&self.mouse_btn_gpu, scene.mouse.button_world(), view, hex_color("#b9d84f"), 0.9 * scene.mouse.pressed, [0.0; 4]);
            }
            self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
            self.draw_static(&self.mouse_seam_gpu, scene.mouse.shape_world(), view, hex_color("#2a2a27"), 1.0, [0.0; 4]);
            self.draw_static(&self.mouse_wheel_gpu, scene.mouse.wheel_world(), view, hex_color("#e9a94f"), 1.0, [0.0; 4]);

            // ===== Order 10~12: face overlays (eyes, blush, whiskers, nose, mouth; depthTest=false) =====
            let pupil_col = hex_color("#141414");
            let shine_col = [1.0, 1.0, 1.0];
            let blush_col = hex_color("#f08a8a");
            let dots = scene.cat.shine_dots();
            for i in 0..2 {
                let ew = scene.cat.eye_world(i);
                let eye = scene.cat.eyes[i];
                if !eye.closed {
                    self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                    let iris_m = ew.mul(Mat4::from_scale(Vec3::new(0.155, 0.14, 1.0)));
                    self.draw_static(&self.disc_gpu, iris_m, view, rl.eye_colors[i], 1.0, [0.0; 4]);

                    let pupil_m = ew.mul(Mat4::from_trs_euler(
                        Vec3::new(eye.pupil_pos.x, eye.pupil_pos.y, 0.002),
                        Vec3::ZERO,
                        Vec3::new(eye.pupil_scale.x, eye.pupil_scale.y, 1.0),
                    ));
                    self.draw_static(&self.disc_gpu, pupil_m, view, pupil_col, 1.0, [0.0; 4]);

                    for d in dots {
                        let sm = ew.mul(Mat4::from_trs_euler(
                            Vec3::new(eye.side * d[0], d[1], 0.004),
                            Vec3::ZERO,
                            Vec3::new(d[2], d[2], 1.0),
                        ));
                        self.draw_static(&self.disc_gpu, sm, view, shine_col, 1.0, [0.0; 4]);
                    }
                } else {
                    self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                    self.draw_static(&self.arc_gpu, ew, view, rl.lid_color, 1.0, [0.0; 4]);
                }

                if scene.cat.blush_opacity > 0.005 {
                    self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_premul);
                    let bm = ew.mul(Mat4::from_trs_euler(
                        Vec3::new(eye.side * 0.06, -0.15, 0.001),
                        Vec3::ZERO,
                        Vec3::new(0.085, 0.048, 1.0),
                    ));
                    self.draw_static(&self.disc_gpu, bm, view, blush_col, scene.cat.blush_opacity, [0.0; 4]);
                }
            }
            if rl.whiskers_visible {
                self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                for wg in &self.whiskers_gpu {
                    self.draw_static(wg, head_w, view, rl.whisker_color, 1.0, [0.0; 4]);
                }
            }
            if rl.face_visible {
                self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                self.draw_static(&self.nose_gpu, scene.cat.nose_world(), view, rl.nose_color, 1.0, [0.0; 4]);
                self.draw_static(&self.mouth_gpu, head_w, view, rl.mouth_color, 1.0, [0.0; 4]);
            }
            if scene.cat.yawn_open > 0.03 {
                self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                let yw = scene.cat.yawn_world();
                self.draw_static(&self.disc_gpu, yw, view, hex_color("#4a1b24"), 1.0, [0.0; 4]);
                let tongue = yw.mul(Mat4::from_trs_euler(Vec3::new(0.0, -0.32, 0.002), Vec3::ZERO, Vec3::new(0.65, 0.48, 1.0)));
                self.draw_static(&self.disc_gpu, tongue, view, hex_color("#f08a9a"), 1.0, [0.0; 4]);
            }
            if scene.cat.lick_out > 0.03 {
                self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                self.draw_static(&self.disc_gpu, scene.cat.lick_world(), view, hex_color("#f393a3"), 1.0, [0.0; 4]);
            }
            if matches!(outfit, Festival::Xmas | Festival::Halloween | Festival::Birthday)
                && self.ensure_hat_gpu(scene)
            {
                if let Some(ref hg) = self.hat_gpu {
                    self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
                    let hr = scene.cat.hat_root_world();
                    match outfit {
                        Festival::Xmas => {
                            self.draw_static(&hg.cone, hr.mul(Mat4::from_scale(Vec3::new(0.28, 0.56, 0.28))), view, hex_color("#d9483b"), 1.0, [0.0; 4]);
                            self.draw_static(&hg.ring, hr.mul(Mat4::from_scale(Vec3::new(0.27, 0.27, 0.27))), view, hex_color("#fbf9f3"), 1.0, [0.0; 4]);
                            self.draw_static(&self.bell_ball_gpu, hr.mul(Mat4::from_trs_euler(Vec3::new(0.0, 0.56, 0.0), Vec3::ZERO, Vec3::splat(1.3))), view, hex_color("#fbf9f3"), 1.0, [0.0; 4]);
                        }
                        Festival::Halloween => {
                            self.draw_static(&hg.brim, hr.mul(Mat4::from_scale(Vec3::new(0.45, 0.028, 0.45))), view, hex_color("#2b2440"), 1.0, [0.0; 4]);
                            self.draw_static(&hg.cone, hr.mul(Mat4::from_scale(Vec3::new(0.22, 0.60, 0.22))), view, hex_color("#2b2440"), 1.0, [0.0; 4]);
                            self.draw_static(&hg.ring, hr.mul(Mat4::from_trs_euler(Vec3::new(0.0, 0.05, 0.0), Vec3::ZERO, Vec3::new(0.21, 0.18, 0.21))), view, hex_color("#9a6bd1"), 1.0, [0.0; 4]);
                        }
                        Festival::Birthday => {
                            self.draw_static(&hg.cone, hr.mul(Mat4::from_scale(Vec3::new(0.21, 0.54, 0.21))), view, hex_color("#ee8fb4"), 1.0, [0.0; 4]);
                            self.draw_static(&hg.ring, hr.mul(Mat4::from_scale(Vec3::new(0.21, 0.16, 0.21))), view, hex_color("#70c4b8"), 1.0, [0.0; 4]);
                            self.draw_static(&self.bell_ball_gpu, hr.mul(Mat4::from_trs_euler(Vec3::new(0.0, 0.54, 0.0), Vec3::ZERO, Vec3::splat(1.15))), view, hex_color("#f2c94c"), 1.0, [0.0; 4]);
                        }
                        _ => {}
                    }
                }
            }

            // ===== Order 18~23: front paw rims, arm rims/bodies (with ARM_ROOT_PUSH), front paws/toes =====
            self.set_basic_pipe(&self.rs_back, &self.ds_ro, &self.blend_opaque);
            for i in 0..2 {
                self.draw_static(&self.paw_gpu, scene.cat.arm_paw_world(i, true), view, paw_rim_col, 1.0, [0.0; 4]);
            }

            self.set_basic_pipe(&self.rs_back, &self.ds_rw, &self.blend_opaque);
            for i in 0..2 {
                self.draw_dynamic(&self.arm_rim_dyn[i], Mat4::IDENTITY, view, rl.arm_rim_color, 1.0, [4.0, 0.0, 0.0, 0.0]);
                let limb = &scene.cat.arms[i].limb;
                let ms = Mat4::from_trs_euler(limb.start_pos, Vec3::ZERO, Vec3::splat(limb.start_scale_rim));
                let me = Mat4::from_trs_euler(limb.end_pos, Vec3::ZERO, Vec3::splat(limb.end_scale_rim));
                self.draw_static(&self.cap_start_gpu, ms, view, rl.arm_rim_color, 1.0, [4.0, 0.0, 0.0, 0.0]);
                self.draw_static(&self.cap_end_gpu, me, view, rl.arm_rim_color, 1.0, [4.0, 0.0, 0.0, 0.0]);
            }

            self.set_fur_pipe(&self.rs_front, &self.ds_rw);
            for i in 0..2 {
                self.draw_dynamic(&self.arm_fill_dyn[i], Mat4::IDENTITY, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 5.0]);
                let limb = &scene.cat.arms[i].limb;
                let ms = Mat4::from_trs_euler(limb.start_pos, Vec3::ZERO, Vec3::splat(limb.start_scale_fill));
                let me = Mat4::from_trs_euler(limb.end_pos, Vec3::ZERO, Vec3::splat(limb.end_scale_fill));
                self.draw_static(&self.cap_start_gpu, ms, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 5.0]);
                self.draw_static(&self.cap_end_gpu, me, view, [1.0; 3], 1.0, [0.0, 0.0, 0.0, 5.0]);
            }

            for i in 0..2 {
                let pm = scene.cat.arm_paw_world(i, false);
                self.set_basic_pipe(&self.rs_front, &self.ds_rw, &self.blend_opaque);
                self.draw_static(&self.paw_gpu, pm, view, rl.paw_color, 1.0, [0.0; 4]);
                self.set_basic_pipe(&self.rs_front, &self.ds_none, &self.blend_opaque);
                for &tx in &[-0.3f32, 0.3] {
                    let tm = pm.mul(Mat4::from_translation(Vec3::new(tx, 0.66, 0.72)));
                    self.draw_static(&self.toe_gpu, tm, view, rl.toe_color, 1.0, [0.0; 4]);
                }
            }

            // ===== Order 40~45: 2D Billboard Sprites =====
            if !scene.sprites.is_empty() {
                if self.atlas.is_none() {
                    self.atlas = SpriteAtlas::new().ok();
                }
                if let Some(ref mut atlas) = self.atlas {
                    unsafe {
                        self.ctx.IASetInputLayout(None);
                        self.ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                        self.ctx.VSSetShader(&self.vs_sprite, None);
                        self.ctx.PSSetShader(&self.ps_sprite, None);
                        self.ctx.RSSetState(&self.rs_none);
                        self.ctx.OMSetDepthStencilState(&self.ds_none, 0);
                        self.ctx.OMSetBlendState(&self.blend_premul, None, u32::MAX);
                        self.ctx.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
                    }
                    for sp in &scene.sprites {
                        let Some(srv) = atlas.get_or_bake(&self.device, &sp.key) else { continue };
                        let mut m = Mat4::IDENTITY;
                        m.cols[3] = [sp.pos.x, sp.pos.y, sp.pos.z, 1.0];
                        let obj = ObjCB {
                            model: m.to_cols_array_2d(),
                            normal_mat: Mat4::IDENTITY.to_cols_array_2d(),
                            color: [1.0, 1.0, 1.0, sp.opacity],
                            params: [sp.scale_x, sp.scale_y, 0.0, 0.0],
                        };
                        write_struct(&self.ctx, &self.cb_obj, &obj);
                        unsafe {
                            self.ctx.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
                            self.ctx.Draw(4, 0);
                        }
                    }
                    unsafe {
                        self.ctx.PSSetShaderResources(0, Some(&[None]));
                        self.ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
                    }
                }
            }

            // Resolve MSAA -> SwapChain back buffer & Present
            unsafe {
                if let Ok(back) = self.swap_chain.GetBuffer::<ID3D11Texture2D>(0) {
                    if let Some(ref msaa) = self.msaa_tex {
                        if self.sample_count > 1 {
                            self.ctx.ResolveSubresource(&back, 0, msaa, 0, DXGI_FORMAT_B8G8R8A8_UNORM);
                        } else {
                            self.ctx.CopyResource(&back, msaa);
                        }
                    }
                }
                let _ = self.swap_chain.Present(0, DXGI_PRESENT(0));
            }
        }

        fn set_basic_pipe(
            &self,
            rs: &ID3D11RasterizerState,
            ds: &ID3D11DepthStencilState,
            blend: &ID3D11BlendState,
        ) {
            unsafe {
                self.ctx.IASetInputLayout(&self.il_vertex);
                self.ctx.VSSetShader(&self.vs_basic, None);
                self.ctx.PSSetShader(&self.ps_basic, None);
                self.ctx.RSSetState(rs);
                self.ctx.OMSetDepthStencilState(ds, 0);
                self.ctx.OMSetBlendState(blend, None, u32::MAX);
            }
        }

        fn set_fur_pipe(&self, rs: &ID3D11RasterizerState, ds: &ID3D11DepthStencilState) {
            unsafe {
                self.ctx.IASetInputLayout(&self.il_vertex);
                self.ctx.VSSetShader(&self.vs_fur, None);
                self.ctx.PSSetShader(&self.ps_fur, None);
                self.ctx.RSSetState(rs);
                self.ctx.OMSetDepthStencilState(ds, 0);
                self.ctx.OMSetBlendState(&self.blend_opaque, None, u32::MAX);
            }
        }

        fn draw_static(
            &self,
            mesh: &GpuMesh,
            model: Mat4,
            view: Mat4,
            color: [f32; 3],
            alpha: f32,
            params: [f32; 4],
        ) {
            let nrm = view.mul(model).normal_matrix();
            let obj = ObjCB {
                model: model.to_cols_array_2d(),
                normal_mat: nrm.to_cols_array_2d(),
                color: [color[0], color[1], color[2], alpha],
                params,
            };
            write_struct(&self.ctx, &self.cb_obj, &obj);
            let stride = mem::size_of::<Vertex>() as u32;
            let offset = 0u32;
            unsafe {
                self.ctx.IASetVertexBuffers(0, 1, Some(&Some(mesh.vb.clone())), Some(&stride), Some(&offset));
                self.ctx.IASetIndexBuffer(&mesh.ib, DXGI_FORMAT_R32_UINT, 0);
                self.ctx.DrawIndexed(mesh.index_count, 0, 0);
            }
        }

        fn draw_dynamic(
            &self,
            mesh: &DynamicMesh,
            model: Mat4,
            view: Mat4,
            color: [f32; 3],
            alpha: f32,
            params: [f32; 4],
        ) {
            let nrm = view.mul(model).normal_matrix();
            let obj = ObjCB {
                model: model.to_cols_array_2d(),
                normal_mat: nrm.to_cols_array_2d(),
                color: [color[0], color[1], color[2], alpha],
                params,
            };
            write_struct(&self.ctx, &self.cb_obj, &obj);
            let stride = mem::size_of::<Vertex>() as u32;
            let offset = 0u32;
            unsafe {
                self.ctx.IASetVertexBuffers(0, 1, Some(&Some(mesh.vb.clone())), Some(&stride), Some(&offset));
                self.ctx.IASetIndexBuffer(&mesh.ib, DXGI_FORMAT_R32_UINT, 0);
                self.ctx.DrawIndexed(mesh.index_count, 0, 0);
            }
        }

        fn draw_keyboard_instanced(&self, board_w: Mat4, count: u32) {
            let obj = ObjCB {
                model: board_w.to_cols_array_2d(),
                normal_mat: Mat4::IDENTITY.to_cols_array_2d(),
                color: [1.0; 4],
                params: [0.0; 4],
            };
            write_struct(&self.ctx, &self.cb_obj, &obj);
            let strides = [mem::size_of::<Vertex>() as u32, mem::size_of::<KeyInstance>() as u32];
            let offsets = [0u32, 0u32];
            unsafe {
                self.ctx.IASetInputLayout(&self.il_instanced);
                self.ctx.VSSetShader(&self.vs_inst, None);
                self.ctx.PSSetShader(&self.ps_inst, None);
                self.ctx.RSSetState(&self.rs_front);
                self.ctx.OMSetDepthStencilState(&self.ds_rw, 0);
                self.ctx.OMSetBlendState(&self.blend_opaque, None, u32::MAX);

                for (mesh, inst) in [
                    (&self.kb_side_gpu, &self.kb_side_inst),
                    (&self.kb_top_gpu, &self.kb_top_inst),
                    (&self.kb_dot_gpu, &self.kb_dot_inst),
                ] {
                    let bufs = [Some(mesh.vb.clone()), Some(inst.clone())];
                    self.ctx.IASetVertexBuffers(0, 2, Some(bufs.as_ptr()), Some(strides.as_ptr()), Some(offsets.as_ptr()));
                    self.ctx.IASetIndexBuffer(&mesh.ib, DXGI_FORMAT_R32_UINT, 0);
                    self.ctx.DrawIndexedInstanced(mesh.index_count, count, 0, 0, 0);
                }
            }
        }
    }

    type FnD3DCompile = unsafe extern "system" fn(
        *const c_void,
        usize,
        PCSTR,
        *const c_void,
        *const c_void,
        PCSTR,
        PCSTR,
        u32,
        u32,
        *mut Option<ID3DBlob>,
        *mut Option<ID3DBlob>,
    ) -> HRESULT;

    struct ShaderCompiler {
        module: HMODULE,
        compile_fn: FnD3DCompile,
    }

    impl ShaderCompiler {
        fn load() -> Result<Self> {
            unsafe {
                let module = LoadLibraryW(w!("d3dcompiler_47.dll"))
                    .or_else(|_| LoadLibraryW(w!("d3dcompiler_43.dll")))?;
                let Some(proc) = GetProcAddress(module, s!("D3DCompile")) else {
                    let _ = FreeLibrary(module);
                    return Err(Error::from_thread());
                };
                Ok(Self {
                    module,
                    compile_fn: mem::transmute::<*const c_void, FnD3DCompile>(proc as *const c_void),
                })
            }
        }

        fn compile(&self, entry: &str, target: &str) -> Result<Vec<u8>> {
            let mut blob: Option<ID3DBlob> = None;
            let mut err: Option<ID3DBlob> = None;
            let hr = unsafe {
                (self.compile_fn)(
                    SHADERS_SRC.as_ptr() as *const c_void,
                    SHADERS_SRC.len(),
                    PCSTR::null(),
                    ptr::null(),
                    ptr::null(),
                    PCSTR(entry.as_ptr()),
                    PCSTR(target.as_ptr()),
                    0,
                    0,
                    &mut blob,
                    &mut err,
                )
            };
            if hr.is_err() {
                if let Some(err_blob) = err.take() {
                    let msg = unsafe {
                        let bytes = std::slice::from_raw_parts(
                            err_blob.GetBufferPointer() as *const u8,
                            err_blob.GetBufferSize(),
                        );
                        String::from_utf8_lossy(bytes).into_owned()
                    };
                    drop(err_blob);
                    if let Some(dir) = std::env::var_os("APPDATA") {
                        let p = std::path::PathBuf::from(dir).join("com.ellobok.keycat");
                        let _ = std::fs::create_dir_all(&p);
                        let _ = std::fs::write(p.join("native-error.txt"), format!("HLSL ({entry}): {msg}\n"));
                    }
                }
                drop(blob);
                return Err(Error::from_hresult(hr));
            }
            drop(err);
            let b = blob.expect("D3DCompile succeeded without blob");
            let out = unsafe {
                std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()).to_vec()
            };
            drop(b);
            Ok(out)
        }
    }

    impl Drop for ShaderCompiler {
        fn drop(&mut self) {
            unsafe {
                let _ = FreeLibrary(self.module);
            }
        }
    }

    fn create_vs(device: &ID3D11Device, bytecode: &[u8]) -> Result<ID3D11VertexShader> {
        let mut vs = None;
        unsafe { device.CreateVertexShader(bytecode, None, Some(&mut vs))? };
        Ok(vs.unwrap())
    }

    fn create_ps(device: &ID3D11Device, bytecode: &[u8]) -> Result<ID3D11PixelShader> {
        let mut ps = None;
        unsafe { device.CreatePixelShader(bytecode, None, Some(&mut ps))? };
        Ok(ps.unwrap())
    }

    fn elem(
        name: &'static [u8],
        index: u32,
        format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT,
        slot: u32,
        offset: u32,
        instanced: bool,
    ) -> D3D11_INPUT_ELEMENT_DESC {
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: PCSTR(name.as_ptr()),
            SemanticIndex: index,
            Format: format,
            InputSlot: slot,
            AlignedByteOffset: offset,
            InputSlotClass: if instanced {
                D3D11_INPUT_PER_INSTANCE_DATA
            } else {
                D3D11_INPUT_PER_VERTEX_DATA
            },
            InstanceDataStepRate: if instanced { 1 } else { 0 },
        }
    }

    fn make_rs(
        device: &ID3D11Device,
        cull: windows::Win32::Graphics::Direct3D11::D3D11_CULL_MODE,
        bias: i32,
        slope_bias: f32,
    ) -> Result<ID3D11RasterizerState> {
        let desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: cull,
            FrontCounterClockwise: BOOL(1),
            DepthBias: bias,
            DepthBiasClamp: 0.0,
            SlopeScaledDepthBias: slope_bias,
            DepthClipEnable: BOOL(1),
            ScissorEnable: BOOL(0),
            MultisampleEnable: BOOL(1),
            AntialiasedLineEnable: BOOL(0),
        };
        let mut out = None;
        unsafe { device.CreateRasterizerState(&desc, Some(&mut out))? };
        Ok(out.unwrap())
    }

    fn make_ds(device: &ID3D11Device, enable: bool, write: bool) -> Result<ID3D11DepthStencilState> {
        let desc = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: BOOL(i32::from(enable)),
            DepthWriteMask: if write { D3D11_DEPTH_WRITE_MASK_ALL } else { D3D11_DEPTH_WRITE_MASK_ZERO },
            DepthFunc: D3D11_COMPARISON_LESS_EQUAL,
            StencilEnable: BOOL(0),
            ..Default::default()
        };
        let mut out = None;
        unsafe { device.CreateDepthStencilState(&desc, Some(&mut out))? };
        Ok(out.unwrap())
    }

    fn make_blend(device: &ID3D11Device, premul: bool) -> Result<ID3D11BlendState> {
        let mut desc = D3D11_BLEND_DESC::default();
        desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
            BlendEnable: BOOL(i32::from(premul)),
            SrcBlend: D3D11_BLEND_ONE,
            DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D11_BLEND_OP_ADD,
            SrcBlendAlpha: D3D11_BLEND_ONE,
            DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D11_BLEND_OP_ADD,
            RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
        };
        let mut out = None;
        unsafe { device.CreateBlendState(&desc, Some(&mut out))? };
        Ok(out.unwrap())
    }

    fn make_sampler(device: &ID3D11Device) -> Result<ID3D11SamplerState> {
        let desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let mut out = None;
        unsafe { device.CreateSamplerState(&desc, Some(&mut out))? };
        Ok(out.unwrap())
    }

    fn make_cb(device: &ID3D11Device, bytes: usize) -> Result<ID3D11Buffer> {
        let aligned = (bytes + 15) & !15;
        make_dynamic_buf(device, aligned, D3D11_BIND_CONSTANT_BUFFER)
    }

    fn make_dynamic_buf(
        device: &ID3D11Device,
        bytes: usize,
        bind: windows::Win32::Graphics::Direct3D11::D3D11_BIND_FLAG,
    ) -> Result<ID3D11Buffer> {
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: bytes.max(16) as u32,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: bind.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let mut out = None;
        unsafe { device.CreateBuffer(&desc, None, Some(&mut out))? };
        Ok(out.unwrap())
    }

    fn upload_static(device: &ID3D11Device, mesh: &MeshData) -> Result<GpuMesh> {
        let vb_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.vertices.len() * mem::size_of::<Vertex>()) as u32,
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            ..Default::default()
        };
        let vb_init = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.vertices.as_ptr() as *const c_void,
            ..Default::default()
        };
        let mut vb = None;
        unsafe { device.CreateBuffer(&vb_desc, Some(&vb_init), Some(&mut vb))? };

        let ib_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.indices.len() * mem::size_of::<u32>()) as u32,
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0 as u32,
            ..Default::default()
        };
        let ib_init = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.indices.as_ptr() as *const c_void,
            ..Default::default()
        };
        let mut ib = None;
        unsafe { device.CreateBuffer(&ib_desc, Some(&ib_init), Some(&mut ib))? };
        Ok(GpuMesh {
            vb: vb.unwrap(),
            ib: ib.unwrap(),
            index_count: mesh.indices.len() as u32,
        })
    }

    fn upload_dynamic(device: &ID3D11Device, mesh: &MeshData) -> Result<DynamicMesh> {
        let vb = make_dynamic_buf(
            device,
            mesh.vertices.len() * mem::size_of::<Vertex>(),
            D3D11_BIND_VERTEX_BUFFER,
        )?;
        let ib_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.indices.len() * mem::size_of::<u32>()) as u32,
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0 as u32,
            ..Default::default()
        };
        let ib_init = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.indices.as_ptr() as *const c_void,
            ..Default::default()
        };
        let mut ib = None;
        unsafe { device.CreateBuffer(&ib_desc, Some(&ib_init), Some(&mut ib))? };
        Ok(DynamicMesh {
            vb,
            ib: ib.unwrap(),
            index_count: mesh.indices.len() as u32,
        })
    }

    fn write_struct<T: Copy>(ctx: &ID3D11DeviceContext, buf: &ID3D11Buffer, data: &T) {
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if ctx.Map(buf, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped)).is_ok() {
                ptr::copy_nonoverlapping(data as *const T as *const u8, mapped.pData as *mut u8, mem::size_of::<T>());
                ctx.Unmap(buf, 0);
            }
        }
    }

    fn write_buf<T: Copy>(ctx: &ID3D11DeviceContext, buf: &ID3D11Buffer, slice: &[T]) {
        if slice.is_empty() {
            return;
        }
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if ctx.Map(buf, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped)).is_ok() {
                ptr::copy_nonoverlapping(
                    slice.as_ptr() as *const u8,
                    mapped.pData as *mut u8,
                    std::mem::size_of_val(slice),
                );
                ctx.Unmap(buf, 0);
            }
        }
    }
}

#[cfg(windows)]
pub use imp::Renderer;
