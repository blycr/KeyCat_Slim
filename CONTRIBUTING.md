# 贡献指南 (Contributing)

欢迎为 KeyCat Slim 提交问题反馈、改进建议或 Pull Request。

## 开发环境要求

- **通用**：Rust 稳定版（建议 1.80+）与 Git。
- **Windows 构建**：Windows 10/11 + MSVC 工具链；打包安装程序需要 [NSIS](https://nsis.sourceforge.io/)。
  - 在 Linux / WSL 环境下可通过 `cargo-xwin` 与 `llvm-rc` / `llvm-lib` 交叉编译 `x86_64-pc-windows-msvc`。
- **macOS 构建**：macOS 13+、Xcode Command Line Tools，并安装双架构 Rust 目标：
  ```sh
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  ```

## 本地开发与校验

请在提交或发起 PR 前先在本地完成校验，尽量避免依赖云端 CI 反复试错：

```sh
# 运行跨平台核心与 FFI 单元测试
cargo test -p keycat-core -p keycat-ffi

# Windows 原生 Release 编译（Windows 本机）
cargo build --release -p keycat-native

# Windows 原生 Release 交叉编译（Linux / WSL）
cargo xwin build --release --target x86_64-pc-windows-msvc -p keycat-native

# macOS 原生 Metal 通用包编译与冒烟测试（macOS）
sh apps/macos/build.sh
build/KeyCat.app/Contents/MacOS/KeyCat --smoke-test dist/smoke.png
```

## 代码与文件规范

- **编码与换行**：遵循根目录 `.editorconfig` 与 `.gitattributes`：
  - 所有源码与 Shell 脚本使用 UTF-8（无 BOM）和 LF 换行。
  - `apps/windows/installer.nsi` 必须保留 **UTF-8 BOM**，且编译时传入 `-INPUTCHARSET UTF8`，确保中文安装向导在英文环境构建时不乱码。
- **零 WebView 原则**：两端应用均不得引入 WebView2、WKWebView、Electron 或内置浏览器内核。
- **跨平台一致性**：与 3D 造型、毛色、物理和状态机相关的修改优先落在 `crates/core`，让 Windows 与 macOS 同步受益。
- **文案风格**：界面、文档、Release Note 与提交记录保持简洁自然，不使用 emoji。

## 维护者发版规范

### 版本号同步清单

1. 根目录 `Cargo.toml` 的 `[workspace.package].version`（`crates/core`、`crates/ffi`、`apps/windows` 及 Windows 托盘显示会自动继承）。
2. `apps/windows/installer.nsi` 中的默认 `VERSION`。
3. `apps/macos/Info.plist` 中的 `CFBundleShortVersionString` 与 `CFBundleVersion`（后者每次发版递增 1）。
4. 更新 `CHANGELOG.md`。

### 推送标签与 Tap 同步

```sh
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main vX.Y.Z
```

标签推送会自动触发 `Release Windows` 与 `Release macOS` 两个工作流，构建并上传全部安装包与 `.sha256` 到对应的 GitHub Release。构建完成后，使用新的 `KeyCat-macOS.zip.sha256` 同步更新 [blycr/homebrew-tap](https://github.com/blycr/homebrew-tap) 中的 `Casks/keycat.rb`。

### Release Note 标准模板

每次发布正式版本必须按以下固定结构编写 Release Note（按实际变更裁剪空节，不随意发挥，不使用 emoji）：

```markdown
## 更新摘要

用 1-2 句话概括本版本的核心定位与主要变化。

## 新增功能

- 按平台或模块明确说明新增了什么能力。

## 优化与改进

- 明确说明体验、性能、界面或工程上的具体改进。

## 问题修复

- 明确说明修复了什么现象及其触发条件。

## 兼容性与升级说明

- 说明配置兼容性、系统版本要求、已知限制或破坏性变更（无特殊说明可精简为一行）。

## 下载与安装

- **Windows**：下载 `KeyCat-Windows-Setup.exe` 双击安装（自动保留原有配置）。
- **macOS**：推荐使用 Homebrew 安装或升级：
  ```sh
  brew install --cask --no-quarantine blycr/tap/keycat
  # 已安装用户直接升级
  brew upgrade --cask keycat
  ```
  也可手动下载 `KeyCat-macOS.zip` 解压至 `/Applications`。
- **完整性校验**：下载对应 `.sha256` 文件校验安装包完整性。

**完整变更记录**：https://github.com/blycr/KeyCat_Slim/compare/v上一版本...v当前版本
```
