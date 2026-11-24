# KeyCat Slim

English | [简体中文](README.md)

[![Latest Release](https://img.shields.io/github/v/release/blycr/KeyCat_Slim?label=Release)](https://github.com/blycr/KeyCat_Slim/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2B%20%7C%20macOS%2013%2B-lightgrey)](https://github.com/blycr/KeyCat_Slim/releases/latest)

A lightweight native desktop pet cat that sits on a tiny keyboard: it taps keys as you type, follows your cursor, closes its eyes when petted, falls asleep when idle, and slams the desk with its tail when annoyed.

KeyCat Slim is a native rewrite based on [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat). Both Windows and macOS editions have completely removed WebView2 and WKWebView, sharing a single cross-platform Rust 3D engine for full visual fidelity, high-refresh smoothness, and minimal resource usage.

![KeyCat Demo](assets/media/keycat.gif)

## Features

- **Zero WebView on Both Platforms**
  - Windows: Rust + Direct3D 11 (4x MSAA) + DirectComposition + Direct2D. Per-user NSIS installer is ~390 KB with no WebView2 dependency.
  - macOS: Swift + Metal (4x MSAA) + AppKit universal binary (`arm64` + `x86_64`), ~1.6 MB zipped, zero WebKit linkage.
- **Shared Cross-Platform Rust Core**
  - Procedural 3D geometry, 11 coats, spring physics, and the behavior state machine live in a single Rust core crate for consistent visuals and reactions across platforms.
- **Adaptive and Locked Frame Rates**
  - Default **Auto** mode renders up to 60 fps (or display refresh rate if lower) while active and scales down automatically when idle or asleep.
  - Manual presets include `30 fps`, `60 fps`, native display refresh rate (`120`/`144`/`165`/`240 fps`), and **Unlimited**.
- **Rich Appearance and Behavior Tuning**
  - 11 coats, 2D/3D shading, eye colors and highlights, collars, blush, whiskers, window scaling, sleep timeout, and a dedicated native tuning window.
- **Privacy First and Reliable Input**
  - Never logs typed text or sends telemetry. Reacts only to key up/down state and cursor coordinates, with automatic stuck-key self-healing after lock screen, UAC prompts, or macOS Secure Input fields.

## Installation

### Windows (Windows 10 / 11, x64)

Download `KeyCat-Windows-Setup.exe` from [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) and run the installer.

- Installs per-user under `%LOCALAPPDATA%\KeyCat` without administrator rights, integrates with Windows Apps & Features, and preserves existing settings on upgrade.
- Because this open-source project is not commercially code-signed, Windows SmartScreen may show a warning; choose **More info -> Run anyway**.

### macOS (macOS 13 Ventura+, Apple Silicon and Intel)

**Recommended via Homebrew Cask**:

```sh
brew install --cask --no-quarantine blycr/tap/keycat
```

Upgrade or uninstall:

```sh
brew upgrade --cask keycat
brew uninstall --cask keycat          # Uninstall app
brew uninstall --cask --zap keycat    # Uninstall and remove preferences
```

**Manual download**: Download `KeyCat-macOS.zip` from [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) and drag `KeyCat.app` into `/Applications`.

- On first launch, grant Input Monitoring in **System Settings -> Privacy & Security -> Input Monitoring**, then restart KeyCat.
- If macOS reports the app as damaged after a manual download, clear the quarantine attribute:
  ```sh
  xattr -dr com.apple.quarantine /Applications/KeyCat.app
  ```

## Usage

KeyCat resides in the system tray (Windows) or menu bar (macOS). Right-click the cat or click the tray/menu bar icon to open the menu:

- **Drag and Click-Through**: Drag the cat anywhere on screen. Enable **Click-Through** while streaming or working full-screen so clicks pass directly to underlying windows.
- **Appearance and Tuning**: Switch coats, 2D/3D mode, scale, and FPS from the menu; open **Tune Reactions** to adjust sensitivity and preview animations immediately (`Esc` closes the window).
- **Privacy and Hide**: Enable **Privacy Mode** to hide floating keycap labels; **Hide** suspends rendering completely.

### Windows Gaming and Elevation Notes

- Games running as Administrator block input hooks from non-elevated processes. Enable **Run as Administrator** in the tray menu to elevate via UAC.
- When combined with **Launch at Login**, KeyCat uses Windows Task Scheduler so subsequent logins and launches do not trigger repeated UAC prompts.
- Exclusive fullscreen games cover desktop overlays; switch games to borderless windowed mode to keep KeyCat visible.

## Building from Source

Requires stable Rust (1.80+); macOS builds also require Xcode Command Line Tools and the `aarch64-apple-darwin` and `x86_64-apple-darwin` targets.

```sh
# Run cross-platform core and FFI unit tests
cargo test -p keycat-core -p keycat-ffi

# Build Windows native binary (on Windows)
cargo build --release -p keycat-native

# Build and package macOS universal Metal app (on macOS)
sh apps/macos/build.sh
sh apps/macos/package.sh
```

## Project Structure

```text
.
├── Cargo.toml         # Root Rust workspace
├── crates/
│   ├── core/          # Cross-platform 3D geometry, coats, spring physics, and behavior FSM
│   └── ffi/           # C-ABI static library bridging crates/core to macOS Swift
├── apps/
│   ├── windows/       # Native Windows app (Direct3D 11 + DirectComposition + Direct2D)
│   └── macos/         # Native macOS app (Swift + Metal + AppKit)
└── assets/
    ├── icons/         # Application and tray icons
    └── media/         # Preview media
```

## Documentation

- [Changelog](CHANGELOG.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security & Privacy](SECURITY.md)
- [Attribution & Third-Party Licenses](NOTICE)

## License

Licensed under the [MIT License](LICENSE). KeyCat Slim builds upon [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) with a native cross-platform rewrite. See [NOTICE](NOTICE) for attribution and third-party dependency details.
