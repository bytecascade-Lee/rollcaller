# CHANGELOG

All notable changes to this project will be documented in this file.

---

## 0.4.0

### Breaking Changes

- 重构目录结构，新增 `root_dir` 区分应用根目录与数据目录，`base_dir` 重命名为 `user_data_dir`
- 重命名 `webview_dir` 为 `webview2_dir`，统一命名规范
- 配置常量生成改由 Python 脚本 `generate_config_constants.py` 驱动，`build.rs` 不再内联解析逻辑

### Added

- 新增 Windows ARM64 (aarch64-pc-windows-msvc) 架构的 CI 构建与发布支持
- 新增 Python 构建脚本工具链（`scripts/common/`），包含版本号处理 `version.py`、Git 操作 `git.py`、tauri-cli 查找 `tauri_cli.py`、打包 `packager.py`、构建 `builder.py`、目标解析 `targets.py`、日志 `logger.py` 模块
- 新增本地构建发布脚本 `release_local.py`，支持指定版本号和多架构（x64/arm64/all）构建
- 新增 CI 发布脚本 `release_ci.py`，将 GitHub Actions Release workflow 中的构建与发布逻辑迁移至 Python，build 与 publish 职责分离
- 新增配置常量自动生成脚本 `generate_config_constants.py`，从 `resources/develop/config-keys` 生成 Rust 和 TypeScript 常量
- 新增帮助窗口系统：
  - 新增 `app_window.rs` 和 `help_window.rs` 窗口管理模块
  - 新增 `cmd/windows.rs` 和 `cmd/help.rs`，暴露窗口管理与文档加载 Tauri 命令
  - 新增 `help.html` 入口页面及 `Help.svelte` 组件
  - 新增 `MarkdownView.svelte` 通用 Markdown 渲染组件，集成 markdown-it、highlight.js、DOMPurify
  - 新增 `NavTree.svelte` 通用树形导航组件，支持递归展开、外部跳转自动折叠
  - 新增 `helpStore` 状态管理，基于 Svelte 5 `$state` 实现文档内容加载与缓存
  - 新增 `buildNavTree` 工具函数，从 meta.json 构建有序导航树
  - 新增 `resources/help/meta.json` 导航配置，覆盖应用全部功能模块
- 新增帮助文档体系（`resources/help/docs/zh-CN/`），包含概览、快速开始、单次点名、自动结束点名、手动暂停点名、添加学生、批量导入、编辑学生、删除学生、导出学生、编辑记录、导出记录共 12 篇中文帮助文档
- 新增帮助窗口底部信息栏，展示应用版本、分支、提交哈希、构建时间
- 新增 `TreeNode`、`NavItem` 类型定义
- 新增 `WindowsCommand`、`HelpCommand` 前端命令调用模块
- 新增 `AppInfo` 懒加载单例（`LazyLock`），`app_info()` 返回 `&'static AppInfo`
- 新增 `root_dir()` 公共函数及 Tauri 命令
- 新增版本校验 job，CI 发布时阻止 alpha/beta 版本发布（仅允许 rc 及以上）
- 新增草稿 Release 支持，便于正式发布前验证构建产物

### Changed

- 重构 Release workflow，将构建和发布拆分为独立 job，支持多架构并行构建与统一发布
- 引入 uv 管理 Python 环境，替代系统 Python
- 重构版本预发布等级评估机制，`version.validate` 的 `strict` 参数改为 `min_level`
- 重构 `update_version.py`，复用 `common.version` 模块校验版本号
- 重命名 `config-keys` 为 `config.key`，`config.rs` 为 `app_config_keys.rs`，`config.ts` 为 `AppConfigKeys.ts`
- 重构 `AppInfo` 从 `entity` 模块移至 `config` 模块，改为 `LazyLock` 懒加载单例
- 重构 `build.rs`，移除内联配置解析逻辑，改为调用 `generate_config_constants.py`
- 重构应用入口：`index.html` 重命名为 `app.html`，`main.ts` 重命名为 `app.ts`，`app.d.ts` 重命名为 `types.d.ts`，`main_window.rs` 重命名为 `app_window.rs`
- 重构文档加载路径统一使用 `root_dir()`
- 优化帮助文档 Markdown 渲染效果：支持文档内链接跳转、外部链接通过系统浏览器打开、长代码块自动换行
- 优化 NavTree 导航树样式：箭头图标缩小、第一层级文字加粗、激活态改为透明背景加粗
- 优化构建脚本：构建前自动同步版本号并校验工作区干净状态、构建后还原版本文件
- 优化 `git.are_clean` 函数返回类型，新增 git status 原始输出

### Fixed

- 修复版本号占位符不一致问题，统一使用 `0.1.0-dev`
- 修复 CI workflow 中 `INPUT_VERSION` 环境变量传递缺失、语法错误（等号改冒号）、拼写错误（INOUT → INPUT）
- 修复 CI Python IO 流编码问题，设置默认 UTF-8 编码
- 修复 arm64 构建缺少 msvc-dev-cmd 环境配置的问题
- 修复 `help_load_readme` 中 README 内 en-US 跳转链接导致渲染错误的问题
- 修复帮助文档文件名格式：下划线改为连字符（`_zh-CN` → `-zh-CN`）
- 修复 `helpStore` 中 `SPECIAL_LOADERS` 键名与 meta.json 节点 ID 不匹配的问题
- 修复 MarkdownView 特殊文档链接匹配规则过于严格的问题，支持任意路径引用
- 修复 `AppInfo` 引用无法直接解引用的问题
- 修复生成的配置常量文件名错误的问题

