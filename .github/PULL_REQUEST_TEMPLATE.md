## 改动说明

简要说明本次 PR 解决的问题、实现方式以及影响范围。

## 关联 Issue

- Closes #

## 测试与检查

- [ ] 已运行 `cargo test -p keycat-core -p keycat-ffi` 且全部通过
- [ ] Windows 改动已在本地完成 release 编译并验证基础渲染与托盘/设置交互
- [ ] macOS 改动已验证 Metal 渲染、键鼠监听与设置窗口交互
- [ ] 若涉及 `apps/windows/installer.nsi`，已保留 UTF-8 BOM
- [ ] 若涉及用户可见变更，已同步更新文档
