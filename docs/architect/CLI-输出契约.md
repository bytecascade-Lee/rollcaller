# CLI 输出契约与参数解析

> 状态：**已落码**（分支 `feat/cli-contract`）。由 AI 起草并实现，人工审定。
> 日期：2026-10-27
> 关联：`backend/src/cli.rs`、`backend/src/cli/args.rs`（新增）、`backend/src/cli/doctor.rs`（新增）、`backend/src/main.rs`

---

## 1. 结论先说

1. **CLI 模式下没有日志，而且是结构上不可能有**：`handle_cli_args()` 在 `tauri::async_runtime::block_on(run())`
   之前就 `exit`；而 `logger::init()` 只在 `bootstrap::init()`（Tauri setup 之后）里。唯一的纪律是
   **不要让 CLI 命令去调 `bootstrap::*`**（将来加 `--db-info` 之类尤其注意）。
2. **输出契约**：stdout 只放数据、stderr 只放诊断；单值命令输出裸值、多字段命令输出对齐的 `key : value`；
   `--json` 输出**恰好一个** JSON 对象；退出码 `0 / 2 / 1`。
3. **解析从"扫到第一个能认的就执行"改成"先确认整条命令合法"**：命令互斥、未知 `-*` 退 2、`-h` 优先、
   非 `-` 开头的参数继续忽略（更新重启会转发原参数）。
4. **`--json` 的形状不随人类视图改变**：`--build-info` / `--app-paths` 的 JSON 与 IPC 同构
   （时间字段自 v0.8.4（不包含） 起为规范 ISO 8601 串，见 §3），所以现有脚本加一个 `--json` 就能继续用。
5. `--config` / `--db-info` / `--check-update` / `--open-logs` **本期不做**（见 §6）。

---

## 2. 为什么 CLI 模式一定没有日志

启动链路只有两条，互不交叉：

```
main()
├─ cli::handle_cli_args()       ← 命中命令：console::attach_for_cli() → 打印 + exit(0/2/1)，到此为止
├─ console::attach()            ← 未命中才走这里：只判定输出通道（受开关控制），不打日志
└─ block_on(run())              ← 未命中才走这里
   └─ Tauri setup → bootstrap::init() → logger::init() → 数据库迁移
```

CLI 命令的控制台通道由 `console::attach_for_cli()` **无条件**打通（不看
`ROLLCALLER_ATTACH_CONSOLE_ON_WIN32`）：GUI 的常规启动默认不接管终端，而 `-v` 这类命令
必然要写 stdout，若跟着开关走就会「release 下敲 `-v` 什么都没打印」。详见
`docs/architect/终端附着与控制台日志.md` §11。

`app_info()` / `app_paths()` 都是纯 `LazyLock`（读文件 / 算路径），不碰日志、不碰数据库。
只要新命令不调 `bootstrap::init()`，CLI 输出就永远是"裸 stdout"，不会被 tracing 掺进来。

顺带一个已实测的事实：控制台里的中文不会乱码——Rust std 对 console 句柄走 `WriteConsoleW`（UTF-16），
对管道/文件走 UTF-8 字节；`--json > a.json` 是 UTF-8 无 BOM，`jq` 直接吃。

---

## 3. 输出契约

| 维度       | 约定                                                                                       |
|:-----------|:-------------------------------------------------------------------------------------------|
| stdout     | **只放数据**（人类视图或 JSON），绝不放日志                                                |
| stderr     | **只放诊断**：用法错误、运行失败                                                           |
| 单值命令   | `-v` / `--mode` 输出裸值，便于 `V=$(rollcaller -v)`                                        |
| 多字段命令 | 对齐的 `key : value`，**键名与 `--json` 的键一一对应**（有测试锁）                         |
| JSON       | 恰好一个对象；`--build-info` / `--app-paths` 复用 Rust 类型的 `Serialize`，形状与 IPC 一致 |
| 时间       | 人类视图与 JSON 同为规范 ISO 8601 串（UTC、恒三位小数、`Z` 后缀）                          |
| 退出码     | `0` 成功 · `2` 用法错误 · `1` 运行失败（序列化等，实践中几乎不会发生）                     |

人类视图样例（debug 构建实测）：

```
$ rollcaller --build-info
version          : 0.1.0-dev
target           : x86_64-pc-windows-msvc
os               : Windows
arch             : X86_64
ref_name         : refactor/shutdown-teardown
commit_count     : 1896
...
commit_time      : 2026-10-27T01:34:23.000Z
build_time       : 2026-10-27T04:42:08.068Z

$ rollcaller --app-paths
mode          : Develop
root_dir      : D:\Rust\rollcaller
config_dir    : D:\Rust\rollcaller\data\config
...
```

