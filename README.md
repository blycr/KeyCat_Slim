# KeyCat Slim

[English](README_EN.md) | 简体中文

[![Latest Release](https://img.shields.io/github/v/release/blycr/KeyCat_Slim?label=Release)](https://github.com/blycr/KeyCat_Slim/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2B%20%7C%20macOS%2013%2B-lightgrey)](https://github.com/blycr/KeyCat_Slim/releases/latest)

一只趴在键盘上的轻量原生桌面宠物猫：你打字它伸爪按键，动鼠标它扭头看，摸头会闭眼，没人理会睡着，被烦到会用尾巴捶桌。

KeyCat Slim 基于 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 进行原生轻量化重构。Windows 端与 macOS 端均已彻底移除 WebView2 与 WKWebView，共用同一套跨平台 Rust 3D 引擎，在保持原有画面细节与高刷流畅度的同时大幅降低体积与系统资源占用。

![KeyCat 演示](assets/media/keycat.gif)

## 特性

- **双端纯原生渲染，零浏览器内核**
  - Windows：Rust + Direct3D 11（4x MSAA）+ DirectComposition + Direct2D，安装包仅约 390 KB，不依赖 WebView2。
  - macOS：Swift + Metal（4x MSAA）+ AppKit 通用二进制（Apple Silicon + Intel），压缩包约 1.6 MB，不链接 WebKit。
- **跨平台共享核心**
  - 3D 几何、11 种毛色、弹簧物理与行为状态机统一实现于 Rust 核心库，双端观感与交互保持一致。
- **灵活的帧率控制**
  - 默认「自动」档：交互活跃时最高 60 fps（显示器低于 60 Hz 时取显示器刷新率），发呆或入睡后自动降帧省电。
  - 支持手动锁定 `30 fps`、`60 fps`、显示器原生高刷（自动识别 `120` / `144` / `165` / `240 fps`）或「不限制」。
- **丰富的外观、桌面与性格调节**
  - 支持 11 种毛色、2D/3D 风格切换、瞳色与高光、手套颜色、项圈、腮红、胡须、节日装扮（圣诞、万圣节、春节、周年庆）。
  - 支持 4 套键盘配色、鼠标线开关、可互动的桌上咖啡杯、4 档窗口透明度、窗口缩放，以及内置 5 套性格预设（默认 / 高冷 / 粘人 / 暴躁 / 胆小）的独立调节窗口。
- **陪伴细节与专注工具**
  - 摸头可触发实时算法合成呼噜声与踩奶动作（默认静音，零音频素材包），双击猫头可触发慢眨眼回应；内置熟睡霸占键盘、打哈欠、理毛、推杯子等小动作。
  - 支持连续工作提醒、深夜催睡与番茄钟模式，本地仅按天记录互动次数以累积熟悉度，绝不记录任何按键内容。
- **隐私友好且输入稳定**
  - 不联网上传任何数据，不记录输入内容，仅响应按键起落与鼠标坐标；内置锁屏、UAC 及密码框后的卡键自愈机制。

## 安装

### Windows（Windows 10 / 11，64 位）

从 [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) 下载 `KeyCat-Windows-Setup.exe` 并运行安装。

- 默认安装至受系统权限保护的 `C:\Program Files\KeyCat`（安装时需确认一次 UAC），从旧版升级时会自动迁移原 `%LOCALAPPDATA%\KeyCat` 安装并保留全部配置，卸载时同步清理自启与计划任务。
- 因个人开源项目未购买商业代码签名证书，Windows SmartScreen 可能会提示「无法确认谁创建了此程序」，点击「更多信息 → 仍要运行」即可。

### macOS（macOS 13 Ventura 及以上，Apple Silicon / Intel）

**推荐通过 Homebrew Cask 安装**：

```sh
brew install --cask blycr/tap/keycat
```

升级与卸载：

```sh
brew upgrade --cask keycat
brew uninstall --cask keycat          # 卸载应用
brew uninstall --cask --zap keycat    # 卸载并清理配置
```

**手动下载**：从 [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) 下载 `KeyCat-macOS.zip`，解压后拖入 `/Applications` 即可。

- 首次运行需在「系统设置 → 隐私与安全性 → 输入监控」中允许 KeyCat，然后重启应用。
- 若未使用 `--no-quarantine` 或手动下载后系统提示「已损坏」，在终端执行：
  ```sh
  xattr -dr com.apple.quarantine /Applications/KeyCat.app
  ```

## 使用说明

小猫常驻在系统托盘（macOS 为菜单栏），右键小猫或点击托盘/菜单栏图标即可打开菜单：

- **移动与穿透**：按住小猫身体可随意拖动位置，双击猫头可对它慢眨眼；直播、录屏或全屏工作时可开启「鼠标穿透」，点击会直接落到下层窗口（此时通过托盘或菜单栏操作）。
- **外观、桌面与习性**：右键菜单分为「外观」「桌面」「习性」三大分组，可切换毛色、手套、键盘配色、咖啡杯、透明度、呼噜声音与帧率；菜单顶部可一键开关番茄钟，点击「习性 → 调节反应…」可切换 5 套性格预设或微调各项阈值，按 `Esc` 可关闭调节窗口。
- **隐私与隐藏**：开启「隐私模式」后不再弹出按键提示标签；点击「隐藏」会暂停画面渲染与声音。

### Windows 游戏与权限说明

- 部分游戏以管理员身份运行，普通权限的 KeyCat 无法接收发给高权限窗口的按键。可在托盘菜单开启「以管理员身份运行」通过 UAC 提权。
- 同时开启「开机自启」与「以管理员身份运行」时，会改用 Windows 计划任务启动，后续登录和手动打开不再重复弹 UAC。
- 以管理员权限运行时，普通权限录屏软件按窗口采集可能抓不到小猫，请改用整屏采集；独占全屏游戏会遮挡所有浮窗，请切换为无边框窗口模式。

## 从源码构建

需要 Rust 稳定版（1.80+）；构建 macOS 应用还需 Xcode Command Line Tools 与 `aarch64-apple-darwin` / `x86_64-apple-darwin` 目标。

```sh
# 运行跨平台核心与 FFI 单元测试
cargo test -p keycat-core -p keycat-ffi

# 构建 Windows 原生版（Windows 本机）
cargo build --release -p keycat-native

# 构建并打包 macOS 通用版（macOS 本机）
sh apps/macos/build.sh
sh apps/macos/package.sh
```

## 项目结构

```text
.
├── Cargo.toml         # 根级 Rust Workspace
├── crates/
│   ├── core/          # 跨平台 3D 几何、毛色、弹簧物理与行为状态机
│   └── ffi/           # 供 macOS Swift 调用的 C-ABI 静态库
├── apps/
│   ├── windows/       # Windows 原生应用（Direct3D 11 + DirectComposition + Direct2D）
│   └── macos/         # macOS 原生应用（Swift + Metal + AppKit）
└── assets/
    ├── icons/         # 应用与托盘图标
    └── media/         # 演示动图
```

## 相关文档

- [更新日志](CHANGELOG.md)
- [贡献指南](CONTRIBUTING.md)
- [安全与隐私说明](SECURITY.md)
- [第三方许可与致谢](NOTICE)

## 开源协议

本项目基于 [MIT License](LICENSE) 开源，源自 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 并完成原生重构，详细版权与依赖说明见 [NOTICE](NOTICE)。
