import Cocoa
import Metal
import QuartzCore

/// Transparent CAMetalLayer-backed view hosting the native 3D cat with zero WebKit overhead.
final class MetalCatView: NSView {
    weak var app: AppDelegate?
    let metalLayer = CAMetalLayer()

    private var dragStart: (mouse: CGPoint, origin: CGPoint)?
    private var petting = false

    init(frame: CGRect, device: MTLDevice) {
        super.init(frame: frame)
        wantsLayer = true
        metalLayer.device = device
        metalLayer.pixelFormat = .bgra8Unorm
        metalLayer.isOpaque = false
        metalLayer.backgroundColor = CGColor(red: 0, green: 0, blue: 0, alpha: 0)
        metalLayer.framebufferOnly = false
        metalLayer.displaySyncEnabled = false
        self.layer = metalLayer
        syncDrawableSize()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) not used") }

    override func makeBackingLayer() -> CALayer { metalLayer }
    override var isOpaque: Bool { false }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    override func scrollWheel(with event: NSEvent) {}
    override func menu(for event: NSEvent) -> NSMenu? { nil }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        syncDrawableSize()
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        syncDrawableSize()
    }

    func syncDrawableSize() {
        let scale = window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 2.0
        metalLayer.contentsScale = scale
        metalLayer.frame = bounds
        metalLayer.drawableSize = CGSize(
            width: max(1, (bounds.width * scale).rounded()),
            height: max(1, (bounds.height * scale).rounded())
        )
    }

    override func mouseMoved(with event: NSEvent) {
        app?.refreshCursor()
    }

    /// Event location in CSS/logical pixel space (top-left origin, matching keycat-core).
    func pagePoint(_ event: NSEvent) -> CGPoint {
        let p = convert(event.locationInWindow, from: nil)
        return CGPoint(x: p.x, y: bounds.height - p.y)
    }

    override func mouseDown(with event: NSEvent) {
        guard let app, let window else { return }
        if app.hit == UInt32(KC_HIT_PET) {
            if event.clickCount == 2 {
                app.doubleClickHead()
            }
            petting = true
            app.pet(down: true, x: pagePoint(event).x)
        } else {
            dragStart = (NSEvent.mouseLocation, window.frame.origin)
            app.dragging = true
            NSCursor.closedHand.set()
        }
    }

    override func mouseDragged(with event: NSEvent) {
        guard let start = dragStart, let window else { return }
        let m = NSEvent.mouseLocation
        window.setFrameOrigin(CGPoint(x: start.origin.x + m.x - start.mouse.x, y: start.origin.y + m.y - start.mouse.y))
    }

    override func mouseUp(with event: NSEvent) {
        cancelInteractions()
        app?.refreshCursor()
    }

    func cancelInteractions() {
        if petting {
            petting = false
            app?.pet(down: false, x: 0)
        }
        if dragStart != nil {
            dragStart = nil
            app?.dragging = false
            app?.saveOrigin()
        }
    }

    override func rightMouseDown(with event: NSEvent) {
        app?.showMenu(event)
    }
}
