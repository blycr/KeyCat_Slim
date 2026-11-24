import Cocoa

/// Dynamic warm cream / dark charcoal palette matching KeyCat's visual identity.
private enum KeyCatTheme {
    private static func dynamic(light: UInt32, dark: UInt32) -> NSColor {
        NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
            let rgb = isDark ? dark : light
            return NSColor(
                srgbRed: CGFloat((rgb >> 16) & 0xff) / 255.0,
                green: CGFloat((rgb >> 8) & 0xff) / 255.0,
                blue: CGFloat(rgb & 0xff) / 255.0,
                alpha: 1.0
            )
        }
    }

    static let bg         = dynamic(light: 0xf7f2e4, dark: 0x23211d)
    static let footerBg   = dynamic(light: 0xf2ecdc, dark: 0x1f1d19)
    static let card       = dynamic(light: 0xfffdf7, dark: 0x2d2a25)
    static let btnHover   = dynamic(light: 0xf8f3e6, dark: 0x38342d)
    static let btnPressed = dynamic(light: 0xebe3cf, dark: 0x454037)
    static let text       = dynamic(light: 0x2a2722, dark: 0xeee8da)
    static let muted      = dynamic(light: 0x8a8274, dark: 0xa59d8d)
    static let line       = dynamic(light: 0xe6dfcd, dark: 0x3d3932)
    static let accent     = dynamic(light: 0xe08a3c, dark: 0xf0a35a)
    static let accentText = dynamic(light: 0xfffdf7, dark: 0x1d1a16)
}

/// Flipped view whose CALayer background/border colors dynamically update on Light/Dark switches.
private final class ThemedView: NSView {
    var fillColor: NSColor? { didSet { needsDisplay = true } }
    var borderColor: NSColor? { didSet { needsDisplay = true } }
    var cornerRadius: CGFloat = 0 { didSet { needsDisplay = true } }
    var borderWidth: CGFloat = 0 { didSet { needsDisplay = true } }

    override var isFlipped: Bool { true }
    override var wantsUpdateLayer: Bool { true }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) not used") }

    override func updateLayer() {
        guard let layer else { return }
        layer.cornerRadius = cornerRadius
        layer.borderWidth = borderWidth
        effectiveAppearance.performAsCurrentDrawingAppearance {
            layer.backgroundColor = fillColor?.cgColor
            layer.borderColor = borderColor?.cgColor
        }
    }
}

/// Custom warm-styled button with hover, active press, hand cursor, and temporary trigger feedback.
private final class WarmActionButton: NSView {
    let baseTitle: String
    let feedbackTitle: String
    let isSecondary: Bool
    var onClick: (() -> Void)?

    private let labelField = NSTextField(labelWithString: "")
    private var isHovered = false
    private var isPressed = false
    private var showingFeedback = false
    private var feedbackWork: DispatchWorkItem?
    private var trackingAreaRef: NSTrackingArea?

    override var wantsUpdateLayer: Bool { true }

    init(frame: CGRect, title: String, feedbackTitle: String, isSecondary: Bool = false) {
        self.baseTitle = title
        self.feedbackTitle = feedbackTitle
        self.isSecondary = isSecondary
        super.init(frame: frame)
        wantsLayer = true

        labelField.stringValue = title
        labelField.alignment = .center
        labelField.font = NSFont.systemFont(ofSize: 13, weight: .semibold)
        labelField.isEditable = false
        labelField.isSelectable = false
        labelField.isBezeled = false
        labelField.drawsBackground = false
        addSubview(labelField)
        layoutLabel()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) not used") }

