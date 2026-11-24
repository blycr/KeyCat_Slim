import Cocoa
import CoreGraphics
import Metal

/// CoreGraphics + AppKit offscreen texture baker for keycap labels, sleep "Z" and FX stickers.
/// Bakes premultiplied BGRA8 textures matching MTLPixelFormat.bgra8Unorm and caches them by key.
final class SpriteAtlas {
    private let device: MTLDevice
    private var cache: [String: MTLTexture] = [:]

    init(device: MTLDevice) {
        self.device = device
    }

    func texture(kind: UInt32, label: String) -> MTLTexture? {
        let key = "\(kind):\(label)"
        if let existing = cache[key] { return existing }
        guard let tex = bake(kind: kind, label: label) else { return nil }
        cache[key] = tex
        return tex
    }

    private func bake(kind: UInt32, label: String) -> MTLTexture? {
        let w: Int
        if kind == UInt32(KC_SPRITE_BUBBLE) {
            w = 512
        } else if kind == UInt32(KC_SPRITE_CAP) && label.count > 1 {
            w = 256
        } else {
            w = 128
        }
        let h = 128
        let bytesPerRow = w * 4
        var pixels = [UInt8](repeating: 0, count: bytesPerRow * h)

        let colorSpace = CGColorSpaceCreateDeviceRGB()
        let bitmapInfo = CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue
        let ok: Bool = pixels.withUnsafeMutableBytes { buf in
            guard let ctx = CGContext(
                data: buf.baseAddress,
                width: w,
                height: h,
                bitsPerComponent: 8,
                bytesPerRow: bytesPerRow,
                space: colorSpace,
                bitmapInfo: bitmapInfo
            ) else { return false }

            ctx.setAllowsAntialiasing(true)
            ctx.setShouldAntialias(true)
            ctx.setAllowsFontSmoothing(true)
            ctx.setShouldSmoothFonts(true)

            // Flip to top-left origin so drawing coordinates match textures.rs and UV (0,0) top-left.
            ctx.translateBy(x: 0, y: CGFloat(h))
            ctx.scaleBy(x: 1, y: -1)

            switch kind {
            case UInt32(KC_SPRITE_CAP):
                drawCap(ctx, w: CGFloat(w), label: label)
            case UInt32(KC_SPRITE_SLEEP_Z):
                drawSleepZ(ctx)
            case UInt32(KC_SPRITE_ANGER):
                drawAnger(ctx)
            case UInt32(KC_SPRITE_BANG):
                drawBang(ctx)
            case UInt32(KC_SPRITE_HEART):
                drawHeart(ctx)
            case UInt32(KC_SPRITE_BUBBLE):
                drawBubble(ctx, w: CGFloat(w), text: label)
            default:
                drawDust(ctx)
            }
            return true
        }
        guard ok else { return nil }

        let desc = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .bgra8Unorm,
            width: w,
            height: h,
            mipmapped: false
        )
        desc.usage = [.shaderRead]
        guard let tex = device.makeTexture(descriptor: desc) else { return nil }
        pixels.withUnsafeBytes { buf in
            tex.replace(
                region: MTLRegionMake2D(0, 0, w, h),
                mipmapLevel: 0,
                withBytes: buf.baseAddress!,
                bytesPerRow: bytesPerRow
            )
        }
        return tex
    }

    private func rgba(_ r: UInt8, _ g: UInt8, _ b: UInt8, _ a: CGFloat = 1.0) -> CGColor {
        CGColor(srgbRed: CGFloat(r) / 255.0, green: CGFloat(g) / 255.0, blue: CGFloat(b) / 255.0, alpha: a)
    }

    private func drawCenteredText(_ ctx: CGContext, text: String, font: NSFont, color: NSColor, rect: CGRect) {
        let style = NSMutableParagraphStyle()
        style.alignment = .center
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: color,
            .paragraphStyle: style,
        ]
        let attrStr = NSAttributedString(string: text, attributes: attrs)
        let size = attrStr.size()
        let drawRect = CGRect(
            x: rect.origin.x,
            y: rect.origin.y + (rect.height - size.height) * 0.5,
            width: rect.width,
            height: size.height
        )
        let nsCtx = NSGraphicsContext(cgContext: ctx, flipped: true)
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = nsCtx
        attrStr.draw(in: drawRect)
        NSGraphicsContext.restoreGraphicsState()
    }

    private func drawCap(_ ctx: CGContext, w: CGFloat, label: String) {
        let x: CGFloat = 8
        let y: CGFloat = 8
        let rw = w - 16
        let rh: CGFloat = 96
        let r: CGFloat = 26

        let shadowPath = CGPath(roundedRect: CGRect(x: x, y: y + 12, width: rw, height: rh), cornerWidth: r, cornerHeight: r, transform: nil)
        ctx.setFillColor(rgba(40, 40, 40, 0.55))
        ctx.addPath(shadowPath)
        ctx.fillPath()

        let boxPath = CGPath(roundedRect: CGRect(x: x, y: y, width: rw, height: rh), cornerWidth: r, cornerHeight: r, transform: nil)
        ctx.setFillColor(rgba(0xfb, 0xfa, 0xf3, 1.0))
        ctx.addPath(boxPath)
        ctx.fillPath()

        ctx.setStrokeColor(rgba(0x2a, 0x2a, 0x2a, 1.0))
        ctx.setLineWidth(5)
        ctx.addPath(boxPath)
        ctx.strokePath()

        let fontSize: CGFloat = label.count > 1 ? 42 : 56
        let font = NSFont.systemFont(ofSize: fontSize, weight: .bold)
        let ink = NSColor(srgbRed: 0x1b / 255.0, green: 0x1b / 255.0, blue: 0x1b / 255.0, alpha: 1.0)
        drawCenteredText(ctx, text: label, font: font, color: ink, rect: CGRect(x: x, y: y, width: rw, height: rh))
    }

    private func drawSleepZ(_ ctx: CGContext) {
        let font = NSFont(name: "MarkerFelt-Wide", size: 80) ?? NSFont.systemFont(ofSize: 80, weight: .heavy)
        let strokeColor = NSColor(srgbRed: 0x1b / 255.0, green: 0x1b / 255.0, blue: 0x1b / 255.0, alpha: 1.0)
        let cream = NSColor(srgbRed: 0xfb / 255.0, green: 0xfa / 255.0, blue: 0xf3 / 255.0, alpha: 1.0)
        let r: CGFloat = 6.0
        for i in 0..<12 {
            let a = (CGFloat(i) / 12.0) * .pi * 2.0
            let rect = CGRect(x: cos(a) * r, y: 2.0 + sin(a) * r, width: 128, height: 128)
            drawCenteredText(ctx, text: "Z", font: font, color: strokeColor, rect: rect)
        }
        drawCenteredText(ctx, text: "Z", font: font, color: cream, rect: CGRect(x: 0, y: 2, width: 128, height: 128))
    }

    private func drawAnger(_ ctx: CGContext) {
        let path = CGMutablePath()
        let signs: [(CGFloat, CGFloat)] = [(-1, -1), (1, -1), (1, 1), (-1, 1)]
        for (sx, sy) in signs {
            path.move(to: CGPoint(x: 64 + sx * 14, y: 64 + sy * 46))
            path.addQuadCurve(to: CGPoint(x: 64 + sx * 46, y: 64 + sy * 14),
                              control: CGPoint(x: 64 + sx * 14, y: 64 + sy * 14))
        }
        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        ctx.setStrokeColor(rgba(0xfb, 0xfa, 0xf3, 1.0))
        ctx.setLineWidth(30)
        ctx.addPath(path)
        ctx.strokePath()

        ctx.setStrokeColor(rgba(0xe2, 0x48, 0x3d, 1.0))
        ctx.setLineWidth(15)
        ctx.addPath(path)
        ctx.strokePath()
    }

    private func drawBang(_ ctx: CGContext) {
        let path = CGMutablePath()
        for i in 0..<20 {
            let a = (CGFloat(i) / 20.0) * .pi * 2.0
            let r: CGFloat = (i % 2 == 1) ? 26 : 58
            let p = CGPoint(x: 64 + cos(a) * r, y: 64 + sin(a) * r)
            if i == 0 { path.move(to: p) } else { path.addLine(to: p) }
        }
        path.closeSubpath()

        ctx.setFillColor(rgba(0xfb, 0xfa, 0xf3, 1.0))
        ctx.addPath(path)
        ctx.fillPath()

        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        ctx.setStrokeColor(rgba(0x1d, 0x1d, 0x1b, 1.0))
        ctx.setLineWidth(7)
        ctx.addPath(path)
        ctx.strokePath()
    }

    private func drawDust(_ ctx: CGContext) {
        let rect = CGRect(x: 12, y: 12, width: 104, height: 104)
        ctx.setFillColor(rgba(0xf4, 0xf1, 0xe6, 1.0))
        ctx.fillEllipse(in: rect)
        ctx.setStrokeColor(rgba(0xc9, 0xc4, 0xb4, 1.0))
        ctx.setLineWidth(6)
        ctx.strokeEllipse(in: rect)
    }

    private func drawHeart(_ ctx: CGContext) {
        let path = CGMutablePath()
        path.move(to: CGPoint(x: 64, y: 108))
        path.addQuadCurve(to: CGPoint(x: 18, y: 44), control: CGPoint(x: 14, y: 74))
        path.addQuadCurve(to: CGPoint(x: 46, y: 22), control: CGPoint(x: 20, y: 18))
        path.addQuadCurve(to: CGPoint(x: 64, y: 38), control: CGPoint(x: 58, y: 24))
        path.addQuadCurve(to: CGPoint(x: 82, y: 22), control: CGPoint(x: 70, y: 24))
        path.addQuadCurve(to: CGPoint(x: 110, y: 44), control: CGPoint(x: 108, y: 18))
        path.addQuadCurve(to: CGPoint(x: 64, y: 108), control: CGPoint(x: 114, y: 74))
        path.closeSubpath()

        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        ctx.setStrokeColor(rgba(0xfb, 0xfa, 0xf3, 1.0))
        ctx.setLineWidth(12)
        ctx.addPath(path)
        ctx.strokePath()

        ctx.setFillColor(rgba(0xf2, 0x7c, 0x94, 1.0))
        ctx.addPath(path)
        ctx.fillPath()
    }

    private func drawBubble(_ ctx: CGContext, w: CGFloat, text: String) {
        let shadowRect = CGRect(x: 10, y: 18, width: w - 20, height: 94)
        let shadowPath = CGPath(roundedRect: shadowRect, cornerWidth: 44, cornerHeight: 44, transform: nil)
        ctx.setFillColor(rgba(30, 26, 22, 0.32))
        ctx.addPath(shadowPath)
        ctx.fillPath()

        let boxRect = CGRect(x: 10, y: 10, width: w - 20, height: 94)
        let boxPath = CGPath(roundedRect: boxRect, cornerWidth: 44, cornerHeight: 44, transform: nil)
        ctx.setFillColor(rgba(0xfb, 0xf8, 0xf1, 0.98))
        ctx.addPath(boxPath)
        ctx.fillPath()

        ctx.setStrokeColor(rgba(0x3a, 0x32, 0x2b, 1.0))
        ctx.setLineWidth(5)
        ctx.addPath(boxPath)
        ctx.strokePath()

        let font = NSFont.systemFont(ofSize: 36, weight: .bold)
        let ink = NSColor(srgbRed: 0x23 / 255.0, green: 0x1e / 255.0, blue: 0x1a / 255.0, alpha: 1.0)
        drawCenteredText(ctx, text: text, font: font, color: ink, rect: CGRect(x: 24, y: 10, width: w - 48, height: 94))
    }
}
