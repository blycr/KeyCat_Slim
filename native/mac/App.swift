import Cocoa
import Metal

/// Window size at scale 1, in logical points (matches CROP_W / CROP_H in keycat-core).
let designSize = CGSize(width: 360, height: 368)

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    enum FpsMode: Equatable {
        case auto
        case fixed(Int)
        case unlimited
    }

    var panel: NSPanel!
    var catView: MetalCatView!
    var renderer: MetalRenderer!
    var scene: OpaquePointer!
    var statusItem: NSStatusItem!

    private var frameTimer: DispatchSourceTimer?
    private var activeTimerHz: Int = 0
    private var lastFrameTime: CFTimeInterval = CACurrentMediaTime()
    private var cachedDisplayHz: Int = 60

    var tapRetry: Timer?
    var eventTap: CFMachPort?
    private var tapSource: CFRunLoopSource?
    private var mouseMonitors: [Any] = []
    var keyboardAllowed = true
    var hit: UInt32 = UInt32(KC_HIT_NONE)
    var dragging = false
    var pendingDelta = CGPoint.zero
    var lastCursor = CGPoint(x: -9999, y: -9999)
    var heldModifiers = Set<Int>()
    var heldKeys: [Int: String] = [:]
    private var lastKeyAudit: CFTimeInterval = 0
    var keyEvents = 0

    let defaults = UserDefaults.standard
    lazy var settings = SettingsWindow(app: self)
    lazy var updater = UpdateChecker(app: self)

    // MARK: Persisted settings (same keys as previous macOS builds for seamless upgrade)

    var scale: CGFloat {
        get { CGFloat(defaults.object(forKey: "catScale") as? Double ?? 0.9) }
        set { defaults.set(Double(newValue), forKey: "catScale") }
    }
    var coat: String {
        get { defaults.string(forKey: "coat") ?? "black" }
        set { defaults.set(newValue, forKey: "coat") }
    }
    var style: String {
        get { defaults.string(forKey: "style") ?? "2d" }
        set { defaults.set(newValue, forKey: "style") }
    }
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
    var tune: [String: Any] {
        get { defaults.dictionary(forKey: "tune") ?? [:] }
        set { defaults.set(newValue, forKey: "tune") }
    }
    var privacy: Bool {
        get { defaults.bool(forKey: "privacy") }
        set { defaults.set(newValue, forKey: "privacy") }
    }
    var clickThrough: Bool {
        get { defaults.bool(forKey: "clickThrough") }
        set {
            defaults.set(newValue, forKey: "clickThrough")
            if !dragging { panel.ignoresMouseEvents = newValue || hit == UInt32(KC_HIT_NONE) }
        }
    }
    var sleepAfter: Int {
        get { defaults.object(forKey: "sleepAfter") as? Int ?? 60 }
        set { defaults.set(newValue, forKey: "sleepAfter") }
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

    func displayMaxFPS() -> Int {
        let hz = NSScreen.screens.map(\.maximumFramesPerSecond).max() ?? 60
        return (15...1000).contains(hz) ? hz : 60
    }

    func detectedHighFPS() -> Int? {
        let maxHz = displayMaxFPS()
        cachedDisplayHz = maxHz
        return maxHz > 60 ? maxHz : nil
    }

    // MARK: Lifecycle

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        cachedDisplayHz = displayMaxFPS()

        guard let s = kc_scene_new(), let r = MetalRenderer(scene: s) else {
            log("failed to initialize Metal renderer or keycat-core scene")
            NSApp.terminate(nil)
            return
        }
        scene = s
        renderer = r

        panel = NSPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary, .ignoresCycle]
        panel.hidesOnDeactivate = false
        panel.isMovable = false
        panel.ignoresMouseEvents = true

        catView = MetalCatView(frame: .zero, device: r.device)
        catView.app = self
        panel.contentView = catView

        applyScale(scale, keepOrigin: false)
        pushOptions()
        kc_scene_step(scene, 1.0 / 60.0)
        renderer.render(scene: scene, layer: catView.metalLayer)
        panel.orderFrontRegardless()

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        updateStatusIcon()
        let menu = NSMenu()
        menu.delegate = self
        statusItem.menu = menu

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(screenParametersChanged),
            name: NSApplication.didChangeScreenParametersNotification,
            object: nil
        )

        installMouseMonitors()
        startKeyboardTap()
        rescheduleFrameLoop()
        updater.start()
        log("started KeyCat native Metal v\(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "?")")
    }

    func applicationWillTerminate(_ notification: Notification) {
        stopFrameLoop()
        tapRetry?.invalidate()
        tapRetry = nil
        NotificationCenter.default.removeObserver(self)
        for monitor in mouseMonitors {
            NSEvent.removeMonitor(monitor)
        }
        mouseMonitors.removeAll()
        if let tap = eventTap {
            CGEvent.tapEnable(tap: tap, enable: false)
            if let source = tapSource {
                CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .commonModes)
                tapSource = nil
            }
            CFMachPortInvalidate(tap)
            eventTap = nil
        }
        if let s = scene {
            kc_scene_free(s)
            scene = nil
        }
    }

    @objc private func screenParametersChanged() {
        cachedDisplayHz = displayMaxFPS()
        catView.syncDrawableSize()
        rescheduleFrameLoop()
    }

    // MARK: Status bar icon (monochrome template vector silhouette, adapts to light/dark menu bar)

    private func makeStatusIcon(warning: Bool) -> NSImage {
        let size = NSSize(width: 18, height: 18)
        let img = NSImage(size: size, flipped: false) { _ in
            NSColor.black.setFill()
            // Simple crisp cat head + ears silhouette
            let head = NSBezierPath(ovalIn: NSRect(x: 3.0, y: 2.2, width: 12.0, height: 10.2))
            head.fill()
            let leftEar = NSBezierPath()
            leftEar.move(to: NSPoint(x: 3.6, y: 9.5))
            leftEar.line(to: NSPoint(x: 4.0, y: 15.6))
            leftEar.line(to: NSPoint(x: 8.2, y: 11.4))
            leftEar.close()
            leftEar.fill()
            let rightEar = NSBezierPath()
            rightEar.move(to: NSPoint(x: 14.4, y: 9.5))
            rightEar.line(to: NSPoint(x: 14.0, y: 15.6))
            rightEar.line(to: NSPoint(x: 9.8, y: 11.4))
            rightEar.close()
            rightEar.fill()
            if warning {
                NSColor.clear.setFill()
                NSBezierPath(ovalIn: NSRect(x: 12.5, y: 1.5, width: 4.8, height: 4.8)).fill()
                NSColor.black.setFill()
                NSBezierPath(ovalIn: NSRect(x: 13.4, y: 2.4, width: 3.0, height: 3.0)).fill()
            }
            return true
        }
        img.isTemplate = true
        return img
    }

    func updateStatusIcon() {
        statusItem?.button?.image = makeStatusIcon(warning: !keyboardAllowed)
        statusItem?.button?.toolTip = keyboardAllowed ? "KeyCat" : "KeyCat（缺少输入监控权限）"
    }

    // MARK: Scene options & tuning

    func pushOptions() {
        guard let scene else { return }
        let style3d: UInt8 = style == "3d" ? 1 : 0
        coat.withCString { cCoat in
            pattern.withCString { cPat in
                eyeColor.withCString { cEye in
                    eyeShine.withCString { cShine in
                        collar.withCString { cCollar in
                            kc_scene_set_look(
                                scene, style3d, cCoat, cPat, cEye, cShine, cCollar,
                                blush ? 1 : 0, whiskers ? 1 : 0
                            )
                        }
                    }
                }
            }
        }
        kc_scene_set_options(scene, privacy ? 1 : 0, sleepAfter > 0 ? Float(sleepAfter) : 1e9)
        if let data = try? JSONSerialization.data(withJSONObject: tune),
           let json = String(data: data, encoding: .utf8) {
            json.withCString { kc_scene_set_tuning_json(scene, $0) }
        }
    }

    func tuningValue(for key: String) -> Float {
        guard let scene else { return 0 }
        return key.withCString { kc_scene_get_tuning_value(scene, $0) }
    }

    func setTuningValue(key: String, value: Float, isToggle: Bool) {
        guard let scene else { return }
        key.withCString { kc_scene_set_tuning_value(scene, $0, value) }
        var t = tune
        t[key] = isToggle ? (value >= 0.5) : Double(value)
        tune = t
    }

    func resetTuning() {
        guard let scene else { return }
        tune = [:]
        kc_scene_reset_tuning(scene)
    }

    func annoyNow() {
        guard let scene else { return }
        kc_scene_annoy_now(scene)
        rescheduleFrameLoop()
    }

    func curiousNow(seconds: Float) {
        guard let scene else { return }
        kc_scene_curious_now(scene, seconds)
        rescheduleFrameLoop()
    }

    // MARK: Visibility & Adaptive Frame Loop

    func setPanelVisible(_ visible: Bool) {
        if visible {
            panel.orderFrontRegardless()
            lastFrameTime = CACurrentMediaTime()
            rescheduleFrameLoop()
        } else {
            stopFrameLoop()
            catView.cancelInteractions()
            if let scene {
                for (_, code) in heldKeys {
                    code.withCString { kc_scene_on_key(scene, $0, 0) }
                }
            }
            heldKeys.removeAll()
            heldModifiers.removeAll()
            hit = UInt32(KC_HIT_NONE)
            dragging = false
            pendingDelta = .zero
            panel.ignoresMouseEvents = true
            panel.orderOut(nil)
        }
    }

    private func desiredFrameHz() -> Int {
        switch fps {
        case .auto:
            let cap = min(60, max(15, cachedDisplayHz))
            let busy = scene != nil && kc_scene_is_busy(scene) != 0
            return busy ? cap : min(cap, 30)
        case .fixed(let n):
            return min(max(n, 15), 240)
        case .unlimited:
            return min(max(cachedDisplayHz, 60), 240)
        }
    }

    func rescheduleFrameLoop() {
        guard panel != nil, panel.isVisible, scene != nil else {
            stopFrameLoop()
            return
        }
        let targetHz = desiredFrameHz()
        if frameTimer != nil && activeTimerHz == targetHz {
            return
        }
        activeTimerHz = targetHz
        let intervalNs = UInt64((1_000_000_000.0 / Double(targetHz)).rounded())
        if let timer = frameTimer {
            timer.schedule(deadline: .now() + .nanoseconds(Int(intervalNs)), repeating: .nanoseconds(Int(intervalNs)), leeway: .milliseconds(1))
        } else {
            let timer = DispatchSource.makeTimerSource(flags: [], queue: .main)
            timer.schedule(deadline: .now(), repeating: .nanoseconds(Int(intervalNs)), leeway: .milliseconds(1))
            timer.setEventHandler { [weak self] in
                MainActor.assumeIsolated { self?.frameTick() }
            }
            timer.resume()
            frameTimer = timer
        }
    }

    func stopFrameLoop() {
        frameTimer?.cancel()
        frameTimer = nil
        activeTimerHz = 0
    }

    private func frameTick() {
        guard panel.isVisible, let scene, let renderer else { return }

        // Forward pointer position & delta synchronously to keycat-core
        let screen = NSEvent.mouseLocation
        let inWindow = panel.convertPoint(fromScreen: screen)
        let page = CGPoint(x: inWindow.x.rounded(), y: (panel.frame.height - inWindow.y).rounded())
        if page != lastCursor {
            lastCursor = page
            let newHit = kc_scene_on_cursor(scene, Float(page.x), Float(page.y))
            if newHit != hit {
                hit = newHit
                if !dragging { panel.ignoresMouseEvents = clickThrough || hit == UInt32(KC_HIT_NONE) }
                refreshCursor()
            }
        }
        if abs(pendingDelta.x) > 0.01 || abs(pendingDelta.y) > 0.01 {
            kc_scene_on_mouse_move(scene, Float(pendingDelta.x), Float(pendingDelta.y))
            pendingDelta = .zero
        }

        let now = CACurrentMediaTime()
        reconcileHeldKeys(now: now, scene: scene)
        let dt = Float(min(0.1, max(0.0, now - lastFrameTime)))
        lastFrameTime = now

        kc_scene_step(scene, dt)
        let currentHit = kc_scene_hit(scene)
        if currentHit != hit {
            hit = currentHit
            if !dragging { panel.ignoresMouseEvents = clickThrough || hit == UInt32(KC_HIT_NONE) }
            refreshCursor()
        }
        renderer.render(scene: scene, layer: catView.metalLayer)

        // In Auto mode, smoothly adjust timer cadence when transitioning between active and idle/sleep.
        if fps == .auto {
            let nextHz = desiredFrameHz()
            if nextHz != activeTimerHz {
                rescheduleFrameLoop()
            }
        }
    }

    func refreshCursor() {
        guard hit != UInt32(KC_HIT_NONE), !clickThrough else { return }
        (dragging ? NSCursor.closedHand : NSCursor.openHand).set()
    }

    func pet(down: Bool, x: CGFloat) {
        guard let scene else { return }
        kc_scene_on_pet(scene, down ? 1 : 0, Float(x))
        if down { rescheduleFrameLoop() }
    }

    // MARK: Logging

    func supportFile(_ name: String) -> URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("KeyCat")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent(name)
    }

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

    /// Recovers from missed keyUp / flagsChanged events (e.g. lock screen or Secure Input password fields).
    private func reconcileHeldKeys(now: CFTimeInterval, scene: OpaquePointer) {
        guard !heldKeys.isEmpty, now - lastKeyAudit >= 0.2 else { return }
        lastKeyAudit = now
        var stale: [(Int, String)] = []
        for (keyCode, code) in heldKeys {
            if !CGEventSource.keyState(.combinedSessionState, key: CGKeyCode(keyCode)) {
                stale.append((keyCode, code))
            }
        }
        for (keyCode, code) in stale {
            heldKeys.removeValue(forKey: keyCode)
            heldModifiers.remove(keyCode)
            code.withCString { kc_scene_on_key(scene, $0, 0) }
        }
    }

    // MARK: Global Input

    func installMouseMonitors() {
        let moves: NSEvent.EventTypeMask = [.mouseMoved, .leftMouseDragged, .rightMouseDragged, .otherMouseDragged]
        if let m = NSEvent.addGlobalMonitorForEvents(matching: moves, handler: { [weak self] event in
            MainActor.assumeIsolated { self?.addDelta(event) }
        }) { mouseMonitors.append(m) }
        if let m = NSEvent.addLocalMonitorForEvents(matching: [.mouseMoved], handler: { [weak self] event in
            MainActor.assumeIsolated { self?.addDelta(event) }
            return event
        }) { mouseMonitors.append(m) }
        if let m = NSEvent.addGlobalMonitorForEvents(matching: [.scrollWheel], handler: { [weak self] event in
            MainActor.assumeIsolated {
                guard let self, self.panel.isVisible else { return }
                self.pendingDelta.y += -event.scrollingDeltaY * 0.4
            }
        }) { mouseMonitors.append(m) }
        let buttons: NSEvent.EventTypeMask = [.leftMouseDown, .leftMouseUp, .rightMouseDown, .rightMouseUp, .otherMouseDown, .otherMouseUp]
        if let m = NSEvent.addGlobalMonitorForEvents(matching: buttons, handler: { [weak self] event in
            MainActor.assumeIsolated {
                guard let self, self.panel.isVisible, let scene = self.scene else { return }
                let down = [.leftMouseDown, .rightMouseDown, .otherMouseDown].contains(event.type)
                if event.type == .leftMouseUp { self.catView.cancelInteractions() }
                kc_scene_on_mouse_button(scene, down ? 1 : 0)
                if down && self.fps == .auto { self.rescheduleFrameLoop() }
            }
        }) { mouseMonitors.append(m) }
    }

    func addDelta(_ event: NSEvent) {
        guard panel.isVisible else { return }
        pendingDelta.x += event.deltaX
        pendingDelta.y += event.deltaY
    }

    func handleKey(_ keyCode: Int, down: Bool) {
        keyEvents += 1
        guard panel.isVisible, let scene, let code = KeyMap.code(keyCode) else { return }
        if down {
            if keyCode != 57 { heldKeys[keyCode] = code }
        } else {
            heldKeys.removeValue(forKey: keyCode)
        }
        code.withCString { kc_scene_on_key(scene, $0, down ? 1 : 0) }
        if down && fps == .auto { rescheduleFrameLoop() }
    }

    func startKeyboardTap() {
        if CGPreflightListenEventAccess(), createTap() {
            keyboardAllowed = true
            return
        }
        keyboardAllowed = false
        CGRequestListenEventAccess()
        updateStatusIcon()
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { [weak self] in
            guard let self, !self.keyboardAllowed else { return }
            self.explainPermission()
        }
        tapRetry = Timer.scheduledTimer(withTimeInterval: 2, repeats: true) { [weak self] timer in
            MainActor.assumeIsolated {
                guard let self, CGPreflightListenEventAccess(), self.createTap() else { return }
                self.keyboardAllowed = true
                self.updateStatusIcon()
                timer.invalidate()
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

    func createTap() -> Bool {
        guard eventTap == nil else { return true }
        let mask = (1 << CGEventType.keyDown.rawValue) | (1 << CGEventType.keyUp.rawValue) | (1 << CGEventType.flagsChanged.rawValue)
        let refcon = Unmanaged.passUnretained(self).toOpaque()
        guard let tap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .listenOnly,
            eventsOfInterest: CGEventMask(mask),
            callback: keyTapCallback,
            userInfo: refcon
        ) else { return false }
        let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        tapSource = source
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
