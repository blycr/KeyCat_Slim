# KeyCat Slim

[English](README_EN.md) | 简体中文

[![Latest Release](https://img.shields.io/github/v/release/blycr/KeyCat_Slim?label=Release)](https://github.com/blycr/KeyCat_Slim/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2B%20%7C%20macOS%2013%2B-lightgrey)](https://github.com/blycr/KeyCat_Slim/releases/latest)

一只趴在键盘上的轻量原生桌面宠物猫：你打字它伸爪按键，动鼠标它扭头看，摸头会闭眼，没人理会睡着，被烦到会用尾巴捶桌。

KeyCat Slim 基于 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 进行全面原生轻量化重构，**Windows 端和 macOS 端均已彻底移除 WebView2 与 WKWebView**，共用同一套跨平台 Rust 3D 核心引擎，在保留完整画面细节与高刷流畅度的同时大幅降低体积与资源占用。

![KeyCat 演示](assets/media/keycat.gif)

## 核心特性

- **双端纯原生渲染，零浏览器内核**
  - Windows：Rust + Direct3D 11（4x MSAA）+ DirectComposition + Direct2D，NSIS 安装包仅约 390 KB，不依赖 WebView2。
  - macOS：Swift + Metal（4x MSAA）+ AppKit 通用二进制（Apple Silicon + Intel），压缩包约 1.6 MB，不链接 WebKit。
- **跨平台 Rust 统一核心**
  - 3D 几何、11 种毛色、弹簧物理与情绪状态机全部收拢在 `crates/core`，双端观感与动作逻辑保持一致。
- **智能帧率控制**
  - 默认开启「自动」档：交互活跃时最高 60 fps（显示器低于 60 Hz 时跟随刷新率），发呆或入睡后自动降帧省电。
  - 也支持手动锁定 `30 fps`、`60 fps`、显示器原生高刷（自动识别 `120` / `144` / `165` / `240 fps`）或「不限制」。
- **丰富外观与细粒度调节**
  - 支持 11 种毛色、2D/3D 风格切换、瞳色与高光、项圈、腮红、胡须、窗口大小、入睡时间与烦躁/好奇灵敏度调节。
- **隐私友好与输入稳定**
  - 不联网上传任何数据，不记录输入内容，仅响应按键起落与鼠标位置；内置锁屏/UAC/密码框后的卡键自愈机制。

## 安装

### Windows（Windows 10 / 11，64 位）

从 [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) 下载 `KeyCat-Windows-Setup.exe` 并双击安装。

- 默认安装到当前用户目录（`%LOCALAPPDATA%\KeyCat`），无需管理员权限，可在系统「已安装的应用」中正常卸载，并保留原有配置。
- 因个人开源项目未购买商业代码签名证书，Windows SmartScreen 可能会提示「无法确认谁创建了此程序」，点击「更多信息 → 仍要运行」即可。

### macOS（macOS 13 Ventura 及以上，Apple Silicon / Intel）

**推荐通过 Homebrew Cask 安装**：

```sh
brew install --cask --no-quarantine blycr/tap/keycat
```

后续升级与卸载：

```sh
brew upgrade --cask keycat
brew uninstall --cask keycat          # 仅卸载应用
brew uninstall --cask --zap keycat    # 卸载并清理配置文件
```

**手动安装**：从 [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) 下载 `KeyCat-macOS.zip`，解压后拖入 `/Applications` 即可。

- 首次运行需在「系统设置 → 隐私与安全性 → 输入监控」中勾选 KeyCat，随后重启应用。
- 若未使用 `--no-quarantine` 或手动下载后提示「已损坏」，在终端执行：
  ```sh
  xattr -dr com.apple.quarantine /Applications/KeyCat.app
  ```

## 日常使用

小猫常驻在系统托盘（macOS 为菜单栏），右键小猫或点击托盘/菜单栏图标即可打开菜单：

- **移动与穿透**：按住小猫身体可随意拖动；直播、录屏或全屏工作时可开启「鼠标穿透」，点击直接穿透到底层窗口（此时通过托盘/菜单栏管理）。
- **外观与行为**：在菜单中切换毛色、2D/3D 风格、尺寸和帧率；打开「调节反应…」可实时测试生气/好奇动画并细调触发灵敏度，支持按 `Esc` 快速关闭调节窗口。
- **隐私与隐藏**：开启「隐私模式」后不再弹出按键提示标签；点击「隐藏」会立即暂停画面渲染。

### Windows 游戏与权限提示

- 许多游戏会以管理员身份运行，普通权限的 KeyCat 无法接收发给高权限窗口的按键。可在托盘菜单开启「以管理员身份运行」通过 UAC 提权。
- 同时开启「开机自启」与「以管理员身份运行」时，KeyCat 会改用 Windows 计划任务启动，后续登录或手动打开不再重复弹 UAC。
- 以管理员权限运行时，普通权限录屏软件按窗口采集可能抓不到小猫，请改用整屏采集；独占全屏游戏会遮挡所有浮窗，请切换为无边框窗口模式。

## 架构与目录结构

```text
.
├── Cargo.toml                 # 根级 Rust Workspace（统一版本与发布编译参数）
├── crates/
│   ├── core/                  # 跨平台 Rust 引擎：3D 几何、毛色、弹簧物理与行为状态机
│   └── ffi/                   # 供 macOS Swift 调用的 C-ABI 静态库（libkeycat_ffi.a）
├── apps/
│   ├── windows/               # Windows 原生外壳（Direct3D 11 + DirectComposition + Direct2D + NSIS）
│   └── macos/                 # macOS 原生外壳（Swift + Metal + AppKit + 构建打包脚本）
├── assets/
│   ├── icons/                 # 应用与托盘图标资源
│   └── media/                 # 文档演示动图
└── .github/
    ├── workflows/             # Windows / macOS 双端标签发布工作流
    └── ISSUE_TEMPLATE/        # Issue 与 PR 模板
```

## 从源码构建

```sh
# 1. 运行跨平台核心与 FFI 单元测试
cargo test -p keycat-core -p keycat-ffi

# 2. 构建 Windows 原生版（在 Windows 上执行）
cargo build --release -p keycat-native
# Linux / WSL 环境可使用 cargo-xwin 交叉编译：
# cargo xwin build --release --target x86_64-pc-windows-msvc -p keycat-native

# 3. 构建并打包 macOS 通用版（在 macOS 上执行，需 Rust 与 Xcode Command Line Tools）
sh apps/macos/build.sh         # 生成 build/KeyCat.app
sh apps/macos/package.sh       # 生成 dist/KeyCat-macOS.zip + .sha256 + latest-mac.json
```

更多本地环境准备、发版步骤与协作约定请查看 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 相关文档

- 更新日志：[CHANGELOG.md](CHANGELOG.md)
- 贡献与发版指南：[CONTRIBUTING.md](CONTRIBUTING.md)
- 隐私与安全说明：[SECURITY.md](SECURITY.md)
- 上游致谢与第三方依赖：[NOTICE](NOTICE)

## 致谢与许可

本项目以 [MIT License](LICENSE) 开源，基于 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 的创意与初代实现重构而来。详细版权归属与第三方依赖声明见 [NOTICE](NOTICE)。
