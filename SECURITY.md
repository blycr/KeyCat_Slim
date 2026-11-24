# 安全与隐私说明 (Security & Privacy)

由于 KeyCat Slim 需要监听全局键盘与鼠标事件以驱动桌面小猫的动作，我们在此明确说明项目的隐私边界与安全实践。

## 隐私承诺

- **不记录输入内容**：KeyCat Slim 仅监听按键的按下/抬起状态（用于驱动左右爪、尾巴动作与可选按键弹字）以及鼠标指针位置（用于转头与摸头判定），不会记录、保存或解析用户输入的文本、密码或剪贴板内容。
- **完全本地运行**：程序不内置统计埋点、广告或遥测上报，不会向任何第三方服务器发送键盘或鼠标数据。
- **最小权限原则**：
  - Windows 端默认以普通用户权限运行，仅在用户主动勾选「以管理员身份运行」（用于兼容高权限游戏）时才会通过系统 UAC 申请提权。
  - macOS 端遵循系统「输入监控」授权模型，且在遇到系统密码框（Secure Input）时不会绕过系统保护，退出后自动恢复按键状态。
- **可审计源码**：所有输入钩子与事件监听实现完全开源，可直接查阅：
  - Windows：`apps/windows/src/hook.rs`
  - macOS：`apps/macos/App.swift`

## 校验发布产物完整性

每个正式 Release 均同时提供安装包与对应的 `.sha256` 校验文件。由于个人开源项目未购买商业代码签名证书，建议下载后核对 SHA-256：

- **Windows (PowerShell)**：
  ```powershell
  Get-FileHash .\KeyCat-Windows-Setup.exe -Algorithm SHA256
  ```
- **macOS (Terminal)**：
  ```sh
  shasum -a 256 KeyCat-macOS.zip
  ```

将输出结果与 Release 页面中对应 `.sha256` 文件内容比对一致即可确认未被篡改。

## 漏洞报告

如果你发现安全隐患（例如权限提升绕过、更新包校验缺陷或未预期的敏感信息泄露）：

1. 请尽量通过 GitHub 的 [Private Vulnerability Reporting](https://github.com/blycr/KeyCat_Slim/security/advisories/new) 私密上报，避免在公开 Issue 中直接暴露利用细节。
2. 请附上受影响的版本、平台、复现步骤与潜在影响说明。
3. 我们会尽快核实并安排修复发布。
