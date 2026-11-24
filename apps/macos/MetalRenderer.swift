import Cocoa
import Metal
import QuartzCore

/// Metal (4x MSAA + Premultiplied Alpha) renderer driven by `keycat-ffi` frame commands.
/// Renders identically to the Windows D3D11 backend with zero WebKit overhead.
final class MetalRenderer {
    private struct GpuMesh {
        let vb: MTLBuffer
        let ib: MTLBuffer
        let indexCount: Int
    }

    private struct DynMesh {
        var vb: MTLBuffer
        let ib: MTLBuffer
        let indexCount: Int
        var capacityBytes: Int
    }

    let device: MTLDevice
    private let queue: MTLCommandQueue
    private let sampleCount: Int

    private let pipeBasicOpaque: MTLRenderPipelineState
    private let pipeBasicPremul: MTLRenderPipelineState
    private let pipeFurOpaque: MTLRenderPipelineState
    private let pipeInstOpaque: MTLRenderPipelineState
    private let pipeSpritePremul: MTLRenderPipelineState

    private let dsRW: MTLDepthStencilState
    private let dsRO: MTLDepthStencilState
    private let dsNone: MTLDepthStencilState
    private let sampler: MTLSamplerState

    private var staticMeshes: [GpuMesh?] = Array(repeating: nil, count: Int(KC_MESH_STATIC_COUNT))
    private var earCache: [UInt32: (fill: GpuMesh, rim: GpuMesh)] = [:]
    private var dynMeshes: [DynMesh?] = Array(repeating: nil, count: Int(KC_DYN_COUNT))
    private var kbInstBufs: [MTLBuffer?] = [nil, nil, nil]

    private let atlas: SpriteAtlas
    private var msaaTex: MTLTexture?
    private var depthTex: MTLTexture?
    private var targetW: Int = 0
    private var targetH: Int = 0

    init?(scene: OpaquePointer) {
        guard let device = MTLCreateSystemDefaultDevice(),
              let queue = device.makeCommandQueue() else {
            return nil
        }
        self.device = device
        self.queue = queue
        self.sampleCount = device.supportsTextureSampleCount(4) ? 4 : 1
        self.atlas = SpriteAtlas(device: device)

        // Load precompiled default.metallib from app bundle, falling back to runtime MSL source.
        let library: MTLLibrary
        if let lib = try? device.makeDefaultLibrary(bundle: Bundle.main) {
            library = lib
        } else if let url = Bundle.main.url(forResource: "Shaders", withExtension: "metal"),
                  let src = try? String(contentsOf: url, encoding: .utf8),
                  let lib = try? device.makeLibrary(source: src, options: nil) {
            library = lib
        } else {
            return nil
        }

        let vtxDesc = MetalRenderer.makeVertexDescriptor(instanced: false)
        let instDesc = MetalRenderer.makeVertexDescriptor(instanced: true)

        guard let pBasicO = MetalRenderer.makePipeline(device: device, lib: library, vs: "vs_basic", fs: "ps_basic", vd: vtxDesc, sampleCount: sampleCount, premul: false),
              let pBasicP = MetalRenderer.makePipeline(device: device, lib: library, vs: "vs_basic", fs: "ps_basic", vd: vtxDesc, sampleCount: sampleCount, premul: true),
              let pFurO = MetalRenderer.makePipeline(device: device, lib: library, vs: "vs_fur", fs: "ps_fur", vd: vtxDesc, sampleCount: sampleCount, premul: false),
              let pInstO = MetalRenderer.makePipeline(device: device, lib: library, vs: "vs_instanced", fs: "ps_instanced", vd: instDesc, sampleCount: sampleCount, premul: false),
              let pSpriteP = MetalRenderer.makePipeline(device: device, lib: library, vs: "vs_sprite", fs: "ps_sprite", vd: nil, sampleCount: sampleCount, premul: true),
              let dsRW = MetalRenderer.makeDepthState(device: device, test: true, write: true),
              let dsRO = MetalRenderer.makeDepthState(device: device, test: true, write: false),
              let dsNone = MetalRenderer.makeDepthState(device: device, test: false, write: false),
              let sampler = MetalRenderer.makeSampler(device: device) else {
            return nil
        }

        self.pipeBasicOpaque = pBasicO
        self.pipeBasicPremul = pBasicP
        self.pipeFurOpaque = pFurO
        self.pipeInstOpaque = pInstO
        self.pipeSpritePremul = pSpriteP
        self.dsRW = dsRW
        self.dsRO = dsRO
        self.dsNone = dsNone
        self.sampler = sampler

        uploadInitialMeshes(scene: scene)
    }