    private func layoutLabel() {
        let h: CGFloat = 18
        labelField.frame = CGRect(x: 4, y: (bounds.height - h) * 0.5, width: bounds.width - 8, height: h)
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        layoutLabel()
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let old = trackingAreaRef { removeTrackingArea(old) }
        let area = NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .activeInKeyWindow, .cursorUpdate],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        trackingAreaRef = area
    }

    override func cursorUpdate(with event: NSEvent) {
        NSCursor.pointingHand.set()
    }

    override func mouseEntered(with event: NSEvent) {
        isHovered = true
        needsDisplay = true
    }

    override func mouseExited(with event: NSEvent) {
        isHovered = false
        isPressed = false
        needsDisplay = true
    }

    override func mouseDown(with event: NSEvent) {
        isPressed = true
        needsDisplay = true
    }

    override func mouseUp(with event: NSEvent) {
        let inside = bounds.contains(convert(event.locationInWindow, from: nil))
        isPressed = false
        if inside {
            triggerFeedback()
            onClick?()
        }
        needsDisplay = true
    }

    private func triggerFeedback() {
        feedbackWork?.cancel()
        showingFeedback = true
        labelField.stringValue = feedbackTitle
        needsDisplay = true
        let work = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.showingFeedback = false
                self.labelField.stringValue = self.baseTitle
                self.needsDisplay = true
            }
        }
        feedbackWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2, execute: work)
    }

    override func updateLayer() {
        guard let layer else { return }
        layer.cornerRadius = 8
        layer.borderWidth = 1
        effectiveAppearance.performAsCurrentDrawingAppearance {
            if showingFeedback {
                layer.backgroundColor = KeyCatTheme.accent.cgColor
                layer.borderColor = KeyCatTheme.accent.cgColor
                labelField.textColor = KeyCatTheme.accentText
            } else if isPressed {
                layer.backgroundColor = KeyCatTheme.btnPressed.cgColor
                layer.borderColor = KeyCatTheme.accent.cgColor
                labelField.textColor = isSecondary ? KeyCatTheme.muted : KeyCatTheme.text
            } else if isHovered {
                layer.backgroundColor = KeyCatTheme.btnHover.cgColor
                layer.borderColor = KeyCatTheme.accent.cgColor
                labelField.textColor = isSecondary ? KeyCatTheme.muted : KeyCatTheme.text
            } else {
                layer.backgroundColor = KeyCatTheme.card.cgColor
                layer.borderColor = KeyCatTheme.line.cgColor
                labelField.textColor = isSecondary ? KeyCatTheme.muted : KeyCatTheme.text
            }
        }
    }
}

/// Custom pill button for personality presets with hover, pressed, and active highlight states.
private final class PresetPillButton: NSView {
    let title: String
    var isSelected: Bool = false { didSet { needsDisplay = true } }
    var onClick: (() -> Void)?

    private let labelField = NSTextField(labelWithString: "")
    private var isHovered = false
    private var isPressed = false
    private var trackingAreaRef: NSTrackingArea?

    override var wantsUpdateLayer: Bool { true }

    init(frame: CGRect, title: String) {
        self.title = title
        super.init(frame: frame)
        wantsLayer = true

        labelField.stringValue = title
        labelField.alignment = .center
        labelField.font = NSFont.systemFont(ofSize: 12.5, weight: .semibold)
        labelField.isEditable = false
        labelField.isSelectable = false
        labelField.isBezeled = false
        labelField.drawsBackground = false
        addSubview(labelField)
        layoutLabel()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) not used") }

    private func layoutLabel() {
        let h: CGFloat = 17
        labelField.frame = CGRect(x: 2, y: (bounds.height - h) * 0.5, width: bounds.width - 4, height: h)
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        layoutLabel()
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let old = trackingAreaRef { removeTrackingArea(old) }
        let area = NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .activeInKeyWindow, .cursorUpdate],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        trackingAreaRef = area
    }

    override func cursorUpdate(with event: NSEvent) {
        NSCursor.pointingHand.set()
    }

    override func mouseEntered(with event: NSEvent) {
        isHovered = true
        needsDisplay = true
    }

    override func mouseExited(with event: NSEvent) {
        isHovered = false
        isPressed = false
        needsDisplay = true
    }

    override func mouseDown(with event: NSEvent) {
        isPressed = true
        needsDisplay = true
    }

    override func mouseUp(with event: NSEvent) {
        let inside = bounds.contains(convert(event.locationInWindow, from: nil))
        isPressed = false
        if inside {
            onClick?()
        }
        needsDisplay = true
    }

    override func updateLayer() {
        guard let layer else { return }
        layer.cornerRadius = 7
        layer.borderWidth = 1
        effectiveAppearance.performAsCurrentDrawingAppearance {
            if isSelected {
                layer.backgroundColor = KeyCatTheme.accent.cgColor
                layer.borderColor = KeyCatTheme.accent.cgColor
                labelField.textColor = KeyCatTheme.accentText
            } else if isPressed {
                layer.backgroundColor = KeyCatTheme.btnPressed.cgColor
                layer.borderColor = KeyCatTheme.accent.cgColor
                labelField.textColor = KeyCatTheme.text
            } else if isHovered {
                layer.backgroundColor = KeyCatTheme.btnHover.cgColor
                layer.borderColor = KeyCatTheme.accent.cgColor
                labelField.textColor = KeyCatTheme.text
            } else {
                layer.backgroundColor = KeyCatTheme.bg.cgColor
                layer.borderColor = KeyCatTheme.line.cgColor
                labelField.textColor = KeyCatTheme.text
            }
        }
    }
}

