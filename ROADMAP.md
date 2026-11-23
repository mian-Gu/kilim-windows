# 项目进度

更新日期：2026-11-23

## 当前阶段

Windows 原生版本已独立放入 `windows/`。功能对照和开发说明见 `windows/PARITY.md` 与 `windows/README.md`。

## 已验证

- 当前源码记录了 63 项 release 回归、clippy、release 构建及离屏渲染验证通过。
- Explorer 桌面层级、透视穿透和隐藏后恢复有本机冒烟验证记录。

## 待验收

- 使用 2026-11-21 录屏中的全屏角落拖动操作复测实际桌面表现。
- 实际体验持续拖拽、声音、图片导入、F8 文件拖放和托盘菜单。
- 验证 Explorer 重启、显示桌面、屏幕配置变化，以及不同 Windows 版本、混合 DPI 和显卡环境。
- 完成功能逐项对照，见 `windows/PARITY.md`。

## 仓库状态

- 本目录从原仓库的已提交 `windows/` 路径导出，仅包含 Windows 工程和必要项目文件。
- 新仓库已初始化并推送到 `https://github.com/mian-Gu/kilim-windows.git`，分支为 `main`。
- 原仓库的两个本地 Windows 提交包含在导出文件中，但合并为新仓库的初始提交 `0367385`，未保留原提交历史。
- 原仓库未跟踪的 `.workbuddy/` 未导入，原仓库文件未删除。