`human_keys_match_json_keys` 这条单测把"人类视图的键 = JSON 的键"钉死：新增字段忘了改人类视图，
`cargo test` 直接红（`webview_dir` 这种名字对不上的情况就是它拦下来的）。

---

## 4. 解析规则（`cli/args.rs`）

| 输入 | 结果 |
|:---|:---|
| 无参数 / 只有位置参数 | `Ok(None)` ⇒ 正常启动图形界面 |
| `-v`、`--version`、`--build-info`、`--app-paths`、`--mode`、`--doctor` | 执行对应命令 |
| `--json`（位置任意、重复无害） | 该次输出改为 JSON |
| `-h` / `--help` | **优先**：立即显示帮助，忽略其它参数 |
| 两个命令同时出现 | `exit 2`（旧实现静默取第一个命中） |
| 未知 `-` 开头参数（含拼错） | `exit 2`（旧实现会**静默启动 GUI**） |
| 只有 `--json` | `exit 2` |
| 非 `-` 开头的参数 | **忽略**（见下） |

为什么位置参数必须忽略：更新安装时 `build_nsis_args()` 会把**当前进程的全部启动参数**
经 NSIS `/ARGS` 转发给重启后的新版本（`service/update/install/nsis.rs`）。对未知位置参数报错，
就会变成"更新完之后应用起不来"。而未知的 `-` 参数仍报错——用户真要带参数，第一次启动就会知道错了。

---

## 5. 命令一览

| 命令 | 输出 | 说明 |
|:---|:---|:---|
| `-h`, `--help` | 帮助文本 | 退出码 0 |
| `-v`, `--version` | 裸版本号 / `{"version"}` | 给脚本用 |
| `--build-info` | 键值 / `AppInfo` JSON | 版本、提交、构建时间、目标平台 |
| `--app-paths` | 键值 / `AppPaths` JSON | 运行形态 + 九个目录 |
| `--mode` | 裸形态 / `{"mode"}` | `Develop` / `Portable` / `Install` |
| `--doctor` | 键值 + 自检块 / 嵌套 JSON | 见下 |

`--doctor` 的自检项（**只读**：探针写完立刻删，不创建目录、不连库、不初始化日志）：

| 检查项 | 判定 |
|:---|:---|
| `data_dir_writable` | 目录存在且能写探针文件 |
| `logs_dir_writable` | 同上 |
| `database_file` | 存在报字节数；不存在则说明"首次启动时创建" |
| `webview2_dir` | WebView2 运行时目录是否存在 |

它的意义是"用户报障只要这一条输出"：版本 + 构建 + 目标平台 + 九个目录 + 可写性 + 数据库 + WebView2。

---

## 6. 本期不做（候选）

| 参数 | 为什么不做 |
|:---|:---|
| `--config` | 配置加载链路还没确认"零 Tauri 依赖"，先不做；**也不做输出 `{}` 的桩**——一个永远说"没有配置"的命令比没有更糟 |
| `--db-info` | 需要独立开库（不能走 `bootstrap`），schema 版本 / 迁移列表值得单独一轮设计 |
| `--check-update` | 要拉 `service::update` 与网络，还牵扯"结果怎么序列化"，属于更新域 |
| `--open-logs` | 价值中等（`--app-paths` 已经能拿到路径），且在 CLI 里拉 explorer 有点越界 |

---

## 7. 不要做

1. 不要在 CLI 分支里 `logger::init()` / 建 Tauri app / 跑迁移——**CLI 必须能在"应用起不来"时跑通**；
2. 不要把时间字段改成毫秒整数或其他形态：`--build-info` 的 JSON 与 IPC 同形，时间统一为
   规范 ISO 8601 串（`2026-10-27T01:34:23.158Z`，UTC + 恒三位小数）。这条曾在 v0.8.4（包含） 之前是
   「保持毫秒整数」，后按时间格式统一方案 **有意推翻**，属于破坏性变更；新代码不要再引入第二种形态；
3. 不要对**位置参数**报错（NSIS 转发原启动参数，会把更新后的应用拦住）；
4. 不要让人类视图与 `--json` 各写一份键名——`cli.rs` 里那两条单测就是为了防这个；
5. 不要把帮助文本塞进 stderr（`-h` 是正常输出，退出码 0）。