### Removed

- 移除旧版构建脚本 `release.py`
- 移除 `build.rs` 中内联的配置解析逻辑

---

## 0.3.0

### Breaking Changes

- 迁移出勤状态从硬编码常量至数据库存储，`AttendanceStatusBadge` 组件 `code` 属性重命名为 `id`，前端调用需同步更新
- 移除 `constants/AttendanceStatus.ts`，改由 `types/AttendanceStatus.ts` 导出
- 扩展 `Result` 枚举：新增 `Info`，重命名 `Warn` 为 `Warning`，影响消息处理逻辑

### Added

- 新增表格排序功能，学生管理、点名、历史记录三个页面均支持单击表头排序
- 新增 `ArrowsDownUpIcon` 图标标识可排序列
- 新增点名后表格自动滚动到底部功能
- 新增"允许重复点名"开关，关闭后全部点完时自动清空列表
- 新增三步导入向导组件：选择文件 → 配置列映射 → 导入与冲突处理
- 新增 `AttendanceStatusDefinition` 数据库表及历史数据迁移
- 新增出勤状态实体类、Repository、Service、Command 全套后端实现

- 新增三个前端出勤状态接口：创建、更新、获取全部
- 新增 `AttendanceStatusStore.svelte.ts` 全局状态管理
- 新增 `Switch` 通用开关组件
- 新增 `card-group` 类支持横向分散排列
- 新增 `budge-button` 类替代 `budge` 类包裹按钮
- 新增版本号统一同步脚本 `sync-version.py`

### Changed

- 重构 `EditRecord` 组件使用 store 获取状态列表
- 重构出勤状态相关组件统一使用 store 获取数据
- 重构历史记录页面过滤逻辑
- 重构点名页面数组操作，由 `filter` 改为 `[...records]` 避免修改原数组
- 重构点名页面默认排序实现，不再通过 `reverse` 翻转
- 重构导入组件布局：外部遮罩不可点击、移除关闭按钮、调整标题样式
- 提取排序 key 为共享常量
- 提取 `switch` 样式到共享 CSS 文件
- 改为仅按点名时间排序时显示分组色块
- 改为从 `VERSION` 环境变量读取版本号，缺省使用 `CARGO_PKG_VERSION`
- 新增应用信息实体类 `version` 字段，初始化时自动从环境变量获取
- 添加 ESLint 及 Svelte 相关开发依赖，初始化 ESLint 配置

### Fixed

- 修复 CI 分支名显示为 "HEAD" 及提交数为 1 的问题
- 修复表头文字可被选中的问题
- 修复点名器在动态学生列表下获取空 ID 导致后端 panic 的问题
- 修复 `EditRecord` 未选择更新项时"确定"按钮仍启用的问题
- 修复点到学生状态显示为"缺勤"的问题（状态值统一 +1）
- 修复非时间排序时仍然自动滚动到底部的问题
- 修复 `type` 类型的异常导入
- 修复应用信息 `commit_time` 与 `build_time` 分隔符

### Removed

- 移除旧版导入组件，统一使用新导入向导入口

---

## 0.2.0

### Added

- 新增 `clickOutside` 动作，支持点击弹窗外部关闭，可配置排除元素
- 新增 `updatePosition` 通用定位函数，支持视口边界检测与自适应
- 新增 `Result` 枚举类型（Doing/Success/Warn/Error/None）
- 新增导出文件名自动添加时间戳避免覆盖
- 新增导出状态反馈（进行中/成功/失败）
- 新增记录导出全部、筛选后、选中三种模式，集成 Tauri 文件保存对话框
- 新增应用启动窗口自动居中
- 新增 `text.css` 文本样式组件，支持标题/副标题/内容及状态颜色变体
- 新增 `card.css` 卡片组件样式及背景色变量 `--color-card`

### Changed

- 重命名 `dialog.css` 为 `popup.css`，调整间距与最小宽度
- 迁移边框颜色变量层级（`--gray-3` → `--gray-10` 映射调整）
- 更新表单元素样式：复选框/单选框主色调、输入框大圆角、焦点高亮、禁用状态光标
- 添加按钮、输入框占位符、工具栏等 `user-select: none` 防止误选
- 为 `AttendanceStatusBadge` 组件新增 `selected` 属性
- 重构点名页面工具栏结构，统一按钮样式类
- 统一使用 `anyhow` 处理错误上下文
- 替换后端 `println!` 为 `info!`/`debug!` 宏
- 移除后端不必要的返回值处理（`student_batch_delete`、`student_export`）
- 移除前端多处调试日志及冗余点击事件

### Fixed

