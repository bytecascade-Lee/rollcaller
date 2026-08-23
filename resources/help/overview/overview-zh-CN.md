# 概览

欢迎使用「自动点名」—— 这是一款**轻量**、**好用**的课堂自动点名工具。它可以帮助老师从学生名单中**随机抽取**学生，**自动记录**每次点名的考勤情况，并支持对学生和考勤记录进行**全面管理**。

## 核心功能

本应用围绕三个核心页面展开，所有操作都可以通过左侧导航栏快速切换：

- **自动点名**：从学生名单中随机抽取学生，支持单次点名和连续点名两种模式，点名结果自动保存为考勤记录。支持语音播报，点名后自动朗读学生姓名。
- **学生管理**：维护班级的学生名单，支持添加、编辑、删除、批量导入和导出学生。
- **历史记录**：查看全部点名记录，支持批量修改考勤状态和备注，并可将记录导出为 Excel 文件。

### 自动点名

- 随机抽取学生，动画流畅、结果清晰展示。
- 支持**单次点名**（手动开始、手动停止）和**连续点名**（设定次数后自动连续点名）。
- 可控制是否允许重复点名，避免或允许同一学生被多次点到。
- 点名结果自动保存，并按点名轮次分组展示。
- 支持**语音播报**，点名后自动朗读学生姓名，支持本地语音和云端 AI 两种模式。详见 [语音播报](../tts/tts-zh-CN.md)。

### 学生管理

- 通过学号和姓名维护学生名单。
- 支持从 Excel 文件**批量导入**学生，自动处理重复数据。
- 支持将全部、筛选后或选中的学生**导出**为 Excel 文件。
- 支持快速搜索、排序。

### 历史记录

- 查看完整的点名考勤记录列表，按点名轮次自动分组。
- 可**批量修改**记录的状态（出勤、缺勤、迟到、早退、请假等）和备注。
- 支持将全部、筛选后或选中的记录**导出**为 Excel 文件。

## 数据与隐私

- 所有数据均保存在本地设备上，**无需联网**，不依赖任何外部服务器。
- 升级版本时，应用会自动完成数据迁移，无需手动干预（早期版本升级前建议备份数据）。

## 系统要求

- 支持**新版 Windows 10 和 Windows 11**，暂不支持 Windows 7 及以下系统。
- 应用依赖 WebView2 运行时。若系统未安装，**安装包**会在安装过程中引导你完成安装；
  **便携版不支持引导安装**，需自行从 [Microsoft WebView2 下载页面](https://developer.microsoft.com/zh-cn/microsoft-edge/webview2/#download-section) 获取并安装后方可使用。
- 提供安装包和便携版两种使用方式，便携版解压后即可直接运行。

## 获取与支持

- 下载最新版本：请访问 [Latest Releases 页面](https://github.com/bytecascade-Lee/rollcaller/releases/latest) 获取最新安装包或便携版压缩包，新版本会持续修复问题并优化体验。
- 下载以往版本：请访问 [Releases 页面](https://github.com/bytecascade-Lee/rollcaller/releases) 获取所有已发布的安装包或便携版压缩包。
- 反馈建议：如果你在使用中遇到问题，或有功能建议，欢迎通过 [Issues 页面](https://github.com/bytecascade-Lee/rollcaller/issues) 提交反馈，我们会及时跟进处理。

## 从这里开始

- 还不熟悉本应用？请阅读 [快速开始](../quick-start/quick-start-zh-CN.md) 一文，几分钟即可上手。
[//]: # (@link: operation-steps)
[//]: # (@link: operation-steps)
- 想了解如何点名？请参考 [单次点名](../single-rollcall/single-rollcall-zh-CN.md#操作步骤) 和 [连续点名](../auto-finish/auto-finish-zh-CN.md#操作步骤) 。
[//]: # (@link: operation-steps)
[//]: # (@link: import-steps)
- 想管理学生名单？请从 [添加学生](../add-student/add-student-zh-CN.md#操作步骤) 或 [批量导入学生](../batch-import/batch-import-zh-CN.md#导入步骤) 开始。
[//]: # (@link: modes)
- 想开启语音播报？请参考 [语音播报](../tts/tts-zh-CN.md#播报模式)。
- 想了解窗口操作？请参考 [主窗口](../app-window/app-window-zh-CN.md) 和 [帮助窗口](../help-window/help-window-zh-CN.md)。

## 关于

- [自述文件](../../../README.md)
- [更新日志](../../../CHANGELOG.md)
- [版本发布说明](../../../RELEASE_NOTES.md)
- [开源许可证](../../../LICENSE)
