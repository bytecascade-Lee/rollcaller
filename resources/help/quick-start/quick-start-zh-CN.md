# 快速开始

本指南将带你在几分钟内完成「自动点名」的安装、配置学生名单并开始第一次点名。

[//]: # (@section: install-app)
## 安装应用 

本应用提供两种使用方式，任选其一：

- **安装包**：运行安装程序，按照提示完成安装，之后通过桌面或开始菜单的快捷方式启动应用。
- **便携版**：将压缩包解压到任意目录，直接运行其中的 `rollcaller.exe` 即可，无需安装。

> 便携版无需安装，适合放到 U 盘等便携设备中使用。如果系统无 WebView2 运行时，需自行安装，可从 [Microsoft WebView2 下载页面](https://developer.microsoft.com/zh-cn/microsoft-edge/webview2/#download-section) 获取。

[//]: # (@section: prepare)
## 准备名单

点名需要先有学生数据。支持两种方式添加学生：

[//]: # (@link: operation-steps)
- **逐个添加**：进入「学生管理」页面，点击添加按钮，填写学号和姓名后保存。适合学生数量较少的场景，详见 [添加单个学生](../add-student/add-student-zh-CN.md#操作步骤) 。
[//]: # (@link: import-steps)
- **批量导入**：如果你有现成的 Excel 学生名单，可以通过三步向导快速导入，详见 [批量导入学生](../batch-import/batch-import-zh-CN.md#导入步骤) 。

[//]: # (@section: start-rollcall)
## 开始点名

学生名单就绪后：

1. 点击左侧导航栏进入「点名」页面。
2. 确认页面左侧显示的学生列表非空。
3. 设置次数为1次，然后点击「开始点名」按钮开始点名，名单会持续滚动，需手动点击「停止点名」，停止后被点到的学生会高亮展示。
4. 如需连续点名，先设定点名次数（大于1），再点击「开始点名」，应用会自动连续点名直到次数完成，支持中途结束。

[//]: # (@link: operation-steps)
[//]: # (@link: operation-steps)
点名结果会自动保存为考勤记录。详见 [单次点名](../single-rollcall/single-rollcall-zh-CN.md#操作步骤) 与 [连续点名](../auto-finish/auto-finish-zh-CN.md#操作步骤) 。

## 查看记录

点名结束后：

1. 进入「历史记录」页面，可以看到所有点名记录，并按点名轮次自动分组。
[//]: # (@link: operation-steps)
2. 你可以为记录标记出勤状态（如出勤、缺勤、迟到等）或补充备注，详见 [修改记录](../edit-record/edit-record-zh-CN.md#操作步骤) 。
[//]: # (@link: operation-steps)
3. 需要留存或上报时，可将记录导出为 Excel 文件，详见 [导出记录](../export-record/export-record-zh-CN.md#操作步骤) 。

## 常见问题

**问：在哪里查看数据？**

答：所有数据均保存在本机，无需联网。

**问：能不能改学生信息？**

[//]: # (@link: operation-steps)
答：可以，在「学生管理」页面即可编辑，详见 [修改学生信息](../edit-student/edit-student-zh-CN.md#操作步骤) 。

**问：误删了学生怎么办？**

[//]: # (@link: operation-steps)
答：当前版本暂无独立的恢复按钮，但可通过「添加学生」功能输入相同的学号和姓名，系统会自动恢复该学生。详见 [删除学生](../delete-student/delete-student-zh-CN.md#操作步骤) 。
