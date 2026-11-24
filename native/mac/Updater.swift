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

    let build = Int(Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "") ?? 0
    weak var app: AppDelegate?
    private(set) var state: State = .idle
    private var timer: Timer?

    init(app: AppDelegate) {
        self.app = app
    }

    func start() {
        DispatchQueue.main.asyncAfter(deadline: .now() + 15) { [weak self] in self?.check() }
        timer = Timer.scheduledTimer(withTimeInterval: 6 * 60 * 60, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.check() }
        }
    }

    func check() {
        guard state != .checking else { return }
        state = .checking
        Task {
            do {
                var req = URLRequest(url: Self.manifestURL)
                req.cachePolicy = .reloadIgnoringLocalCacheData
                let (data, resp) = try await URLSession.shared.data(for: req)
                guard (resp as? HTTPURLResponse)?.statusCode ?? 200 == 200 else {
                    self.state = .failed
                    return
                }
                let manifest = try JSONDecoder().decode(Manifest.self, from: data)
                if manifest.build > self.build {
                    self.app?.log("update \(manifest.version) (build \(manifest.build)) available")
                    self.state = .available(manifest.version)
                } else {
                    self.state = .current
                }
            } catch {
                self.app?.log("update check failed: \(error.localizedDescription)")
                self.state = .failed
            }
        }
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
