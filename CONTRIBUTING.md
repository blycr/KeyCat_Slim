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
- **文案风格**：界面、文档与提交记录保持简洁直观，不使用 emoji。

## 版本发布流程（维护者参考）

1. 更新版本号：
   - 根目录 `Cargo.toml` 的 `[workspace.package].version`（`crates/core`、`crates/ffi`、`apps/windows` 及托盘显示会自动继承）。
   - `apps/windows/installer.nsi` 中的默认 `VERSION`。
   - `apps/macos/Info.plist` 中的 `CFBundleShortVersionString` 与 `CFBundleVersion`（后者每次发版递增 1）。
   - 更新 `CHANGELOG.md`。
2. 在本地运行测试与编译，确认零错误、零警告。
3. 提交合入 `main`，打标注标签并推送：
   ```sh
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin main vX.Y.Z
   ```
4. 标签推送会自动触发 `Release Windows` 与 `Release macOS` 两个工作流，构建并上传全部安装包与 `.sha256` 到对应的 GitHub Release。
5. 使用新生成的 `KeyCat-macOS.zip.sha256` 同步更新 [blycr/homebrew-tap](https://github.com/blycr/homebrew-tap) 中的 `Casks/keycat.rb`。
