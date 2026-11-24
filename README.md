# KeyCat Slim

一只趴在键盘上的轻量桌面猫：你打字它伸爪按键，动鼠标它扭头看，摸头会闭眼，没人理会睡着，被烦到会用尾巴捶桌。

本项目基于 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 进行轻量化重构，支持 Windows 10+ 和 macOS 13+，不收集任何输入内容。

![KeyCat](docs/screenshots/keycat.gif)

## 推荐版本：v2.0.0

建议直接安装最新的 [v2.0.0](https://github.com/blycr/KeyCat_Slim/releases/latest)，主要原因如下：

- **更省资源**：Windows 端已彻底去掉 WebView2，改用 Rust + Direct3D 11（4x MSAA）+ DirectComposition 原生渲染，在保持原有观感和流畅度的同时，内存与 CPU 占用大幅下降，安装程序不到 400 KB。
- **更合理的帧率控制**：菜单新增「自动」帧率档位（默认开启），活跃时最高 60 fps（显示器低于 60 Hz 时取显示器刷新率），发呆或睡着时自动降帧省电；手动选固定档位时则严格锁定，不再私自降帧。
- **高回报率鼠标更顺滑**：Windows 端鼠标监听改用 Raw Input，1000–8000 Hz 游戏鼠标移动不卡顿；隐藏小猫时自动暂停渲染。

## 安装

### Windows（推荐）

从 [Releases](https://github.com/blycr/KeyCat_Slim/releases/latest) 下载 `KeyCat-Windows-Setup.exe` 并运行安装。

- 安装到当前用户目录（`%LOCALAPPDATA%\KeyCat`），无需管理员权限，支持在 Windows「已安装的应用」中正常卸载，并沿用原有配置文件。
- 因为个人开源项目未购买商业代码签名证书，Windows SmartScreen 可能会提示「无法确认谁创建了此程序」，点「更多信息 → 仍要运行」即可。

### macOS

在终端执行以下命令安装并启动（支持 Apple Silicon 与 Intel）：

```sh
curl -L -o /tmp/KeyCat-macOS.zip https://github.com/blycr/KeyCat_Slim/releases/latest/download/KeyCat-macOS.zip \
  && unzip -qo /tmp/KeyCat-macOS.zip -d /Applications && open /Applications/KeyCat.app
```

- 首次启动需在「系统设置 → 隐私与安全性 → 输入监控」里允许 KeyCat，然后重启应用。它只读取按键事件，不记录输入内容。
- 如果通过浏览器手动下载并提示「已损坏」，在终端执行 `xattr -dr com.apple.quarantine /Applications/KeyCat.app`。

## 使用

小猫常驻在系统托盘（macOS 为菜单栏）。右键小猫或点击托盘/菜单栏图标即可打开菜单：

- **外观与互动**：支持 11 种毛色、2D/3D 风格切换、眼睛颜色与高光、项圈、腮红、胡须、窗口大小和入睡时间调节；在「调节反应…」里可以细调烦躁与好奇的灵敏度。
- **帧率设置**：默认「自动」，也可以手动固定为 `30 fps`、`60 fps`、显示器原生高刷（自动识别，如 `120` / `144` / `165` / `240 fps`）或「不限制」。
- **拖动与穿透**：按住小猫身体可以拖动位置；直播或录屏时可开启「鼠标穿透」，点击会直接落到下层窗口，此时菜单需通过托盘或菜单栏打开。
- **隐私与隐藏**：开启「隐私模式」后不再弹出按键名；点击「隐藏」会暂停画面渲染。

### Windows 游戏与权限说明

- 很多游戏以管理员身份运行，普通权限的 KeyCat 收不到发给它们的按键。在托盘菜单打开「以管理员身份运行」后，KeyCat 会通过 UAC 以管理员权限重启。
- 同时开启「开机自启」与「以管理员身份运行」时，会改用计划任务启动，后续登录和手动打开都不再重复弹 UAC。关闭该选项后，需下次重启 KeyCat 恢复普通权限。
- 以管理员身份运行时，普通权限录屏软件按窗口采集可能抓不到小猫，请改用整屏采集；独占全屏游戏会盖住所有浮窗，如需显示小猫请将游戏切换为无边框窗口模式。

## 构建与发布

### 本地构建

```sh
# Windows 原生版（推荐，不带 WebView2）
cd native
cargo test -p keycat-core
cargo build --release -p keycat-native
# Linux/WSL 环境可使用 cargo-xwin 交叉编译：
# cargo xwin build --release --target x86_64-pc-windows-msvc -p keycat-native

# macOS 通用版（需 Xcode 命令行工具）
sh mac/build.sh
sh mac/package.sh

# 旧版 Windows（Tauri + WebView2，仅作兼容保留）
cd win && npm ci && npm run dist
```

### 发布流程

推送 `vX.Y.Z` 标签后，GitHub Actions 会自动编译并上传两端产物到对应 Release：

1. 同步更新 `native/core/Cargo.toml`、`native/win/Cargo.toml` 与 `mac/Info.plist` 中的版本号（macOS 的 `CFBundleVersion` 每次发布加 1），合入 `main`。
2. 推送 `vX.Y.Z` 标签：
   - `.github/workflows/native-windows.yml` 会构建并上传 `KeyCat-Windows-Setup.exe` 与 `.sha256`。
   - `.github/workflows/macos.yml` 会构建并上传 `KeyCat-macOS.zip`、`.sha256` 与 `latest-mac.json`。

## 目录结构

```text
native/    原生重构版：core/ 为跨平台 3D 与动作逻辑，win/ 为无 WebView2 的 Windows 外壳（D3D11 + DirectComposition + Direct2D）及 NSIS 安装脚本
mac/       macOS 外壳（Swift）：浮窗、菜单、键鼠监听、自动更新、构建/签名/打包脚本
web/       three.js 场景（供 macOS 及旧版 Windows 共用）：猫、键盘、动作与参数定义
win/       旧版 Windows 外壳（Tauri + WebView2），保留用于对照和兼容
site/      下载展示页
tools/     开发与调试脚本
```

## 说明与致谢

本项目基于 [ElloboDK/KeyCat](https://github.com/ElloboDK/KeyCat) 进行轻量化改进，仅供个人使用与学习交流，不用于商业分发。three.js 为 MIT 许可（见 `web/THREE_LICENSE`），Tauri 为 MIT / Apache-2.0 双许可。

<details>
<summary>展开查看原仓库（KeyCat 1.x）原始说明</summary>

> 以下内容节选自上游原仓库 `ElloboDK/KeyCat` 的原始 README，保留作历史与旧版（1.x WebView2）参考。

一只趴在键盘上的桌面猫：你打字它伸爪按键，动鼠标它扭头看，摸头会闭眼，没人理会睡着，被烦到会用尾巴捶桌。支持 macOS 13+ 和 Windows 10+，除了从 GitHub 检查和下载更新，不联网。

**旧版签名与构建备注**：

- mac 版用 `signing/` 里自动生成的本地证书签名，这样重新编译后「输入监控」授权仍然有效。mac 自动更新只接受同一张证书签名的新版，所以发布包必须在有这个目录的机器上打，证书丢了已安装的 mac 版也收不到更新。
- 旧版 Windows（Tauri）自动更新的签名私钥同样放在 `signing/` 下（`tauri-updater.key` 和 `tauri-updater.password`），对应仓库的 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 两个 Secret，公钥写在 `win/tauri.conf.json`。私钥丢了已安装的版本就收不到更新，需妥善备份且不要提交 `signing/` 目录。

</details>