    private func uploadInitialMeshes(scene: OpaquePointer) {
        // Upload core static meshes; defer optional cup/hat meshes until actually drawn.
        for id in 0..<UInt32(KC_MESH_CUP_BODY) {
            let mv = kc_scene_static_mesh(scene, id)
            staticMeshes[Int(id)] = uploadStatic(mv)
        }
        // Upload core dynamic meshes; defer cup steam until first steam update.
        for id in 0..<UInt32(KC_DYN_STEAM_A) {
            let mv = kc_scene_dynamic_mesh(scene, id)
            dynMeshes[Int(id)] = uploadDynamic(mv)
        }
        uploadKeyboardInstances(scene: scene)
    }

    private func ensureStaticMesh(scene: OpaquePointer, id: UInt32) -> GpuMesh? {
        let idx = Int(id)
        if let existing = staticMeshes[idx] { return existing }
        let mv = kc_scene_static_mesh(scene, id)
        guard let uploaded = uploadStatic(mv) else { return nil }
        staticMeshes[idx] = uploaded
        return uploaded
    }

    private func uploadStatic(_ mv: KcMeshView) -> GpuMesh? {
        guard mv.vertex_count > 0, mv.index_count > 0,
              let vp = mv.vertices, let ip = mv.indices else { return nil }
        let vBytes = Int(mv.vertex_count) * MemoryLayout<KcVertex>.stride
        let iBytes = Int(mv.index_count) * MemoryLayout<UInt32>.stride
        guard let vb = device.makeBuffer(bytes: vp, length: vBytes, options: .storageModeShared),
              let ib = device.makeBuffer(bytes: ip, length: iBytes, options: .storageModeShared) else {
            return nil
        }
        return GpuMesh(vb: vb, ib: ib, indexCount: Int(mv.index_count))
    }

    private func uploadDynamic(_ mv: KcMeshView) -> DynMesh? {
        guard mv.vertex_count > 0, mv.index_count > 0,
              let vp = mv.vertices, let ip = mv.indices else { return nil }
        let vBytes = Int(mv.vertex_count) * MemoryLayout<KcVertex>.stride
        let iBytes = Int(mv.index_count) * MemoryLayout<UInt32>.stride
        guard let vb = device.makeBuffer(bytes: vp, length: vBytes, options: .storageModeShared),
              let ib = device.makeBuffer(bytes: ip, length: iBytes, options: .storageModeShared) else {
            return nil
        }
        return DynMesh(vb: vb, ib: ib, indexCount: Int(mv.index_count), capacityBytes: vBytes)
    }

    private func updateDynamic(id: UInt32, mv: KcMeshView) {
        guard mv.vertex_count > 0, let vp = mv.vertices else { return }
        let vBytes = Int(mv.vertex_count) * MemoryLayout<KcVertex>.stride
        let idx = Int(id)
        if let dm = dynMeshes[idx], dm.capacityBytes >= vBytes {
            memcpy(dm.vb.contents(), vp, vBytes)
        } else if let fresh = uploadDynamic(mv) {
            dynMeshes[idx] = fresh
        }
    }

    private func uploadKeyboardInstances(scene: OpaquePointer) {
        for slot in 0..<UInt32(3) {
            var count: UInt32 = 0
            guard let ptr = kc_scene_kb_instances(scene, slot, &count), count > 0 else { continue }
            let bytes = Int(count) * MemoryLayout<KcKeyInstance>.stride
            let idx = Int(slot)
            if let buf = kbInstBufs[idx], buf.length >= bytes {
                memcpy(buf.contents(), ptr, bytes)
            } else {
                kbInstBufs[idx] = device.makeBuffer(bytes: ptr, length: bytes, options: .storageModeShared)
            }
        }
    }

