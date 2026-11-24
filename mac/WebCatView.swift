import Cocoa
import WebKit

/// The web scene's host view. It keeps every mouse event for itself: the page never
/// scrolls or selects; a press on the head pets the cat and a press elsewhere drags the window.
final class WebCatView: WKWebView {
    weak var app: AppDelegate?
    private var dragStart: (mouse: CGPoint, origin: CGPoint)?
    private var petting = false

    override func hitTest(_ point: NSPoint) -> NSView? {
        bounds.contains(convert(point, from: superview)) ? self : nil
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    override func scrollWheel(with event: NSEvent) {}
    override func menu(for event: NSEvent) -> NSMenu? { nil }
    override func mouseMoved(with event: NSEvent) { app?.refreshCursor() }

    /// Event location in the page's CSS pixel space (top-left origin).
    func pagePoint(_ event: NSEvent) -> CGPoint {
        let p = convert(event.locationInWindow, from: nil)
        return CGPoint(x: p.x, y: bounds.height - p.y)
    }

    override func mouseDown(with event: NSEvent) {
        guard let app, let window else { return }
        if app.hit == "pet" {
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

    /// Safely clears any in-progress pet or drag (e.g. on mouseUp, panel hide, or interrupted click).
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

    override func rightMouseDown(with event: NSEvent) { app?.showMenu(event) }
}

/// Breaks the retain cycle between WKUserContentController and the app delegate.
final class WeakScriptHandler: NSObject, WKScriptMessageHandler {
    weak var target: WKScriptMessageHandler?
    init(_ target: WKScriptMessageHandler) { self.target = target }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        target?.userContentController(controller, didReceive: message)
    }
}
