# 更新日志 (Changelog)

本文件记录 KeyCat Slim 的重要版本变更，格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。

## [Unreleased]

### Changed
- 将仓库重构为根级 Monorepo 结构：`crates/{core,ffi}`、`apps/{windows,macos}`、`assets/{icons,media}`，并通过根级 `Cargo.toml` 的 `[workspace.package]` 统一版本号管理。
- 移除不再维护的旧版 1.x WebView2 / WKWebView 目录（`win/`、`mac/`、`web/`、`site/`、`tools/`）及旧工作流（完整历史保留在 Git 标签 `v1.1.0` 至 `v2.1.1`）。
- Windows 托盘版本号改为编译期直接读取 `CARGO_PKG_VERSION`，避免版本不同步。

### Added
- 补全 `LICENSE`（MIT）、`NOTICE`、中英文 `README`、`CHANGELOG.md`、`CONTRIBUTING.md`、`SECURITY.md`。
- 新增 `.editorconfig`、`.gitattributes` 以及 GitHub Issue / PR 模板。

## [2.1.1] - 2026-11-24

### Added
- Windows 与 macOS 双端新增卡键自愈机制：在锁屏、UAC 弹窗或 macOS 密码输入框（Secure Input）后自动纠正未释放按键，避免猫咪爪子一直按住不放。
- Windows 端引入原子合并唤醒机制（减少高键速下重复唤醒渲染线程），并补齐双端退出时的钩子、监听器与定时器清理。
- Windows 调节窗口补齐深色标题栏、应用图标、手形指针、多行说明自动折行与键盘滚动。
- macOS 调节窗口统一为 KeyCat 暖米白 / 暖深灰 + 焦糖橙动态配色，跟随系统浅深色实时切换。

### Changed
- 全面重做双端「KeyCat 调节」窗口：底部操作栏改为固定吸底，补充悬停/按下态与 1.2 秒焦糖色“已触发 / 已恢复”反馈；统一双端按钮文案；关闭某项功能开关时同步禁用对应滑块组；支持按 `Esc` 关闭窗口。
- 全局隐藏调节窗口滚动条，界面更简洁。
- 移除上游遗留的 GitHub Pages 工作流，并收紧双端发布工作流触发条件，减少无效 CI 运行。

### Fixed
- 修复点击「让它生气试试」在猫咪睡觉、好奇或刚隐藏时没有立即生效的问题；点击测试按钮时若猫咪处于隐藏状态会自动显示。

## [2.1.0] - 2026-11-24

### Added
- 完成 macOS 纯原生 Metal + AppKit 架构重写（复用 `crates/core` 与 `crates/ffi`），彻底移除 WKWebView 与 WebKit 依赖，输出通用二进制（Apple Silicon + Intel）。
- 上线 Homebrew Tap 仓库（`blycr/homebrew-tap`），macOS 支持 `brew install --cask --no-quarantine blycr/tap/keycat` 安装与升级。
- CI 新增 macOS 离屏 Metal 渲染冒烟测试与 `brew style` 校验。

## [2.0.0] - 2026-11-24

### Added
- 全新跨平台 Rust 3D/物理/行为核心引擎，以及基于 Direct3D 11（4x MSAA）+ DirectComposition + Direct2D 的 Windows 纯原生客户端，彻底移除 WebView2 依赖。
- 新增「自动」帧率档位（活跃时 60 fps / 显示器刷新率，发呆或睡着自动降帧），同时支持固定 30/60/显示器原生高刷/不限制档位。
- 提供约 390 KB 的简体中文 NSIS 安装程序（支持每用户安装、升级保留旧配置、标准卸载）。

## [1.1.0] - 2026-11-23

### Changed
- 在保留 1.x WebView 架构的基础上完成首轮零画质损失性能与内存优化，新增多档帧率调节。

## [1.0.x]

- 基于上游 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 的初始跨平台版本（macOS WKWebView + Windows Tauri/WebView2）。
