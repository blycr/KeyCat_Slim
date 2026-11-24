# KeyCat Slim

English | [简体中文](README.md)

[![Latest Release](https://img.shields.io/github/v/release/blycr/KeyCat_Slim?label=Release)](https://github.com/blycr/KeyCat_Slim/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2B%20%7C%20macOS%2013%2B-lightgrey)](https://github.com/blycr/KeyCat_Slim/releases/latest)

A lightweight native desktop pet cat that sits on a tiny keyboard: it taps keys as you type, follows your cursor, closes its eyes when petted, falls asleep when idle, and slams the desk with its tail when annoyed.

KeyCat Slim is a native rewrite based on [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat). **Both Windows and macOS editions have completely removed WebView2 and WKWebView**, sharing a single Rust 3D/physics/behavior engine for full visual fidelity, high-refresh smoothness, and minimal footprint.

![KeyCat Demo](assets/media/keycat.gif)

## Highlights

- **Zero WebView on Both Platforms**
  - Windows: Rust + Direct3D 11 (4x MSAA) + DirectComposition + Direct2D. Per-user NSIS installer is ~390 KB with no WebView2 dependency.
  - macOS: Swift + Metal (4x MSAA) + AppKit universal binary (`arm64` + `x86_64`), ~1.6 MB zipped, zero WebKit linkage.
- **Shared Cross-Platform Rust Core**
  - Procedural 3D geometry, 11 coat styles, spring physics, and behavior state machine live in `crates/core`, ensuring consistent visuals and reactions across OSes.
- **Adaptive & Locked Frame Rates**
  - Default **Auto** mode renders up to 60 fps (or display refresh rate if lower) while active, and throttles down automatically while idle or asleep.
  - Manual presets include `30 fps`, `60 fps`, native high-refresh display rate (`120`/`144`/`165`/`240 fps`), or **Unlimited**.
- **Rich Customization**
  - 11 coats, 2D/3D shading mode, eye colors & highlights, collars, blush, whiskers, window scaling, sleep timeout, and a dedicated native tuning panel for curiosity/annoyance thresholds.
- **Privacy First & Reliable Input**
  - Never records typed text or uploads telemetry. Only reacts to key up/down events and cursor position, with automatic stuck-key self-healing after lock screen, UAC prompts, or macOS Secure Input fields.

## Installation

### Windows (Windows 10 / 11, x64)

Download `KeyCat-Windows-Setup.exe` from [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) and run it.

- Installs per-user under `%LOCALAPPDATA%\KeyCat` without requiring administrator rights and registers cleanly in Windows Apps & Features.
- Because this open-source project is not commercially code-signed, Windows SmartScreen may show a warning; choose **More info -> Run anyway**.

### macOS (macOS 13 Ventura+, Apple Silicon & Intel)

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

**Manual download**: Grab `KeyCat-macOS.zip` from [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) and drag `KeyCat.app` into `/Applications`.

- On first launch, grant Input Monitoring in **System Settings -> Privacy & Security -> Input Monitoring**, then restart KeyCat.
- If macOS reports the app as damaged after a browser download, clear the quarantine attribute:
  ```sh
  xattr -dr com.apple.quarantine /Applications/KeyCat.app
  ```

## Usage

KeyCat lives in the system tray (Windows) or menu bar (macOS). Right-click the cat or click the tray/menu bar icon to configure:

- **Drag & Click-Through**: Drag the cat anywhere on screen. Enable **Click-Through** for streaming or full-screen work so clicks pass straight through to underlying windows.
- **Appearance & Tuning**: Switch coats, 2D/3D mode, size, and FPS; open the **Tune Reactions** window to adjust thresholds or trigger test animations immediately (`Esc` closes the window).
- **Privacy & Hide**: Enable **Privacy Mode** to suppress floating keycap labels; **Hide** suspends rendering completely.

### Windows Gaming & Elevation Notes

- Games running as Administrator block input hooks from non-elevated processes. Enable **Run as Administrator** in the tray menu to elevate via UAC.
- When combined with **Launch at Login**, KeyCat uses Windows Task Scheduler so subsequent logins and launches do not trigger repeated UAC prompts.
- Exclusive fullscreen games cover all desktop overlays; switch games to borderless windowed mode to keep KeyCat visible.

## Repository Layout

```text
.
├── Cargo.toml                 # Root Rust workspace (shared version & release profile)
├── crates/
│   ├── core/                  # Cross-platform 3D geometry, coats, springs, and behavior FSM
│   └── ffi/                   # C-ABI static library bridging crates/core to macOS Swift
├── apps/
│   ├── windows/               # Native Windows app (Direct3D 11 + DirectComposition + Direct2D + NSIS)
│   └── macos/                 # Native macOS app (Swift + Metal + AppKit + packaging scripts)
├── assets/
│   ├── icons/                 # Application and tray icons
│   └── media/                 # Documentation preview media
└── .github/
    ├── workflows/             # Tag-driven release workflows for Windows and macOS
    └── ISSUE_TEMPLATE/        # Issue and PR templates
```

## Building from Source

```sh
# 1. Run core and FFI unit tests
cargo test -p keycat-core -p keycat-ffi

# 2. Build Windows native binary (on Windows)
cargo build --release -p keycat-native
# Cross-compile from Linux/WSL using cargo-xwin:
# cargo xwin build --release --target x86_64-pc-windows-msvc -p keycat-native

# 3. Build and package macOS universal Metal app (on macOS with Rust + Xcode CLT)
sh apps/macos/build.sh         # Produces build/KeyCat.app
sh apps/macos/package.sh       # Produces dist/KeyCat-macOS.zip + .sha256 + latest-mac.json
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and release procedures.

## Documentation

- Changelog: [CHANGELOG.md](CHANGELOG.md)
- Contributing & Release Guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Security & Privacy: [SECURITY.md](SECURITY.md)
- Upstream Attribution & Licenses: [NOTICE](NOTICE)

## License & Attribution

Licensed under the [MIT License](LICENSE). KeyCat Slim builds upon the original concept and early implementation of [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat). See [NOTICE](NOTICE) for attribution and third-party dependency details.