    private func ensureEar(scene: OpaquePointer, key: UInt32) -> (fill: GpuMesh, rim: GpuMesh)? {
        if let cached = earCache[key] { return cached }
        let fillMv = kc_scene_ear_mesh(scene, key, 0)
        let rimMv = kc_scene_ear_mesh(scene, key, 1)
        guard let fill = uploadStatic(fillMv), let rim = uploadStatic(rimMv) else { return nil }
        let pair = (fill: fill, rim: rim)
        earCache[key] = pair
        return pair
    }

    private func ensureTargets(width: Int, height: Int) {
        let w = max(width, 1)
        let h = max(height, 1)
        if w == targetW && h == targetH && depthTex != nil && (sampleCount == 1 || msaaTex != nil) {
            return
        }
        targetW = w
        targetH = h
        if sampleCount > 1 {
            let desc = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .bgra8Unorm, width: w, height: h, mipmapped: false)
            desc.textureType = .type2DMultisample
            desc.sampleCount = sampleCount
            desc.storageMode = .private
            desc.usage = [.renderTarget]
            msaaTex = device.makeTexture(descriptor: desc)
        } else {
            msaaTex = nil
        }
        let dDesc = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .depth32Float, width: w, height: h, mipmapped: false)
        dDesc.textureType = sampleCount > 1 ? .type2DMultisample : .type2D
        dDesc.sampleCount = sampleCount
        dDesc.storageMode = .private
        dDesc.usage = [.renderTarget]
        depthTex = device.makeTexture(descriptor: dDesc)
    }

    /// Releases offscreen MSAA and depth textures while the window is hidden to free GPU/unified memory.
    func releaseTargets() {
        msaaTex = nil
        depthTex = nil
        targetW = 0
        targetH = 0
    }

    /// Renders one frame onto a CAMetalLayer drawable.
    func render(scene: OpaquePointer, layer: CAMetalLayer) {
        let w = Int(layer.drawableSize.width)
        let h = Int(layer.drawableSize.height)
        guard w > 0, h > 0 else { return }
        autoreleasepool {
            guard let drawable = layer.nextDrawable() else { return }
            renderInto(scene: scene, resolveTarget: drawable.texture, present: drawable)
        }
    }

    /// Renders one frame offscreen and returns a premultiplied BGRA8 CGImage (for CI smoke tests).
    func renderOffscreen(scene: OpaquePointer, width: Int, height: Int) -> CGImage? {
        let desc = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .bgra8Unorm, width: width, height: height, mipmapped: false)
        desc.storageMode = device.hasUnifiedMemory ? .shared : .managed
        desc.usage = [.renderTarget, .shaderRead]
        guard let tex = device.makeTexture(descriptor: desc) else { return nil }

        guard let cmd = renderInto(scene: scene, resolveTarget: tex, present: nil) else { return nil }
        if tex.storageMode == .managed, let blit = cmd.makeBlitCommandEncoder() {
            blit.synchronize(resource: tex)
            blit.endEncoding()
        }
        cmd.commit()
        cmd.waitUntilCompleted()

        let bytesPerRow = width * 4
        var pixels = [UInt8](repeating: 0, count: bytesPerRow * height)
        tex.getBytes(&pixels, bytesPerRow: bytesPerRow, from: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0)

        let cs = CGColorSpaceCreateDeviceRGB()
        let info = CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue
        guard let provider = CGDataProvider(data: Data(pixels) as CFData) else { return nil }
        return CGImage(
            width: width,
            height: height,
            bitsPerComponent: 8,
            bitsPerPixel: 32,
            bytesPerRow: bytesPerRow,
            space: cs,
            bitmapInfo: CGBitmapInfo(rawValue: info),
            provider: provider,
            decode: nil,
            shouldInterpolate: false,
            intent: .defaultIntent
        )
    }

    @discardableResult
    private func renderInto(scene: OpaquePointer, resolveTarget: MTLTexture, present: CAMetalDrawable?) -> MTLCommandBuffer? {
        let w = resolveTarget.width
        let h = resolveTarget.height
        ensureTargets(width: w, height: h)
        guard let depthTex else { return nil }

        let frame = kc_scene_prepare_frame(scene)
        if frame.kb_dirty != 0 {
            uploadKeyboardInstances(scene: scene)
        }
        if frame.cable_dirty != 0 {
            updateDynamic(id: UInt32(KC_DYN_CABLE), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_CABLE)))
        }
        if (frame.cat_dyn_dirty & 1) != 0 {
            updateDynamic(id: UInt32(KC_DYN_TAIL_FILL), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_TAIL_FILL)))
            updateDynamic(id: UInt32(KC_DYN_TAIL_RIM), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_TAIL_RIM)))
        }
        if (frame.cat_dyn_dirty & 2) != 0 {
            updateDynamic(id: UInt32(KC_DYN_ARM_FILL_0), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_ARM_FILL_0)))
            updateDynamic(id: UInt32(KC_DYN_ARM_RIM_0), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_ARM_RIM_0)))
        }
        if (frame.cat_dyn_dirty & 4) != 0 {
            updateDynamic(id: UInt32(KC_DYN_ARM_FILL_1), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_ARM_FILL_1)))
            updateDynamic(id: UInt32(KC_DYN_ARM_RIM_1), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_ARM_RIM_1)))
        }
        if frame.steam_dirty != 0 {
            updateDynamic(id: UInt32(KC_DYN_STEAM_A), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_STEAM_A)))
            updateDynamic(id: UInt32(KC_DYN_STEAM_B), mv: kc_scene_dynamic_mesh(scene, UInt32(KC_DYN_STEAM_B)))
        }
        let activeEar = ensureEar(scene: scene, key: frame.ear_key)

        guard let cmd = queue.makeCommandBuffer() else { return nil }
        let pass = MTLRenderPassDescriptor()
        if let msaa = msaaTex, sampleCount > 1 {
            pass.colorAttachments[0].texture = msaa
            pass.colorAttachments[0].resolveTexture = resolveTarget
            pass.colorAttachments[0].storeAction = .multisampleResolve
        } else {
            pass.colorAttachments[0].texture = resolveTarget
            pass.colorAttachments[0].storeAction = .store
        }
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 0)

        pass.depthAttachment.texture = depthTex
        pass.depthAttachment.loadAction = .clear
        pass.depthAttachment.storeAction = .dontCare
        pass.depthAttachment.clearDepth = 1.0

        guard let enc = cmd.makeRenderCommandEncoder(descriptor: pass) else { return nil }
        enc.setViewport(MTLViewport(originX: 0, originY: 0, width: Double(w), height: Double(h), znear: 0, zfar: 1))
        enc.setFrontFacing(.counterClockwise)

        var frameCB = frame.frame_cb
        var furCB = frame.fur_cb
        enc.setVertexBytes(&frameCB, length: MemoryLayout<KcFrameCB>.stride, index: 2)
        enc.setFragmentBytes(&furCB, length: MemoryLayout<KcFurCB>.stride, index: 3)

        if let drawsPtr = frame.draws {
            let drawCount = Int(frame.draw_count)
            for i in 0..<drawCount {
                let d = drawsPtr[i]
                switch d.pipeline {
                case UInt32(KC_PIPE_FUR):
                    enc.setRenderPipelineState(pipeFurOpaque)
                case UInt32(KC_PIPE_INSTANCED):
                    enc.setRenderPipelineState(pipeInstOpaque)
                default:
                    enc.setRenderPipelineState(d.blend == UInt32(KC_BLEND_PREMUL) ? pipeBasicPremul : pipeBasicOpaque)
                }

                switch d.cull {
                case UInt32(KC_CULL_FRONT):
                    enc.setCullMode(.front)
                    enc.setDepthBias(0, slopeScale: 0, clamp: 0)
                case UInt32(KC_CULL_NONE):
                    enc.setCullMode(.none)
                    enc.setDepthBias(0, slopeScale: 0, clamp: 0)
                case UInt32(KC_CULL_DECAL):
                    enc.setCullMode(.back)
                    enc.setDepthBias(-200, slopeScale: -2.0, clamp: 0)
                default:
                    enc.setCullMode(.back)
                    enc.setDepthBias(0, slopeScale: 0, clamp: 0)
                }

                switch d.depth {
                case UInt32(KC_DEPTH_RO): enc.setDepthStencilState(dsRO)
                case UInt32(KC_DEPTH_NONE): enc.setDepthStencilState(dsNone)
                default: enc.setDepthStencilState(dsRW)
                }

                var obj = d.obj
                enc.setVertexBytes(&obj, length: MemoryLayout<KcObjCB>.stride, index: 4)
                enc.setFragmentBytes(&obj, length: MemoryLayout<KcObjCB>.stride, index: 4)

                let vb: MTLBuffer
                let ib: MTLBuffer
                let indexCount: Int
                if d.is_dynamic != 0 {
                    if dynMeshes[Int(d.mesh_id)] == nil {
                        dynMeshes[Int(d.mesh_id)] = uploadDynamic(kc_scene_dynamic_mesh(scene, d.mesh_id))
                    }
                    guard let dm = dynMeshes[Int(d.mesh_id)] else { continue }
                    vb = dm.vb; ib = dm.ib; indexCount = dm.indexCount
                } else if d.mesh_id == UInt32(KC_MESH_EAR_FILL) {
                    guard let eg = activeEar else { continue }
                    vb = eg.fill.vb; ib = eg.fill.ib; indexCount = eg.fill.indexCount
                } else if d.mesh_id == UInt32(KC_MESH_EAR_RIM) {
                    guard let eg = activeEar else { continue }
                    vb = eg.rim.vb; ib = eg.rim.ib; indexCount = eg.rim.indexCount
                } else {
                    guard let sm = ensureStaticMesh(scene: scene, id: d.mesh_id) else { continue }
                    vb = sm.vb; ib = sm.ib; indexCount = sm.indexCount
                }

                enc.setVertexBuffer(vb, offset: 0, index: 0)
                if d.pipeline == UInt32(KC_PIPE_INSTANCED) {
                    guard let instBuf = kbInstBufs[Int(d.inst_slot)] else { continue }
                    enc.setVertexBuffer(instBuf, offset: 0, index: 1)
                    enc.drawIndexedPrimitives(
                        type: .triangle,
                        indexCount: indexCount,
                        indexType: .uint32,
                        indexBuffer: ib,
                        indexBufferOffset: 0,
                        instanceCount: Int(d.instance_count)
                    )
                } else {
                    enc.drawIndexedPrimitives(
                        type: .triangle,
                        indexCount: indexCount,
                        indexType: .uint32,
                        indexBuffer: ib,
                        indexBufferOffset: 0
                    )
                }
            }
        }

        // Billboard sprites
        if let spritesPtr = frame.sprites, frame.sprite_count > 0 {
            enc.setRenderPipelineState(pipeSpritePremul)
            enc.setCullMode(.none)
            enc.setDepthBias(0, slopeScale: 0, clamp: 0)
            enc.setDepthStencilState(dsNone)
            enc.setFragmentSamplerState(sampler, index: 0)
            for i in 0..<Int(frame.sprite_count) {
                let sp = spritesPtr[i]
                let label = sp.label.map { String(cString: $0) } ?? ""
                guard let tex = atlas.texture(kind: sp.kind, label: label) else { continue }
                var obj = KcObjCB()
                obj.model = (
                    1, 0, 0, 0,
                    0, 1, 0, 0,
                    0, 0, 1, 0,
                    sp.pos.0, sp.pos.1, sp.pos.2, 1
                )
                obj.normal_mat = (1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)
                obj.color = (1, 1, 1, sp.opacity)
                obj.params = (sp.scale_x, sp.scale_y, 0, 0)
                enc.setVertexBytes(&obj, length: MemoryLayout<KcObjCB>.stride, index: 4)
                enc.setFragmentBytes(&obj, length: MemoryLayout<KcObjCB>.stride, index: 4)
                enc.setFragmentTexture(tex, index: 0)
                enc.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
            }
        }

        enc.endEncoding()
        if let present {
            cmd.present(present)
            cmd.commit()
            return nil
        }
        return cmd
    }

    // MARK: Pipeline helpers

    private static func makeVertexDescriptor(instanced: Bool) -> MTLVertexDescriptor {
        let vd = MTLVertexDescriptor()
        // Buffer 0: KcVertex (stride = 52)
        vd.attributes[0].format = .float3; vd.attributes[0].offset = 0;  vd.attributes[0].bufferIndex = 0
        vd.attributes[1].format = .float3; vd.attributes[1].offset = 12; vd.attributes[1].bufferIndex = 0
        vd.attributes[2].format = .float3; vd.attributes[2].offset = 24; vd.attributes[2].bufferIndex = 0
        vd.attributes[3].format = .float3; vd.attributes[3].offset = 36; vd.attributes[3].bufferIndex = 0
        vd.attributes[4].format = .float;  vd.attributes[4].offset = 48; vd.attributes[4].bufferIndex = 0
        vd.layouts[0].stride = MemoryLayout<KcVertex>.stride
        vd.layouts[0].stepFunction = .perVertex

        if instanced {
            // Buffer 1: KcKeyInstance (stride = 80)
            vd.attributes[5].format = .float4; vd.attributes[5].offset = 0;  vd.attributes[5].bufferIndex = 1
            vd.attributes[6].format = .float4; vd.attributes[6].offset = 16; vd.attributes[6].bufferIndex = 1
            vd.attributes[7].format = .float4; vd.attributes[7].offset = 32; vd.attributes[7].bufferIndex = 1
            vd.attributes[8].format = .float4; vd.attributes[8].offset = 48; vd.attributes[8].bufferIndex = 1
            vd.attributes[9].format = .float4; vd.attributes[9].offset = 64; vd.attributes[9].bufferIndex = 1
            vd.layouts[1].stride = MemoryLayout<KcKeyInstance>.stride
            vd.layouts[1].stepFunction = .perInstance
        }
        return vd
    }

    private static func makePipeline(
        device: MTLDevice,
        lib: MTLLibrary,
        vs: String,
        fs: String,
        vd: MTLVertexDescriptor?,
        sampleCount: Int,
        premul: Bool
    ) -> MTLRenderPipelineState? {
        guard let vfn = lib.makeFunction(name: vs), let ffn = lib.makeFunction(name: fs) else { return nil }
        let desc = MTLRenderPipelineDescriptor()
        desc.vertexFunction = vfn
        desc.fragmentFunction = ffn
        desc.vertexDescriptor = vd
        desc.rasterSampleCount = sampleCount
        desc.depthAttachmentPixelFormat = .depth32Float
        let ca = desc.colorAttachments[0]!
        ca.pixelFormat = .bgra8Unorm
        if premul {
            ca.isBlendingEnabled = true
            ca.sourceRGBBlendFactor = .one
            ca.destinationRGBBlendFactor = .oneMinusSourceAlpha
            ca.rgbBlendOperation = .add
            ca.sourceAlphaBlendFactor = .one
            ca.destinationAlphaBlendFactor = .oneMinusSourceAlpha
            ca.alphaBlendOperation = .add
        } else {
            ca.isBlendingEnabled = false
        }
        return try? device.makeRenderPipelineState(descriptor: desc)
    }

    private static func makeDepthState(device: MTLDevice, test: Bool, write: Bool) -> MTLDepthStencilState? {
        let desc = MTLDepthStencilDescriptor()
        desc.depthCompareFunction = test ? .lessEqual : .always
        desc.isDepthWriteEnabled = write
        return device.makeDepthStencilState(descriptor: desc)
    }

    private static func makeSampler(device: MTLDevice) -> MTLSamplerState? {
        let desc = MTLSamplerDescriptor()
        desc.minFilter = .linear
        desc.magFilter = .linear
        desc.sAddressMode = .clampToEdge
        desc.tAddressMode = .clampToEdge
        return device.makeSamplerState(descriptor: desc)
    }
}
