import Cocoa

/// Lightweight update checker for Homebrew-managed macOS installs.
/// Checks `latest-mac.json` on GitHub Releases and guides the user to upgrade via Homebrew
/// (or opens the Release page), never overwriting a Homebrew-managed bundle behind brew's back.
@MainActor
final class UpdateChecker {
    enum State: Equatable {
        case idle
        case checking
        case current
        case available(String)
        case failed
    }

    struct Manifest: Decodable {
        let version: String
        let build: Int
        let url: URL
    }

    static let manifestURL = URL(string: "https://github.com/blycr/KeyCat_Slim/releases/latest/download/latest-mac.json")!
    static let releasesURL = URL(string: "https://github.com/blycr/KeyCat_Slim/releases/latest")!
    static let brewUpgradeCmd = "brew upgrade --cask keycat"
    private static let retryDelays: [TimeInterval] = [
        30 * 60,
        60 * 60,
        2 * 60 * 60,
        4 * 60 * 60,
        6 * 60 * 60,
    ]

    let build = Int(Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "") ?? 0
    weak var app: AppDelegate?
    private(set) var state: State = .idle
    private var timer: Timer?
    private var inFlight = false
    private var retryStep = 0

    init(app: AppDelegate) {
        self.app = app
    }

    func start() {
        scheduleNext(after: 15)
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    private func scheduleNext(after interval: TimeInterval) {
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: interval, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated { self?.check(manual: false) }
        }
    }

    func check(manual: Bool = true) {
        guard !inFlight else { return }
        inFlight = true
        if manual {
            state = .checking
        }
        Task {
            defer { self.inFlight = false }
            do {
                var req = URLRequest(url: Self.manifestURL)
                req.cachePolicy = .reloadIgnoringLocalCacheData
                req.timeoutInterval = 10
                let (data, resp) = try await URLSession.shared.data(for: req)
                guard (resp as? HTTPURLResponse)?.statusCode ?? 200 == 200 else {
                    self.handleFailure(manual: manual, message: "unexpected HTTP status")
                    return
                }
                let manifest = try JSONDecoder().decode(Manifest.self, from: data)
                self.retryStep = 0
                if manifest.build > self.build {
                    self.app?.log("update \(manifest.version) (build \(manifest.build)) available")
                    self.state = .available(manifest.version)
                } else {
                    self.state = .current
                }
                self.scheduleNext(after: 6 * 60 * 60)
            } catch {
                self.handleFailure(manual: manual, message: error.localizedDescription)
            }
        }
    }

    private func handleFailure(manual: Bool, message: String) {
        app?.log("update check failed: \(message)")
        if manual {
            state = .failed
        } else if state == .checking {
            state = .idle
        }
        let delay = Self.retryDelays[min(retryStep, Self.retryDelays.count - 1)]
        retryStep = min(retryStep + 1, Self.retryDelays.count - 1)
        scheduleNext(after: delay)
    }

    func promptUpgrade(version: String) {
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(Self.brewUpgradeCmd, forType: .string)

        NSApp.activate(ignoringOtherApps: true)
        let alert = NSAlert()
        alert.messageText = "发现新版本 v\(version)"
        alert.informativeText = "如果通过 Homebrew 安装，可以直接在终端运行（命令已复制到剪贴板）：\n\n\(Self.brewUpgradeCmd)\n\n也可以直接打开发布页手动下载。"
        alert.addButton(withTitle: "打开发布页")
        alert.addButton(withTitle: "知道了")
        if alert.runModal() == .alertFirstButtonReturn {
            NSWorkspace.shared.open(Self.releasesURL)
        }
    }
}
