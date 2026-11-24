import Cocoa

/// Flipped NSView so top-down vertical layout is clean and predictable inside NSScrollView.
private final class FlippedView: NSView {
    override var isFlipped: Bool { true }
}

/// Pure AppKit "调节反应…" window driven directly by `keycat-core`'s TUNING_SCHEMA (zero WebKit).
@MainActor
final class SettingsWindow: NSObject, NSWindowDelegate {
    private struct ParamItem {
        let group: String
        let key: String
        let label: String
        let hint: String
        let unit: String
        let isToggle: Bool
        let minVal: Float
        let maxVal: Float
        let step: Float
        let defVal: Float
    }

    weak var app: AppDelegate?
    var window: NSWindow?

    private var items: [ParamItem] = []
    private var toggles: [String: NSSwitch] = [:]
    private var sliders: [String: NSSlider] = [:]
    private var valueLabels: [String: NSTextField] = [:]

    init(app: AppDelegate) {
        self.app = app
        super.init()
        loadSchema()
    }

    private func loadSchema() {
        let count = Int(kc_tuning_count())
        items.reserveCapacity(count)
        for i in 0..<count {
            let m = kc_tuning_meta(UInt32(i))
            guard let key = m.key.map({ String(cString: $0) }), !key.isEmpty else { continue }
            items.append(ParamItem(
                group: m.group.map { String(cString: $0) } ?? "",
                key: key,
                label: m.label.map { String(cString: $0) } ?? key,
                hint: m.hint.map { String(cString: $0) } ?? "",
                unit: m.unit.map { String(cString: $0) } ?? "",
                isToggle: m.is_toggle != 0,
                minVal: m.min_val,
                maxVal: m.max_val,
                step: m.step,
                defVal: m.def_val
            ))
        }
    }

    func show() {
        if window == nil { build() }
        syncControlsFromScene()
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
    }

    private func formatValue(_ item: ParamItem, value: Float) -> String {
        let num = item.step < 1.0 ? String(format: "%.1f", value) : "\(Int(value.rounded()))"
        return item.unit.isEmpty ? num : "\(num) \(item.unit)"
    }

