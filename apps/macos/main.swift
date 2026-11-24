import Cocoa
import Metal

/// Headless smoke test (`KeyCat --smoke-test [out.png]`): verifies keycat-core FFI, hit testing,
/// and (when a Metal GPU is available on the runner) renders an offscreen 720x736 frame and checks
/// that transparent corners stay clear while the cat/keyboard pixels are drawn.
@MainActor
private func runSmokeTest(outputPath: String?) -> Int32 {
    guard let scene = kc_scene_new() else {
        fputs("smoke-test: kc_scene_new returned nil\n", stderr)
        return 1
    }
    defer { kc_scene_free(scene) }

    kc_scene_set_viewport(scene, Float(designSize.width), Float(designSize.height))
    "orange".withCString { coat in
        "tabby".withCString { pat in
            "gold".withCString { eye in
                "sparkle".withCString { shine in
                    "red".withCString { collar in
                        kc_scene_set_look(scene, 1, coat, pat, eye, shine, collar, 1, 1)
                    }
                }
            }
        }
    }
    "KeyF".withCString { kc_scene_on_key(scene, $0, 1) }
    for _ in 0..<12 {
        kc_scene_step(scene, 1.0 / 60.0)
    }

    let headHit = kc_scene_on_cursor(scene, 171, 135)
    let bodyHit = kc_scene_on_cursor(scene, 171, 240)
    let emptyHit = kc_scene_on_cursor(scene, 20, 20)
    print("smoke-test hits: head=\(headHit) body=\(bodyHit) empty=\(emptyHit)")
    guard headHit == UInt32(KC_HIT_PET), bodyHit == UInt32(KC_HIT_DRAG), emptyHit == UInt32(KC_HIT_NONE) else {
        fputs("smoke-test: unexpected hit test results\n", stderr)
        return 1
    }

    let frame = kc_scene_prepare_frame(scene)
    print("smoke-test frame: draws=\(frame.draw_count) sprites=\(frame.sprite_count)")
    guard frame.draw_count >= 50, frame.sprite_count >= 1 else {
        fputs("smoke-test: insufficient draws or sprites\n", stderr)
        return 1
    }

    if let renderer = MetalRenderer(scene: scene) {
        guard let img = renderer.renderOffscreen(scene: scene, width: 720, height: 736) else {
            fputs("smoke-test: Metal offscreen render failed\n", stderr)
            return 1
        }
        let rep = NSBitmapImageRep(cgImage: img)
        let corner = rep.colorAt(x: 4, y: 4)?.alphaComponent ?? 1.0
        var opaqueCount = 0
        for y in stride(from: 200, to: 650, by: 16) {
            for x in stride(from: 120, to: 600, by: 16) {
                if (rep.colorAt(x: x, y: y)?.alphaComponent ?? 0) > 0.5 {
                    opaqueCount += 1
                }
            }
        }
        print("smoke-test Metal offscreen: cornerAlpha=\(corner) opaqueSamples=\(opaqueCount)")
        guard corner < 0.05, opaqueCount > 50 else {
            fputs("smoke-test: rendered pixels look wrong\n", stderr)
            return 1
        }
        if let out = outputPath, let png = rep.representation(using: .png, properties: [:]) {
            try? png.write(to: URL(fileURLWithPath: out))
            print("smoke-test wrote \(out)")
        }
    } else {
        // Headless macOS CI VMs without paravirtualized GPU may return nil from MTLCreateSystemDefaultDevice.
        print("smoke-test: no Metal device in headless VM; FFI & scene checks passed")
    }
    return 0
}

let arguments = CommandLine.arguments
if let idx = arguments.firstIndex(of: "--smoke-test") {
    let out = idx + 1 < arguments.count ? arguments[idx + 1] : nil
    let code = MainActor.assumeIsolated { runSmokeTest(outputPath: out) }
    exit(code)
}

let app = NSApplication.shared
let delegate = MainActor.assumeIsolated { AppDelegate() }
app.delegate = delegate
app.run()
