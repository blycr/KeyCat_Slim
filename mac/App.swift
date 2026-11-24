import Cocoa
import WebKit

/// Window size at scale 1, in points: the crop of the web scene's frame set in web/core.js
/// (`CROP`). At scale 1 the cat is drawn at the size it has on the reference web page.
let designSize = CGSize(width: 360, height: 368)

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate, WKScriptMessageHandler, WKNavigationDelegate {
    var panel: NSPanel!
    var webView: WebCatView!
    var statusItem: NSStatusItem!
    var timer: Timer?
    var tapRetry: Timer?
    var statusTimer: Timer?
    var eventTap: CFMachPort?
    var keyboardAllowed = true
    var pageReady = false
    var hit = "none"
    var dragging = false
    var pendingDelta = CGPoint.zero
    var lastCursor = CGPoint(x: -1, y: -1)
    var heldModifiers = Set<Int>()
    /// Count only, for status.txt; which keys were pressed is never kept.
    var keyEvents = 0
    let defaults = UserDefaults.standard
    lazy var settings = SettingsWindow(app: self)
    lazy var updater = Updater(app: self)

    var scale: CGFloat {
        get { CGFloat(defaults.object(forKey: "catScale") as? Double ?? 0.9) }
        set { defaults.set(Double(newValue), forKey: "catScale") }
    }
    var coat: String {
        get { defaults.string(forKey: "coat") ?? "black" }
        set { defaults.set(newValue, forKey: "coat") }
    }
    /// "2d" (flat, the default) or "3d" (soft shading).
    var style: String {
        get { defaults.string(forKey: "style") ?? "2d" }
        set { defaults.set(newValue, forKey: "style") }
    }
    /// Stripes are remembered per coat: "auto" lets the page pick (orange is a tabby, grey is plain).
    var pattern: String {
        get { defaults.string(forKey: "pattern.\(coat)") ?? "auto" }
        set { defaults.set(newValue, forKey: "pattern.\(coat)") }
    }
    var eyeColor: String {
        get { defaults.string(forKey: "eyeColor") ?? "auto" }
        set { defaults.set(newValue, forKey: "eyeColor") }
    }
    var eyeShine: String {
        get { defaults.string(forKey: "eyeShine") ?? "classic" }
        set { defaults.set(newValue, forKey: "eyeShine") }
    }
    var blush: Bool {
        get { defaults.bool(forKey: "blush") }
        set { defaults.set(newValue, forKey: "blush") }
    }
    var whiskers: Bool {
        get { defaults.object(forKey: "whiskers") as? Bool ?? true }
        set { defaults.set(newValue, forKey: "whiskers") }
    }
    var collar: String {
        get { defaults.string(forKey: "collar") ?? "none" }
        set { defaults.set(newValue, forKey: "collar") }
    }
    /// Behaviour settings from the 调节 window, only those changed from the defaults in web/tuning.js.
    var tune: [String: Any] {
        get { defaults.dictionary(forKey: "tune") ?? [:] }
        set { defaults.set(newValue, forKey: "tune") }
    }
    var privacy: Bool {
        get { defaults.bool(forKey: "privacy") }
        set { defaults.set(newValue, forKey: "privacy") }
    }
    /// Clicks always pass through to the window underneath; the menu is then reached from the menu bar.
    var clickThrough: Bool {
        get { defaults.bool(forKey: "clickThrough") }
        set {
            defaults.set(newValue, forKey: "clickThrough")
            if !dragging { panel.ignoresMouseEvents = newValue || hit == "none" }
        }
    }
    var sleepAfter: Int {
        get { defaults.object(forKey: "sleepAfter") as? Int ?? 60 }
        set { defaults.set(newValue, forKey: "sleepAfter") }
    }
    /// Frame-rate mode: "auto" (default: min(60, displayHz) active, auto step-down when idle/asleep),
    /// fixed integer (30 / 60 / high refresh, never throttles down), or 0 for unlimited.
    enum FpsMode: Equatable {
        case auto
        case fixed(Int)
        case unlimited

        var jsonValue: Any {
            switch self {
            case .auto: return "auto"
            case .fixed(let n): return n
            case .unlimited: return 0
            }
        }
    }

    var fps: FpsMode {
        get {
            if let s = defaults.string(forKey: "fps"), s == "auto" { return .auto }
            if let n = defaults.object(forKey: "fps") as? Int {
                if n == 0 { return .unlimited }
                if (15...1000).contains(n) { return .fixed(n) }
            }
            return .auto
        }
        set {
            switch newValue {
            case .auto: defaults.set("auto", forKey: "fps")
            case .fixed(let n): defaults.set(n, forKey: "fps")
            case .unlimited: defaults.set(0, forKey: "fps")
            }
        }
    }
    var debugStatusEnabled: Bool {
        ProcessInfo.processInfo.environment["KEYCAT_DEBUG_STATUS"] != nil
    }
    /// Highest native refresh rate among connected displays (falling back to 60).
    func displayMaxFPS() -> Int {
        let hz = NSScreen.screens.map(\.maximumFramesPerSecond).max() ?? 60
        return (15...1000).contains(hz) ? hz : 60
    }
    /// Highest native refresh rate (> 60 Hz) among connected displays, if any.
    func detectedHighFPS() -> Int? {
        let maxHz = displayMaxFPS()
        return maxHz > 60 ? maxHz : nil
    }
    private func pointerTickInterval() -> TimeInterval {
        let displayHz = displayMaxFPS()
        let target: Int
        switch fps {
        case .auto: target = min(60, max(15, displayHz))
        case .fixed(let n): target = min(max(n, 15), 240)
        case .unlimited: target = min(max(displayHz, 60), 240)
        }
        return 1.0 / Double(target)
    }

    func startPointerTimer() {
        timer?.invalidate()
        guard panel.isVisible else { timer = nil; return }
        let t = Timer(timeInterval: pointerTickInterval(), repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.tick() }
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    func stopPointerTimer() {
        timer?.invalidate()
        timer = nil
    }

    func setPanelVisible(_ visible: Bool) {
        if visible {
            panel.orderFrontRegardless()
            js("cat.setPaused(false)")
            startPointerTimer()
        } else {
            stopPointerTimer()
            webView.cancelInteractions()
            hit = "none"
            dragging = false
            pendingDelta = .zero
            panel.ignoresMouseEvents = true
            js("cat.setPaused(true)")
            panel.orderOut(nil)
        }
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        panel = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary, .ignoresCycle]
        panel.hidesOnDeactivate = false
        panel.isMovable = false
        panel.ignoresMouseEvents = true

        webView = makeWebView()
        panel.contentView = webView
        applyScale(scale, keepOrigin: false)
        panel.orderFrontRegardless()

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.title = "🐈‍⬛"
        let menu = NSMenu()
        menu.delegate = self
        statusItem.menu = menu

        installMouseMonitors()
        startKeyboardTap()
        startPointerTimer()
        updater.start()
    }

    func applicationWillTerminate(_ notification: Notification) {
        updater.install()
    }

    func makeWebView() -> WebCatView {
        let config = WKWebViewConfiguration()
        config.userContentController.add(WeakScriptHandler(self), name: "cat")
        config.suppressesIncrementalRendering = true
        let view = WebCatView(frame: .zero, configuration: config)
        view.app = self
        view.setValue(false, forKey: "drawsBackground")
        view.underPageBackgroundColor = .clear
        view.navigationDelegate = self
        view.autoresizingMask = [.width, .height]
        if let index = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "web") {
            view.loadFileURL(index, allowingReadAccessTo: index.deletingLastPathComponent())
            log("loading \(index.path)")
        } else {
            log("web/index.html missing from the app bundle")
        }
        return view
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        log("page loaded")
        pageReady = true
        pushOptions()
        if !panel.isVisible {
            js("cat.setPaused(true)")
        }
        if debugStatusEnabled {
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in self?.reportStatus() }
            statusTimer?.invalidate()
            statusTimer = Timer.scheduledTimer(withTimeInterval: 15, repeats: true) { [weak self] _ in
                MainActor.assumeIsolated { self?.reportStatus() }
            }
        }
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        log("page failed: \(error.localizedDescription)")
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        log("page failed to start: \(error.localizedDescription)")
    }

    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        log("web content process ended; reloading")
        pageReady = false
        webView.reload()
    }

    /// ~/Library/Application Support/KeyCat/<name>, creating the folder if needed.
    func supportFile(_ name: String) -> URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("KeyCat")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent(name)
    }

    /// Appends to log.txt in the support folder (lifecycle only, never input), rotating at 1 MB.
    func log(_ text: String) {
        let url = supportFile("log.txt")
        if let attrs = try? FileManager.default.attributesOfItem(atPath: url.path),
           let size = attrs[.size] as? NSNumber, size.int64Value > (1 << 20) {
            let old = supportFile("log.old.txt")
            try? FileManager.default.removeItem(at: old)
            try? FileManager.default.moveItem(at: url, to: old)
        }
        let line = "\(Date()) \(text)\n".data(using: .utf8)!
        if let handle = try? FileHandle(forWritingTo: url) {
            handle.seekToEndOfFile()
            handle.write(line)
            try? handle.close()
        } else {
            try? line.write(to: url)
        }
    }

    func pushOptions() {
        let options: [String: Any] = ["style": style, "coat": coat, "pattern": pattern, "eye": eyeColor, "shine": eyeShine, "blush": blush,
                                      "whiskers": whiskers, "collar": collar,
                                      "privacy": privacy, "sleepAfter": sleepAfter > 0 ? sleepAfter : 1e9,
                                      "fps": fps.jsonValue, "displayHz": displayMaxFPS(), "tune": tune]
        guard let data = try? JSONSerialization.data(withJSONObject: options),
              let json = String(data: data, encoding: .utf8) else { return }
        js("cat.options(\(json))")
    }

    func js(_ script: String) {
        guard pageReady else { return }
        webView.evaluateJavaScript(script, completionHandler: nil)
    }

    // MARK: Frame loop: forward the pointer to the page.

    func tick() {
        guard panel.isVisible else { return }
        let screen = NSEvent.mouseLocation
        let inWindow = panel.convertPoint(fromScreen: screen)
        let page = CGPoint(x: inWindow.x.rounded(), y: (panel.frame.height - inWindow.y).rounded())
        var calls: [String] = []
        if page != lastCursor {
            lastCursor = page
            calls.append("cat.cursor(\(Int(page.x)),\(Int(page.y)))")
        }
        if pendingDelta != .zero {
            calls.append(String(format: "cat.mouseMove(%.1f,%.1f)", pendingDelta.x, pendingDelta.y))
            pendingDelta = .zero
        }
        if !calls.isEmpty { js(calls.joined(separator: ";")) }
    }

    /// The page reports what is under the pointer; only the cat itself takes clicks.
    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let body = message.body as? [String: Any], let value = body["hit"] as? String else { return }
        hit = value
        if !dragging { panel.ignoresMouseEvents = clickThrough || value == "none" }
        refreshCursor()
    }

    func refreshCursor() {
        guard hit != "none", !clickThrough else { return }
        (dragging ? NSCursor.closedHand : NSCursor.openHand).set()
    }

    func pet(down: Bool, x: CGFloat) { js("cat.pet(\(down), \(Int(x)))") }

    // MARK: Input

    func installMouseMonitors() {
        let moves: NSEvent.EventTypeMask = [.mouseMoved, .leftMouseDragged, .rightMouseDragged, .otherMouseDragged]
        NSEvent.addGlobalMonitorForEvents(matching: moves) { [weak self] event in
            MainActor.assumeIsolated { self?.addDelta(event) }
        }
        NSEvent.addLocalMonitorForEvents(matching: [.mouseMoved]) { [weak self] event in
            MainActor.assumeIsolated { self?.addDelta(event) }
            return event
        }
        NSEvent.addGlobalMonitorForEvents(matching: [.scrollWheel]) { [weak self] event in
            MainActor.assumeIsolated {
                guard let self, self.panel.isVisible else { return }
                self.pendingDelta.y += -event.scrollingDeltaY * 0.4
            }
        }
        let buttons: NSEvent.EventTypeMask = [.leftMouseDown, .leftMouseUp, .rightMouseDown, .rightMouseUp, .otherMouseDown, .otherMouseUp]
        NSEvent.addGlobalMonitorForEvents(matching: buttons) { [weak self] event in
            MainActor.assumeIsolated {
                guard let self, self.panel.isVisible else { return }
                let down = [.leftMouseDown, .rightMouseDown, .otherMouseDown].contains(event.type)
                if event.type == .leftMouseUp { self.webView.cancelInteractions() }
                self.js("cat.mouseButton(\(down))")
            }
        }
    }

    func addDelta(_ event: NSEvent) {
        guard panel.isVisible else { return }
        pendingDelta.x += event.deltaX
        pendingDelta.y += event.deltaY
    }

    /// Only which key went down or up reaches the page; nothing typed is stored or logged.
    func handleKey(_ keyCode: Int, down: Bool) {
        keyEvents += 1
        guard panel.isVisible, let code = KeyMap.code(keyCode) else { return }
        js("cat.key('\(code)',\(down))")
    }

    /// A listen-only tap can be created without Input Monitoring and then never fire,
    /// so the permission itself decides whether the keyboard counts as working.
    func startKeyboardTap() {
        if CGPreflightListenEventAccess(), createTap() {
            keyboardAllowed = true
            return
        }
        keyboardAllowed = false
        CGRequestListenEventAccess()
        updateStatusTitle()
        // Give the system prompt a moment; if access is still missing, say what to do.
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { [weak self] in
            guard let self, !self.keyboardAllowed else { return }
            self.explainPermission()
        }
        tapRetry = Timer.scheduledTimer(withTimeInterval: 2, repeats: true) { [weak self] timer in
            MainActor.assumeIsolated {
                guard let self, CGPreflightListenEventAccess(), self.createTap() else { return }
                self.keyboardAllowed = true
                self.updateStatusTitle()
                timer.invalidate()
                if self.debugStatusEnabled { self.reportStatus() }
            }
        }
    }

    func explainPermission() {
        NSApp.activate(ignoringOtherApps: true)
        let alert = NSAlert()
        alert.messageText = "KeyCat 需要「输入监控」权限才能跟着你敲键盘"
        alert.informativeText = "请在「系统设置 › 隐私与安全性 › 输入监控」里打开 KeyCat。"
            + "如果列表里已有 KeyCat 但打开后仍没反应，先用「−」删掉它，再重新打开 KeyCat 授权。"
            + "KeyCat 只用按键位置驱动动画，不记录你输入的内容。"
        alert.addButton(withTitle: "打开系统设置")
        alert.addButton(withTitle: "稍后")
        if alert.runModal() == .alertFirstButtonReturn { openPrivacy() }
    }

    func updateStatusTitle() {
        statusItem?.button?.title = keyboardAllowed ? "🐈‍⬛" : "🐈‍⬛⚠️"
    }

    /// Troubleshooting line in ~/Library/Application Support/KeyCat/status.txt:
    /// keyboard permission and how many animation frames the page drew in the last second.
    func reportStatus() {
        js("cat.state.ticks = 0; 1")
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in
            guard let self else { return }
            self.webView.evaluateJavaScript("cat.state.ticks") { result, _ in
                let fps = (result as? NSNumber).map { "\($0.intValue)" } ?? "?"
                let access = CGPreflightListenEventAccess() ? "granted" : "missing"
                let line = "\(Date()) keyboard=\(self.keyboardAllowed ? "active" : "waiting for permission") "
                    + "inputMonitoring=\(access) keyEvents=\(self.keyEvents) fps=\(fps)\n"
                try? line.write(to: self.supportFile("status.txt"), atomically: true, encoding: .utf8)
            }
        }
    }

    func createTap() -> Bool {
        guard eventTap == nil else { return true }
        let mask = (1 << CGEventType.keyDown.rawValue) | (1 << CGEventType.keyUp.rawValue) | (1 << CGEventType.flagsChanged.rawValue)
        let refcon = Unmanaged.passUnretained(self).toOpaque()
        guard let tap = CGEvent.tapCreate(tap: .cgSessionEventTap, place: .headInsertEventTap, options: .listenOnly,
                                          eventsOfInterest: CGEventMask(mask), callback: keyTapCallback, userInfo: refcon)
        else { return false }
        let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        eventTap = tap
        return true
    }

    func handleFlags(_ keyCode: Int, _ flags: CGEventFlags) {
        if keyCode == 57 {
            handleKey(keyCode, down: true)
            return
        }
        let flag: CGEventFlags
        switch keyCode {
        case 56, 60: flag = .maskShift
        case 59, 62: flag = .maskControl
        case 58, 61: flag = .maskAlternate
        case 54, 55: flag = .maskCommand
        case 63: flag = .maskSecondaryFn
        default: return
        }
        // flagsChanged names the key that toggled; with its family flag cleared it was released.
        let isDown = flags.contains(flag) && !heldModifiers.contains(keyCode)
        if isDown { heldModifiers.insert(keyCode) } else { heldModifiers.remove(keyCode) }
        handleKey(keyCode, down: isDown)
    }
}

private let keyTapCallback: CGEventTapCallBack = { _, type, event, refcon in
    guard let refcon else { return Unmanaged.passUnretained(event) }
    let app = Unmanaged<AppDelegate>.fromOpaque(refcon).takeUnretainedValue()
    MainActor.assumeIsolated {
        let keyCode = Int(event.getIntegerValueField(.keyboardEventKeycode))
        switch type {
        case .tapDisabledByTimeout, .tapDisabledByUserInput:
            if let tap = app.eventTap { CGEvent.tapEnable(tap: tap, enable: true) }
        case .keyDown:
            if event.getIntegerValueField(.keyboardEventAutorepeat) == 0 { app.handleKey(keyCode, down: true) }
        case .keyUp:
            app.handleKey(keyCode, down: false)
        case .flagsChanged:
            app.handleFlags(keyCode, event.flags)
        default:
            break
        }
    }
    return Unmanaged.passUnretained(event)
}