    private func build() {
        toggles.removeAll()
        sliders.removeAll()
        valueLabels.removeAll()

        let winW: CGFloat = 420
        let maxH = min(820, (NSScreen.main?.visibleFrame.height ?? 900) - 80)

        let docW: CGFloat = winW - 16
        let padX: CGFloat = 18
        let cardW = docW - padX * 2
        let innerPad: CGFloat = 14
        let innerW = cardW - innerPad * 2

        let doc = FlippedView(frame: CGRect(x: 0, y: 0, width: docW, height: 100))
        var y: CGFloat = 16

        var groups: [String] = []
        for it in items where !groups.contains(it.group) {
            groups.append(it.group)
        }

        for (gi, group) in groups.enumerated() {
            if gi > 0 { y += 14 }

            let header = makeLabel(group, size: 12.5, weight: .semibold, color: .secondaryLabelColor)
            header.frame = CGRect(x: padX + 4, y: y, width: cardW - 8, height: 20)
            doc.addSubview(header)
            y += 24

            let cardTop = y
            let card = FlippedView(frame: CGRect(x: padX, y: cardTop, width: cardW, height: 10))
            card.wantsLayer = true
            card.layer?.cornerRadius = 10
            card.layer?.backgroundColor = NSColor.controlBackgroundColor.cgColor
            card.layer?.borderWidth = 1
            card.layer?.borderColor = NSColor.separatorColor.cgColor

            var cy: CGFloat = 8
            let groupItems = items.filter { $0.group == group }
            for (idx, it) in groupItems.enumerated() {
                if idx > 0 {
                    let sep = NSBox(frame: CGRect(x: innerPad, y: cy, width: innerW, height: 1))
                    sep.boxType = .separator
                    card.addSubview(sep)
                    cy += 8
                }

                if it.isToggle {
                    let lbl = makeLabel(it.label, size: 13, weight: .medium, color: .labelColor)
                    lbl.frame = CGRect(x: innerPad, y: cy + 2, width: innerW - 50, height: 20)
                    card.addSubview(lbl)

                    let sw = NSSwitch(frame: CGRect(x: cardW - innerPad - 40, y: cy, width: 40, height: 24))
                    sw.controlSize = .small
                    sw.identifier = NSUserInterfaceItemIdentifier(it.key)
                    sw.target = self
                    sw.action = #selector(toggleChanged(_:))
                    card.addSubview(sw)
                    toggles[it.key] = sw
                    cy += 26
                } else {
                    let lbl = makeLabel(it.label, size: 13, weight: .medium, color: .labelColor)
                    lbl.frame = CGRect(x: innerPad, y: cy, width: innerW - 90, height: 18)
                    card.addSubview(lbl)

                    let val = makeLabel("", size: 12.5, weight: .semibold, color: .controlAccentColor)
                    val.alignment = .right
                    val.frame = CGRect(x: cardW - innerPad - 90, y: cy, width: 90, height: 18)
                    card.addSubview(val)
                    valueLabels[it.key] = val
                    cy += 22

                    let sl = NSSlider(
                        value: Double(it.defVal),
                        minValue: Double(it.minVal),
                        maxValue: Double(it.maxVal),
                        target: self,
                        action: #selector(sliderChanged(_:))
                    )
                    sl.isContinuous = true
                    sl.identifier = NSUserInterfaceItemIdentifier(it.key)
                    sl.frame = CGRect(x: innerPad, y: cy, width: innerW, height: 20)
                    card.addSubview(sl)
                    sliders[it.key] = sl
                    cy += 24
                }

                if !it.hint.isEmpty {
                    let hint = makeLabel(it.hint, size: 11, weight: .regular, color: .secondaryLabelColor)
                    hint.maximumNumberOfLines = 2
                    hint.lineBreakMode = .byWordWrapping
                    let h = hint.sizeThatFits(NSSize(width: innerW, height: 40)).height
                    hint.frame = CGRect(x: innerPad, y: cy, width: innerW, height: max(15, ceil(h)))
                    card.addSubview(hint)
                    cy += max(15, ceil(h)) + 6
                } else {
                    cy += 4
                }
            }
            cy += 4
            card.frame = CGRect(x: padX, y: cardTop, width: cardW, height: cy)
            doc.addSubview(card)
            y = cardTop + cy
        }

        // Action buttons row
        y += 16
        let gap: CGFloat = 8
        let resetW: CGFloat = 88
        let testW = (cardW - resetW - gap * 2) * 0.5
        let btnH: CGFloat = 30

        let btnAnnoy = NSButton(title: "试试惹烦它", target: self, action: #selector(testAnnoy))
        btnAnnoy.bezelStyle = .rounded
        btnAnnoy.frame = CGRect(x: padX, y: y, width: testW, height: btnH)
        doc.addSubview(btnAnnoy)

        let btnCurious = NSButton(title: "试试逗它好奇", target: self, action: #selector(testCurious))
        btnCurious.bezelStyle = .rounded
        btnCurious.frame = CGRect(x: padX + testW + gap, y: y, width: testW, height: btnH)
        doc.addSubview(btnCurious)

        let btnReset = NSButton(title: "恢复默认", target: self, action: #selector(resetAll))
        btnReset.bezelStyle = .rounded
        btnReset.frame = CGRect(x: padX + cardW - resetW, y: y, width: resetW, height: btnH)
        doc.addSubview(btnReset)

        y += btnH + 20
        doc.frame = CGRect(x: 0, y: 0, width: docW, height: y)

        let winH = min(maxH, y)
        let scroll = NSScrollView(frame: CGRect(x: 0, y: 0, width: winW, height: winH))
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.documentView = doc

        let win = NSWindow(
            contentRect: CGRect(x: 0, y: 0, width: winW, height: winH),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        win.title = "KeyCat 调节"
        win.contentView = scroll
        win.delegate = self
        win.isReleasedWhenClosed = false
        win.center()
        self.window = win
    }

    private func makeLabel(_ text: String, size: CGFloat, weight: NSFont.Weight, color: NSColor) -> NSTextField {
        let tf = NSTextField(labelWithString: text)
        tf.font = NSFont.systemFont(ofSize: size, weight: weight)
        tf.textColor = color
        return tf
    }

    private func syncControlsFromScene() {
        guard let app else { return }
        for it in items {
            let v = app.tuningValue(for: it.key)
            if it.isToggle {
                toggles[it.key]?.state = v >= 0.5 ? .on : .off
            } else {
                sliders[it.key]?.doubleValue = Double(v)
                valueLabels[it.key]?.stringValue = formatValue(it, value: v)
            }
        }
    }

    @objc private func toggleChanged(_ sender: NSSwitch) {
        guard let key = sender.identifier?.rawValue, let app else { return }
        let on = sender.state == .on
        app.setTuningValue(key: key, value: on ? 1.0 : 0.0, isToggle: true)
    }

    @objc private func sliderChanged(_ sender: NSSlider) {
        guard let key = sender.identifier?.rawValue,
              let item = items.first(where: { $0.key == key }),
              let app else { return }
        let raw = Float(sender.doubleValue)
        let snapped = (raw / item.step).rounded() * item.step
        let clamped = min(max(snapped, item.minVal), item.maxVal)
        sender.doubleValue = Double(clamped)
        valueLabels[key]?.stringValue = formatValue(item, value: clamped)
        app.setTuningValue(key: key, value: clamped, isToggle: false)
    }

    @objc private func testAnnoy() {
        app?.annoyNow()
    }

    @objc private func testCurious() {
        app?.curiousNow(seconds: 3.0)
    }

    @objc private func resetAll() {
        app?.resetTuning()
        syncControlsFromScene()
    }

    func windowWillClose(_ notification: Notification) {
        toggles.removeAll()
        sliders.removeAll()
        valueLabels.removeAll()
        window?.contentView = nil
        window?.delegate = nil
        window = nil
    }
}