/// NSWindow subclass that cleanly closes when Escape is pressed.
private final class EscClosableWindow: NSWindow {
    override func cancelOperation(_ sender: Any?) {
        close()
    }
}

/// Pure AppKit "调节反应…" window driven directly by `keycat-core`'s TUNING_SCHEMA (zero WebKit).
/// Matches KeyCat's warm cream / dark charcoal + caramel accent palette, with sticky bottom
/// action bar, group toggle dimming, instant button feedback, and hidden scrollbars.
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

    private struct PresetItem {
        let name: String
        let hint: String
    }

    weak var app: AppDelegate?
    var window: NSWindow?

    private var items: [ParamItem] = []
    private var presets: [PresetItem] = []
    private var presetButtons: [PresetPillButton] = []
    private var presetHintLabel: NSTextField?
    private var toggles: [String: NSSwitch] = [:]
    private var sliders: [String: NSSlider] = [:]
    private var valueLabels: [String: NSTextField] = [:]
    private var sliderRowViews: [String: NSView] = [:]

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
        let pCount = Int(kc_preset_count())
        presets.reserveCapacity(pCount)
        for i in 0..<pCount {
            let name = kc_preset_name(UInt32(i)).map { String(cString: $0) } ?? ""
            let hint = kc_preset_hint(UInt32(i)).map { String(cString: $0) } ?? ""
            if !name.isEmpty {
                presets.append(PresetItem(name: name, hint: hint))
            }
        }
    }

    func show() {
        if window == nil { build() }
        syncControlsFromScene()
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
    }

    private func formatValue(_ item: ParamItem, value: Float) -> String {
        if item.unit == "hour" {
            let floored = value.rounded(.down)
            let whole = Int(floored)
            let h = ((whole % 24) + 24) % 24
            let m = Int(((value - floored) * 60.0).rounded())
            return String(format: "%02d:%02d", h, m)
        }
        let num = item.step < 1.0 ? String(format: "%.1f", value) : "\(Int(value.rounded()))"
        return item.unit.isEmpty ? num : "\(num) \(item.unit)"
    }

    private func build() {
        presetButtons.removeAll()
        presetHintLabel = nil
        toggles.removeAll()
        sliders.removeAll()
        valueLabels.removeAll()
        sliderRowViews.removeAll()

        let winW: CGFloat = 424
        let footerH: CGFloat = 58
        let maxH = min(780, (NSScreen.main?.visibleFrame.height ?? 900) - 80)

        let padX: CGFloat = 20
        let cardW = winW - padX * 2
        let innerPad: CGFloat = 14
        let innerW = cardW - innerPad * 2

        let doc = ThemedView(frame: CGRect(x: 0, y: 0, width: winW, height: 100))
        doc.fillColor = KeyCatTheme.bg

        var y: CGFloat = 14
        let subtitle = makeLabel("改动实时生效，可随时点下方按钮看小猫反应", size: 12, weight: .regular, color: KeyCatTheme.muted)
        subtitle.frame = CGRect(x: padX + 4, y: y, width: cardW - 8, height: 18)
        doc.addSubview(subtitle)
        y += 28

        // Personality presets card
        if !presets.isEmpty {
            let pHeader = makeLabel("性格预设", size: 12.5, weight: .semibold, color: KeyCatTheme.muted)
            pHeader.frame = CGRect(x: padX + 4, y: y, width: cardW - 8, height: 20)
            doc.addSubview(pHeader)
            y += 24

            let pCardTop = y
            let pCard = ThemedView(frame: CGRect(x: padX, y: pCardTop, width: cardW, height: 76))
            pCard.fillColor = KeyCatTheme.card
            pCard.borderColor = KeyCatTheme.line
            pCard.borderWidth = 1
            pCard.cornerRadius = 10

            let pGap: CGFloat = 6
            let pillCount = CGFloat(presets.count)
            let pillW = (innerW - pGap * (pillCount - 1)) / pillCount
            let pillH: CGFloat = 28
            for (idx, preset) in presets.enumerated() {
                let px = innerPad + CGFloat(idx) * (pillW + pGap)
                let pill = PresetPillButton(
                    frame: CGRect(x: px, y: 12, width: pillW, height: pillH),
                    title: preset.name
                )
                pill.onClick = { [weak self] in
                    self?.app?.applyPreset(idx)
                    self?.syncControlsFromScene()
                }
                pCard.addSubview(pill)
                presetButtons.append(pill)
            }

            let hint = makeLabel("", size: 11.5, weight: .regular, color: KeyCatTheme.muted)
            hint.frame = CGRect(x: innerPad, y: 48, width: innerW, height: 18)
            pCard.addSubview(hint)
            presetHintLabel = hint

            doc.addSubview(pCard)
            y = pCardTop + 76 + 16
        }

        var groups: [String] = []
        for it in items where !groups.contains(it.group) {
            groups.append(it.group)
        }

        for (gi, group) in groups.enumerated() {
            if gi > 0 { y += 16 }

            let header = makeLabel(group, size: 12.5, weight: .semibold, color: KeyCatTheme.muted)
            header.frame = CGRect(x: padX + 4, y: y, width: cardW - 8, height: 20)
            doc.addSubview(header)
            y += 24

            let cardTop = y
            let card = ThemedView(frame: CGRect(x: padX, y: cardTop, width: cardW, height: 10))
            card.fillColor = KeyCatTheme.card
            card.borderColor = KeyCatTheme.line
            card.borderWidth = 1
            card.cornerRadius = 10

            var cy: CGFloat = 6
            let groupItems = items.filter { $0.group == group }
            for (idx, it) in groupItems.enumerated() {
                if idx > 0 {
                    let sep = ThemedView(frame: CGRect(x: innerPad, y: cy, width: innerW, height: 1))
                    sep.fillColor = KeyCatTheme.line
                    card.addSubview(sep)
                    cy += 8
                }

                let rowTop = cy
                let rowView = ThemedView(frame: CGRect(x: 0, y: rowTop, width: cardW, height: 10))
                var ry: CGFloat = 2

                if it.isToggle {
                    let lbl = makeLabel(it.label, size: 13, weight: .medium, color: KeyCatTheme.text)
                    lbl.frame = CGRect(x: innerPad, y: ry + 2, width: innerW - 50, height: 20)
                    rowView.addSubview(lbl)

                    let sw = NSSwitch(frame: CGRect(x: cardW - innerPad - 40, y: ry, width: 40, height: 24))
                    sw.controlSize = .small
                    sw.identifier = NSUserInterfaceItemIdentifier(it.key)
                    sw.target = self
                    sw.action = #selector(toggleChanged(_:))
                    rowView.addSubview(sw)
                    toggles[it.key] = sw
                    ry += 26
                } else {
                    let lbl = makeLabel(it.label, size: 13, weight: .medium, color: KeyCatTheme.text)
                    lbl.frame = CGRect(x: innerPad, y: ry, width: innerW - 92, height: 18)
                    rowView.addSubview(lbl)

                    let val = makeLabel("", size: 12.5, weight: .semibold, color: KeyCatTheme.accent)
                    val.alignment = .right
                    val.frame = CGRect(x: cardW - innerPad - 92, y: ry, width: 92, height: 18)
                    rowView.addSubview(val)
                    valueLabels[it.key] = val
                    ry += 22

                    let sl = NSSlider(
                        value: Double(it.defVal),
                        minValue: Double(it.minVal),
                        maxValue: Double(it.maxVal),
                        target: self,
                        action: #selector(sliderChanged(_:))
                    )
                    sl.isContinuous = true
                    sl.trackFillColor = KeyCatTheme.accent
                    sl.identifier = NSUserInterfaceItemIdentifier(it.key)
                    sl.frame = CGRect(x: innerPad, y: ry, width: innerW, height: 20)
                    rowView.addSubview(sl)
                    sliders[it.key] = sl
                    sliderRowViews[it.key] = rowView
                    ry += 24
                }

                if !it.hint.isEmpty {
                    let hint = makeLabel(it.hint, size: 11.5, weight: .regular, color: KeyCatTheme.muted)
                    hint.maximumNumberOfLines = 2
                    hint.lineBreakMode = .byWordWrapping
                    let h = hint.sizeThatFits(NSSize(width: innerW, height: 44)).height
                    let hintH = max(16, ceil(h))
                    hint.frame = CGRect(x: innerPad, y: ry, width: innerW, height: hintH)
                    rowView.addSubview(hint)
                    ry += hintH + 6
                } else {
                    ry += 4
                }

                rowView.frame = CGRect(x: 0, y: rowTop, width: cardW, height: ry)
                card.addSubview(rowView)
                cy = rowTop + ry
            }
            cy += 4
            card.frame = CGRect(x: padX, y: cardTop, width: cardW, height: cy)
            doc.addSubview(card)
            y = cardTop + cy
        }

        y += 18
        doc.frame = CGRect(x: 0, y: 0, width: winW, height: y)

        let winH = min(maxH, y + footerH)
        let scrollH = winH - footerH

        let root = ThemedView(frame: CGRect(x: 0, y: 0, width: winW, height: winH))
        root.fillColor = KeyCatTheme.bg

        // Scrollable parameter list with hidden scrollbars
        let scroll = NSScrollView(frame: CGRect(x: 0, y: 0, width: winW, height: scrollH))
        scroll.hasVerticalScroller = false
        scroll.hasHorizontalScroller = false
        scroll.scrollerStyle = .overlay
        scroll.drawsBackground = false
        scroll.documentView = doc
        root.addSubview(scroll)

        // Sticky bottom action bar
        let footer = ThemedView(frame: CGRect(x: 0, y: scrollH, width: winW, height: footerH))
        footer.fillColor = KeyCatTheme.footerBg

        let footerLine = ThemedView(frame: CGRect(x: 0, y: 0, width: winW, height: 1))
        footerLine.fillColor = KeyCatTheme.line
        footer.addSubview(footerLine)

        let gap: CGFloat = 8
        let resetW: CGFloat = 88
        let testW = (cardW - resetW - gap * 2) * 0.5
        let btnH: CGFloat = 34
        let btnY = (footerH - btnH) * 0.5

        let btnAnnoy = WarmActionButton(
            frame: CGRect(x: padX, y: btnY, width: testW, height: btnH),
            title: "让它生气试试",
            feedbackTitle: "已触发"
        )
        btnAnnoy.onClick = { [weak self] in self?.app?.annoyNow() }
        footer.addSubview(btnAnnoy)

        let btnCurious = WarmActionButton(
            frame: CGRect(x: padX + testW + gap, y: btnY, width: testW, height: btnH),
            title: "让它好奇试试",
            feedbackTitle: "已触发"
        )
        btnCurious.onClick = { [weak self] in self?.app?.curiousNow(seconds: 3.0) }
        footer.addSubview(btnCurious)

        let btnReset = WarmActionButton(
            frame: CGRect(x: padX + cardW - resetW, y: btnY, width: resetW, height: btnH),
            title: "恢复默认",
            feedbackTitle: "已恢复",
            isSecondary: true
        )
        btnReset.onClick = { [weak self] in
            self?.app?.resetTuning()
            self?.syncControlsFromScene()
        }
        footer.addSubview(btnReset)

        root.addSubview(footer)

        let win = EscClosableWindow(
            contentRect: CGRect(x: 0, y: 0, width: winW, height: winH),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        win.title = "KeyCat 调节"
        win.backgroundColor = KeyCatTheme.bg
        win.contentView = root
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
        updateGroupEnabledStates()
        updatePresetStates()
    }

    private func updatePresetStates() {
        guard let app else { return }
        let matched = app.matchingPreset()
        for (idx, btn) in presetButtons.enumerated() {
            btn.isSelected = (idx == matched)
        }
        if matched >= 0 && matched < presets.count {
            presetHintLabel?.stringValue = presets[matched].hint
        } else {
            presetHintLabel?.stringValue = "已自定义参数，随时可点上方预设或恢复默认"
        }
    }

    private func updateGroupEnabledStates() {
        guard let app else { return }
        let annoyOn = app.tuningValue(for: "annoyOn") >= 0.5
        let curiousOn = app.tuningValue(for: "curiousOn") >= 0.5
        let nightOn = app.tuningValue(for: "nightOn") >= 0.5
        var groups: [String] = []
        for it in items where !groups.contains(it.group) {
            groups.append(it.group)
        }
        let group0 = groups.indices.contains(0) ? groups[0] : ""
        let group1 = groups.indices.contains(1) ? groups[1] : ""
        for it in items where !it.isToggle {
            let enabled: Bool
            if it.group == group0 {
                enabled = annoyOn
            } else if it.group == group1 {
                enabled = curiousOn
            } else if it.key == "nightAt" {
                enabled = nightOn
            } else {
                enabled = true
            }
            sliders[it.key]?.isEnabled = enabled
            sliderRowViews[it.key]?.alphaValue = enabled ? 1.0 : 0.42
        }
    }

    @objc private func toggleChanged(_ sender: NSSwitch) {
        guard let key = sender.identifier?.rawValue, let app else { return }
        let on = sender.state == .on
        app.setTuningValue(key: key, value: on ? 1.0 : 0.0, isToggle: true)
        updateGroupEnabledStates()
        updatePresetStates()
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
        updatePresetStates()
    }

    func windowWillClose(_ notification: Notification) {
        presetButtons.removeAll()
        presetHintLabel = nil
        toggles.removeAll()
        sliders.removeAll()
        valueLabels.removeAll()
        sliderRowViews.removeAll()
        window?.contentView = nil
        window?.delegate = nil
        window = nil
    }
}