- 修复点名结果展示区域第一次点名时不滚动的问题
- 修复记录历史页面刷新按钮因 `this` 上下文丢失导致无法访问私有字段的问题
- 修复导出文件时 Windows 系统下确认覆盖已有文件失败的问题
- 修复图标按钮禁用状态下仍显示激活背景色的问题
- 修复后端服务层考勤状态码映射错误

---

## 0.1.0

基于 0.1.0-rc.1 的首个正式发布版本，包含全部新增功能、改进和修复。以下为 rc.1 之后的增量变更。

### Added

- 新增完整 CSS 变量设计系统，涵盖颜色、间距、字体、圆角、阴影、过渡等设计令牌
- 新增布局组件：侧边栏、导航栏、工具栏、页脚、标题栏
- 新增表单组件：输入框、选择器、文本域及全局重置样式
- 新增表格组件，支持固定表头、行悬停、自定义滚动条
- 新增反馈组件：弹窗（Dialog/Popup）、覆盖层（Overlay）、状态徽章（Badge）
- 新增按钮体系：图标按钮、文字按钮、按钮组，支持 yes/warn/error 类型
- 新增搜索组件，支持焦点状态交互
- 新增点名引擎，动画间隔 120ms，持续时间 720ms，结果展示 1200ms
- 新增构建时自动注入 Git 分支、提交次数、提交哈希、构建时间到前端
- 新增变更日志生成脚本，自动输出 Markdown 格式提交记录

### Changed

- 统一 CSS 类名命名规范，提升语义化
- 切换默认中文字体为 "Microsoft Yahei"
- 调整表格结果面板高度比例为 20%
- 调整按钮组布局从 `inline-flex` 改为 `flex`，支持右对齐
- 调整图标按钮使用 `aspect-ratio: 1` 保持正方形比例
- 移除全局样式中的冗余代码，将样式拆分为独立模块文件
- 移除内联样式，统一使用 CSS 类
- 移除废弃的 overlay、dialog、field 相关样式规则
- 重构模块导入顺序，按字母排列提升可读性

### Fixed

- 修复点名页面无记录时布局跳动问题
- 修复学生编辑功能中更新数据不一致的问题（使用数据库返回的最新数据）
- 修复历史记录页面搜索图标大小与其他页面不一致的问题
- 修复禁用状态颜色变量拼写错误（`diasbled` → `disabled`）
- 修复导入学生组件中 ImportCommand 路径错误
- 修复应用信息命令中调试宏调用错误

---

## 0.1.0-rc.1

首个候选发布版本。所有变更已并入 [0.1.0](#010)，以下为架构与核心实现概要。

### Added

- 新增点名状态机（空闲 → 动画 → 选人 → 展示），支持单次与连续点名模式
- 新增学生 CRUD 操作，支持软删除、恢复、按学号/姓名搜索、批量删除/恢复
- 新增考勤记录批量编辑，支持修改考勤状态（缺勤/出勤/迟到/早退/请假）和备注
- 新增 Excel 导入功能，支持列映射配置、表头行数设置、重复数据冲突检测（覆盖/跳过/保留）
- 新增 Excel 导出功能，支持全部或选中学生导出
- 新增路径管理模块，支持安装模式与便携模式（通过 `portable.mode` 文件检测），自动适配数据/配置/缓存/日志目录
- 新增 SQLite 数据库初始化及版本化迁移机制，迁移文件自动校验哈希
- 新增 Svelte 5 前端框架及 TypeScript 支持，侧边栏导航三页面架构
- 新增统一弹窗管理系统及 Phosphor 图标库、Open Props 设计令牌
- 新增 Rust 后端分层架构（cmd/service/repo）
- 新增前端路径别名（`$components`、`$stores`、`$utils` 等）
- 新增跨平台开发启动脚本（Windows、macOS/Linux）
- 新增单元实例运行检测

### Changed

- 统一使用 `anyhow` 处理错误上下文
- 实现数据库连接池懒加载单例，支持开发/生产环境独立数据库文件
- 实现日志系统控制台、文件双输出
- 重构前端状态管理为 Store 类（`StudentStore`、`RecordStore`）
- 封装命令调用层（`StudentCommand`、`RecordCommand`、`ImportCommand`、`RollcallCommand`）
- 统一组件样式至 CSS 变量驱动的设计系统，移除硬编码
- 合并冗余批量更新接口（状态+备注合并为单一批量更新）
- 优化事务处理确保数据一致性
- 增强用户界面错误提示信息

### Fixed

- 修复学生更新时学号冲突检测逻辑，允许更新自身学号
- 修复记录创建后 ID 获取失败的问题
- 修正学生批量删除和恢复 SQL 条件中的字段名错误（`ids` → `id`）
- 修复事务提交后继续查询导致的错误（调整事务内操作顺序）
- 修复 Windows 平台便携模式和安装模式下资源路径计算异常
- 修复前端学生编辑组件中多选状态下的空值处理及数据绑定问题
- 修复导入组件中 `Map` 类型操作错误（使用 `get/set/delete` 替代索引访问）
- 修复历史记录页面自动重复加载数据的问题
- 修复学生创建对话框遮罩点击意外关闭的问题
- 修复开发环境下 JSON 日志配置导致的构建失败
