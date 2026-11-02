# 深度链接：`rollcaller://` 协议接入构想

状态：构想。前置文档：`docs/architect/导航架构-地址驱动的全局跳转体系.md`。

## 1 结论

`rollcaller://` 是同一套地址体系在操作系统上的第二个入口。 **不需要新的导航机制**：把 URL 翻译成 `Action`，走既有的 `nav_open` 通路即可。

```
rollcaller://app/rollcall      →  #/app/rollcall
rollcaller://help/tts/modes    →  #/help/tts/modes
```

三条不变量：

1. **URI = 外部进入，event = 进程内广播**，两者不共用命名空间。
2. **深链只表达「去哪里」，不表达「做什么」**：不接受载荷，不触发流程。
3. 解析与投递全在 Rust；前端零改动（`nav.start(initial)` 与 `navTakePending()` 已覆盖）。

## 2 地址映射

| 输入                                               | route                                   | 处理                                     |
|----------------------------------------------------|-----------------------------------------|------------------------------------------|
| `rollcaller://app/rollcall`                        | `#/app/rollcall`                        | 接受                                     |
| `rollcaller://app/student-management/edit-student` | `#/app/student-management/edit-student` | 接受，段数不限                           |
| `rollcaller://help/tts/modes`                      | `#/help/tts/modes`                      | 接受，doc + anchor                       |
| `rollcaller://app`                                 | `#/app`                                 | host 合法、段为空，由 app 域拒绝（warn） |
| `rollcaller://help/tts?q=x`                        | `#/help/tts`                            | query 与 fragment 丢弃（v1）             |
| `rollcaller://update/view`                         | —                                       | host 不在闭集，丢弃 + warn               |
| `rollcaller:app/rollcall`                          | —                                       | 无 authority 形式不支持                  |

Rust 侧只做三件事：验 scheme；host 归一化为小写后查 `WindowKind::from_host`；`path` 去掉首尾斜杠后拼成 `#/<host>/<path>`。
**段不校验、不做百分号解码**——合法性归域解码器（`resolveAppChain` 严格逐段比对，`HelpAddress.fromSegments` 只取前两段）。这与 `host_of` 现有的「后端对地址的理解止于第一段」一致。

不做解码的理由：合法段只有 `[a-z0-9-]`，任何编码形式都进不了域解码器；不解码反而让 `%2e%2e` 保持字面量，不构成路径穿越。

## 3 三条链路

| 场景               | 通路                                                                      |
|--------------------|---------------------------------------------------------------------------|
| 应用未运行，点链接 | OS 用 argv 启动新进程 → `setup` 里取当前 URL → `deliver`                  |
| 应用已运行，点链接 | OS 启动第二实例 → single-instance 拦下 → 回调里从 argv 取 URL → `deliver` |
| 应用内跳转         | 现有 `nav.go` / 门面，不经深链                                            |

三条最终都落到同一处：`deliver(route, payload)` = 建窗 + `NavState::put` + 唤醒事件。`nav_open` 命令已经是这个动作，深链只是换一个调用者。

`NavState` 是每窗口单槽 last-wins，因此连续点击多条链接只保留最后一条；窗口销毁时槽位由 `on_window_event` 清掉，行为已就绪。

## 4 冷启动的窗口策略

- 主窗口是应用身份，`init` 无条件创建，深链不改变这一点。
- 目标是 app：pending 在 `app_window::open` 之前落槽，主窗口挂载时 `navTakePending()` 取到，`nav.start` 的 `initial` 优先于 hash 与 fallback。
- 目标是 help：`open_by_host(help)` 建窗并落 pending，帮助窗口首帧即深链地址；主窗口仍落默认页。 **结果是两个窗口**，与「应用内点帮助按钮」一致，接受。
- 备选（不做）：目标为 help 时不建主窗口。要绕开 `init` 的无条件建窗，收益低于代价。

## 5 改动清单

Rust，按落盘顺序：

1. `Cargo.toml`：`tauri-plugin-single-instance` 加 `features = ["deep-link"]`；新增 `tauri-plugin-deep-link`（要求 rustc ≥ 1.90，本机 1.96）。
2. `tauri.conf.json5`：`plugins.deep-link.desktop.schemes = ["rollcaller"]`。Windows 上打包时由安装器写注册表；debug 下用 `register_all()` 指向当前 exe。
3. `lib.rs`：插件顺序改为 **single-instance
   最先**（deep-link 文档的硬要求），空回调换成 `|app, argv, _cwd| deep_link::from_args(app, argv)`；`setup` 里调 `get_current()` 与 `on_open_url(...)`；debug 分支 `register_all()`。
4. `cmd/nav.rs`：把「建窗 + put + 唤醒」抽成 `pub fn deliver(app, route, payload)`，`nav_open` 退化为解析 + 调它。
5. 新 `cmd/deep_link.rs`：纯函数 `parse(&str) -> Option<Route>`（`#[cfg(test)]` 表驱动）、`from_args`、`handle`。
6. `cmd/help.rs::help_load_markdown`：加 kebab 白名单，见 §7。

前端： **无改动**。不引入 `@tauri-apps/plugin-deep-link`，不改 `capabilities/*.json`——解析在 Rust，JS 侧不需要权限。

不交给前端的理由：投递前必须知道 host 才能决定建哪个窗，而那时目标窗口的前端还不存在。

## 6 测试矩阵

| 场景          | 操作                                               | 预期                                 |
|---------------|----------------------------------------------------|--------------------------------------|
| 冷启动到 app  | 退出应用后 `start rollcaller://app/record-history` | 主窗口首帧为记录页                   |
| 冷启动到 help | 同上，`rollcaller://help/tts/modes`                | 帮助窗口落在该锚点，主窗口落默认页   |
| 热路径        | 应用运行时执行同样命令                             | 目标窗口被唤到前台并落地，不新建实例 |
| 未知 host     | `start rollcaller://update/view`                   | 无导航，日志 warn                    |
| 非法段        | `start rollcaller://app/nope`                      | 无导航，日志 warn                    |
| 连点三次      | 依次三条不同地址                                   | 只有最后一条落地（单槽 last-wins）   |

单测覆盖 `parse`：合法 / 尾斜杠 / 大写 host / 无 authority / 未知 host / 带 query / 超长。

## 7 安全

1. **威胁模型**：任何网页都能触发 `rollcaller://`，Windows 无二次确认。攻击面止于「唤起窗口」与「导航到某个入口」——都是视图，不改数据。
2. **能力边界写进不变量**：深链不触发流程（不提供 `rollcaller://action/start-rollcall` 一类），不接受载荷。否则一次误点即可开始点名。
3. **校验清单**（Rust）：scheme 精确匹配（Tauri 按配置过滤过 argv，仍要自检，官方文档亦如此要求）；host 闭集；总长 ≤ 512；段数 ≤ 8；拒绝含 `..`、`\`、空白的段。
4. **`doc` 段从此是外部输入**。`help_load_markdown(id)` 现在是 `resources_dir().join(format!("help/{0}/{0}-zh-CN.md", id))`，
   **没有白名单**；今天 `id` 只来自生成句柄与 md 内链正则（`[^/]+`，允许 `..`），深链之后任何网页都能提供它。修法：`help_load_markdown` 里加 `^[a-z0-9-]+$`（纵深防御，不依赖调用方），深链入口再加一次（治标）。两处都加。
5. **`csp: null` 放大后果**：越权读到的 md 会被当文档渲染，其中若含远程图片即产生外联请求。

## 8 命名冲突：`rollcaller://update/*`

`UPDATE_VIEW_EVENT = "rollcaller://update/view"`、`DOWNLOAD_PROGRESS_EVENT = "rollcaller://update/download"` 是 **Tauri
事件名**，不是 URI。注册 scheme 后它们成为语法合法的 OS URL：`start rollcaller://update/view` 会拉起应用，然后被 host 闭集拒掉。

不构成漏洞，但「事件名长得像外部入口」迟早误导人。建议（不阻塞）：内部事件改用非 URI 命名空间，如 `app:update/view`，Rust 与 TS 各一处常量。若保留现状，深链解析里也不要对 `update` 做任何特殊处理——它就该是「未知 host」。

## 9 不做的

- URL 携带载荷（`?q=`）：载荷是一次性数据，进 URL 就永久化。若要支持，须先给出「哪些入口允许从 query 构造载荷」的白名单，且只接受字符串。
- 深链触发流程（开始点名、安装更新）。
- 多实例并存：single-instance 已保证唯一实例。
- macOS / Linux：`scripts/common/support.py` 在册只有 windows-msvc。macOS 走 `RunEvent::Opened`，Linux 走 desktop 文件，随平台扩展再做。
- 深链跳转到其它应用：`rollcaller://` 只处理本应用。

## 10 落地顺序

1 → 2（配置，不改行为）→ 4（抽 `deliver`，不改行为）→ 5（解析 + 单测）→ 3（接线）→ 6（防御校验）→ 本文升为正式文档。
