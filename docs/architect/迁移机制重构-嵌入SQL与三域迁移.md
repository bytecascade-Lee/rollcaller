# 迁移机制重构：SQL 迁移编译进二进制

> 状态：**SQL 部分已实现**（2026-10-25）；fs / config 两域未开始，设计留档于 §7。
> 代码：`backend/src/database/migration.rs` + `backend/src/database/migration/{ledger,migrate,snapshot,sql}.rs`。

## 1. 结论速览

| 项目 | 重构前 | 重构后 |
|---|---|---|
| 迁移脚本 | 运行期扫 `resources/database/migrations`，读工作区字节 | `include_str!` 编译进二进制，运行期零 IO |
| 账本表 | `migration_history`（10 列 + 触发器），存 SQL 全文与 checksum | `database_migration_history(version, migrated_at)` |
| 防误改 | 运行期比对 checksum | 编译期金标哈希测试 |
| 事务 | 全部迁移一个事务，但 `PRAGMA foreign_keys` 空转 | 整批一个事务 + `defer_foreign_keys`（事务内有效） |
| 行尾 | 迁移文件必须 CRLF，否则校验和变化 | 载入时规范化，仓库统一 LF |
| V10 | 改账本表 `description` | 删除（账本没有该列了） |
| `checksums.json` / `checksum.py` | 运行期校验 | 退休（无消费者） |

## 2. 旧实现里的三个坑

### 2.1 账本表是"被迁移对象"，不是普通表

`V10__update-v1-description.sql` 的全部内容是：

```sql
UPDATE migration_history SET description = 'initialize schema' WHERE version == 0;
```

一条迁移直接改账本表自己的列。这就是"这张表没法迁移自己"的根源：**迁移由账本驱动，账本的形状必须在迁移引擎启动之前就是对的**。

### 2.2 V30 不可重入：账本丢了 = 用户数据损坏

`V30` 第一条有效语句是 `UPDATE records SET attendance_status = attendance_status + 1;`，随后重建表加外键。
它只能执行一次，重跑会把所有记录状态再 +1，并且 `CREATE TABLE records_new` 还会因表已存在而失败。

所以任何"账本读不出来就重建空表"的兜底都是**错的**。新实现的选择是 **fail closed**：认不出的账本直接拒绝启动，把决定权交给人。

### 2.3 `PRAGMA foreign_keys` 在事务内是空操作

V30 开头有 `PRAGMA foreign_keys = OFF;`、结尾有 `= ON;`。SQLite 文档明确：该 pragma **在事务内不生效**。
旧实现把所有迁移包在一个事务里 ⇒ 这两行一直在空转，表重建实际是在外键开启状态下完成的（能跑通是因为当时数据恰好没有违例）。

新实现不改 V30 的字节（已发布迁移不允许改），而是在事务内改用 **`PRAGMA defer_foreign_keys = ON`**——它专为"事务内暂时违反外键、提交时统一校验"设计，事务结束自动复位。

## 3. SQL 编译进二进制

```rust
pub(crate) struct SqlMigration {
    pub version: i64,
    pub name: &'static str,     // 仅日志与审计，不进库
    pub sql: &'static str,      // include_str! 原文，执行前过 normalize
}

pub(crate) const MIGRATIONS: &[SqlMigration] = &[
    SqlMigration { version: 0,  name: "initialize schema",
                   sql: include_str!("../../../../resources/database/migrations/V0__initialize-schema.sql") },
    SqlMigration { version: 20, name: "create attendance_status_definition table", sql: include_str!(…) },
    SqlMigration { version: 30, name: "add foreign key to attendance_status",      sql: include_str!(…) },
];
```

约定：

- **显式有序常量表，不扫目录**（与 `codegen::GENERATORS` 同一条约定）。
- 版本号保留 0 / 20 / 30 与 10 的步长：存量库里记的就是这些数，不能重排；新增继续 40、50。
- `include_str!` 要求 UTF-8；rustc 会把被包含文件记进 dep-info，改 SQL 即触发重编。
- `resources/database/` 不再进 Tauri 资源包（`tauri.conf.json5` 已删除该条目）。

### 3.1 载入时规范化

```rust
pub(crate) fn normalize(raw: &str) -> String {
    let body = raw.strip_prefix('\u{feff}').unwrap_or(raw);   // BOM：include_str! 会把它带进首行
    body.replace("\r\n", "\n").trim_end_matches('\n').to_owned()
}
```

只动"无意义的字符"：换行符与文件末尾空行。**不逐行 trim 尾随空格**——那可能改到 SQL 字符串字面量。
于是：`.gitattributes` 的 `eol=crlf` 例外被删除，迁移脚本转 LF，全仓一条规则；谁哪天写回 CRLF 也会在载入时被抹平。

### 3.2 金标哈希（checksum 的替代）

`sql::tests::published_migrations_are_frozen` 断言每条归一化 SQL 的 sha256。改历史迁移 → 测试红 → 必须显式改期望值。
把"防止误改已发布迁移"从运行期搬到 CI，而且与行尾彻底解耦。

## 4. 账本表怎么改（详解）

### 4.1 为什么不能用一条 SQL 迁移改它

迁移的执行顺序是：**先读账本 → 算出待执行集合 → 再执行 SQL**。
如果"改账本形状"本身是一条迁移，那么执行它之前，引擎必须已经能读旧账本；执行之后又要按新形状写账本——同一个事务里两代结构并存，代码得同时理解两种形状。
结论：账本的演进不属于"迁移"，属于**引擎自身的启动自检**。

### 4.2 新形状

```sql
CREATE TABLE IF NOT EXISTS database_migration_history (
    version    INTEGER PRIMARY KEY,   -- 对应 MIGRATIONS 里的 version
    migrated_at INTEGER NOT NULL       -- 毫秒时间戳，由 Rust 传入
);
```

被删掉的列与理由：

| 列 | 删除理由 |
|---|---|
| `content` | 与二进制里的常量重复；旧库里它存着整份 SQL，几十 KB |
| `checksum` | 原本校验"磁盘文件 vs 记录"；文件已编译进二进制，没有第二个副本可漂移 |
| `description` | 同样来自常量（`name` 字段），日志里打即可 |
| `status` | 改为"账本行与迁移 SQL 同事务提交"后不再需要三态，见 §5.2 |
| `id` / `executed_at` / `created_at` / `updated_at` / `remark` / 触发器 | 为普通业务表准备的，账本只需要"哪个版本、何时" |

### 4.3 `app_meta` 是什么

```sql
CREATE TABLE IF NOT EXISTS app_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

**用途**：放那些"想持久化、但不值得为它改表结构"的应用级元数据。它的形状**永远不需要迁移**（只加行不加列），这正是账本表要避免的问题。

当前只写一个键：`ledger_rebuilt_at`（账本从旧形状收敛的时刻）。
预留的典型用途：`last_boot_version`（上次运行的版本，便于支持排障）、`last_migration_error`、`first_install_at`。

硬规则：**账本表禁止加列，需要新信息一律进 `app_meta`**。

### 4.4 为什么改名 `database_migration_history`

因为账本以后不止一张：`fs_migration_history`（用户数据目录）、`config_migration_history`（配置项）都会落在同一个库里。
三个域各用独立编号空间，表名带域前缀才不会混。旧表叫 `migration_history`，语义上就是"数据库的"，趁这次重建顺手改掉——重建本来就要写一份新 DDL，改名零成本。

### 4.5 收敛算法（启动时，一次性）

入口是**三个纯函数**（都不需要数据库路径）：

| 函数 | 作用 |
|---|---|
| `ledger::inspect(rb) -> State` | 探测形状：`Missing` / `Ready` / `Legacy { rows }`，认不出就 `Err` |
| `ledger::create(rb)` | 全新库建表 |
| `ledger::rebuild(rb, rows)` | 旧表 → 新表，整表搬迁后删旧表 |

情形对照：

| 情形 | 判定 | 动作 |
|---|---|---|
| A 全新库 | 新旧表都不存在 | `CREATE TABLE`，打印"已创建迁移账本表" |
| B 存量库 | 旧表存在且含 `version` 列 | **重建 + 搬行 + 校验行数 + 删旧表**（事务内） |
| C 已收敛 | 新表存在且含 `version` / `migrated_at` | 直接返回（幂等） |
| D 认不出 | 新表存在但缺关键列；或旧表连 `version` 都没有 | **`Err`，拒绝启动**，绝不建空表 |

情形 B 的实际时序（**快照由 runner 在调 `rebuild` 之前打**，见 §5.1）：

```
1. before = SELECT COUNT(*) FROM migration_history              -- inspect 阶段读好，传给 rebuild
2. VACUUM INTO '<data>/<db>-bak-<ts>.db'                       -- runner 打快照（失败只 warn）
3. BEGIN
   CREATE TABLE database_migration_history (version INTEGER PRIMARY KEY, migrated_at INTEGER NOT NULL);
   INSERT INTO database_migration_history (version, migrated_at)
       SELECT version, COALESCE(executed_at, created_at, 0) FROM migration_history;
   after = rows_affected                                        -- 搬迁条数，不额外 COUNT
   if after != before → ROLLBACK + 报错                          -- 行数对不上就放弃
   DROP TABLE migration_history                                  -- 触发器随表删除
   COMMIT
4. INSERT INTO app_meta(key, value) VALUES ('ledger_rebuilt_at', <ts>)
```

两个细节：

- **只搬 `version` 与最近时间戳，不搬 `content`**：旧库里 content 是整份 SQL，搬它既慢又没意义。
- **`FAILED` 行也要搬**：它代表"曾经成功应用过、后来因 checksum 冲突被标脏"，schema 其实已经在了；丢了它会导致 V30 重跑 → 数据损坏。
  （`RUNNING_IN_TX` 行不需要考虑：它写在事务内，进程崩溃时随事务回滚，永远不会落盘。）

### 4.6 这段兼容代码会不会越堆越多

只有一段，且不随版本增长：

1. 只认**当前生产环境的那一种旧形状**（含 `version` 的 9 列表），不做通用 shape 迁移框架；
2. 收敛一次后走情形 C，这段代码不再执行；
3. 日落条件：`app_meta.ledger_rebuilt_at` 有值即说明该机已收敛；全部用户升过一个大版本后，把 `rebuild` 连同情形 B 分支一起删掉（判据：新账本随首个版本发布满 6 个月 / 下个大版本）；
4. 账本表冻结（见 §4.3），所以不会每版都加一段。

⚠️ **不能提前删**：9 列的 `migration_history` 从 `v0.1.0` 起就随每个正式版本发出去过（`git tag --contains` 可证），删掉分支等于让所有存量用户的库启动即报错或丢账本。

## 5. 执行语义

### 5.1 整批一个事务

```
BEGIN
  PRAGMA defer_foreign_keys = ON          -- 事务内唯一的正确做法
  for m in pending: exec(normalize(m.sql))
  SELECT COUNT(*) FROM pragma_foreign_key_check → 必须为 0
  for m in pending: INSERT INTO database_migration_history(version, migrated_at)
COMMIT
```

为什么不是每条一个事务：桌面应用要考虑**降级**。
整批失败 ⇒ 回滚 ⇒ 数据库回到升级前的字节状态 ⇒ 用户装回旧版仍可用。
每条一提交会留下"中间 schema"：新版跑不完、旧版不认识，只能删库。

**快照策略统一由 runner 负责**（`migration::run` → `snapshot::database`），账本模块不再自己备份：

| 场景 | 是否快照 |
|---|---|
| 旧账本要收敛（要 `DROP TABLE`） | ✅ 收敛**之前**打一份 |
| 已有数据的库有待执行迁移 | ✅ 执行前打一份 |
| 全新库首次迁移 | ❌ 空库没有备份价值 |
| 无待执行迁移 | ❌ 直接把启动开销压到零 |

同一个启动周期最多一份（账本要收敛时先把快照打掉，避免"收敛 + 迁移"打两份）。
快照用 `VACUUM INTO`（连带 WAL 中未回写的页），失败只 `warn` 不阻断；保留策略按构建模式分档——**发布版**只留最近 3 份，避免每次升级在数据目录里堆一份整库副本；**开发模式**一份都不删，调试期反复迁移时被清掉的往往正是要对照的那份。

### 5.2 "没有 status 列，怎么知道成功"

**行存在 = 成功**。账本行与迁移 SQL 在同一个事务里提交，要么两者都有，要么都没有。
旧实现的 `RUNNING_IN_TX → SUCCESS` 三态是想把中间态持久化，但 SQLite 事务已经保证中间态不落盘；它的 `FAILED` 分支只服务 checksum 冲突，而这个场景已随 `checksum` 列一起消失。

失败不写库：回滚本身抹掉了痕迹，错误进日志与启动失败对话框。想留持久痕迹的话，用 `app_meta.last_migration_error`（当前未实现，需要时加一行）。

### 5.3 迁移集合的可见性

- 账本里有、常量表里没有的版本（如删掉的 V10）→ 忽略，不报错。
- 常量表里有、账本里没有的版本 → 待执行。
- 版本严格递增由测试保证（`versions_are_strictly_increasing`）。

## 6. 代码地图

| 文件 | 职责 |
|---|---|
| `database/migration.rs` | `run(rb, db_path)`：建 `app_meta` → 账本收敛 → 读账本 → 快照 → 单事务执行 + 记账 |
| `database/migration/ledger.rs` | `inspect` / `create` / `rebuild`：账本形状收敛（不碰路径、不打快照），`rebuild` 统一处理旧表搬迁与时间戳改写两种来源 |
| `database/migration/snapshot.rs` | `database()`：`VACUUM INTO` 快照 + 按构建模式清理（发布版留 3 份、开发模式全留） |
| `database/migration/sql.rs` | `MIGRATIONS` 常量、`normalize`、金标哈希与版本序测试 |
| `database/migration/migrate.rs` | 账本与 `app_meta` 的 SQL（含 `pragma_table_info` / `pragma_foreign_key_check` 的表值函数形式） |
| `database/database_pool.rs` | `db_path()`：debug 用 `DEVELOP_DATABASE_FILENAME`、release 用 `sqlite.db`，供迁移备份复用 |
| `database/database_bootstrap.rs` | 去掉 `resources/database/migrations` 路径，改调 `migration::run` |
| `tauri.conf.json5` | 移除 `"../resources/database/"` 打包项 |

两条实现约束（改之前先看这里）：

- **`select_applied` 必须返回结构体**，不能简化成 `Vec<i64>`：rbatis 的 `decode` 把每行解成 `{列名: 值}` 的 map，`Vec<i64>` 会解码失败。
- **`PRAGMA foreign_keys` 不要手动开**：`rbdc-sqlite` 的默认 pragmas 已含 `foreign_keys=ON`，每条新连接都会执行；而在 `rb.link()` 之后 `rb.exec` 只作用于池里的一条连接，反而制造"有的连接开、有的没开"。

测试（`cargo test --lib -- database::migration`，11 个）：

- 版本严格递增 / 金标哈希 / normalize 幂等与边界（BOM、CRLF、末尾空行、行尾空格保留）
- 全新库：建账本 → 跑完 V0/V20/V30 → 账本为 `[0,20,30]` → 再跑不重复 → 不留快照
- 旧库：9 列表 + 3 行（含 FAILED）→ 收敛 → 版本集合与时间戳保留、旧表消失、幂等
- 旧库收敛只产生一份快照；快照保留策略随构建模式（发布版 3 份、开发模式全留）
- 认不出的账本表 → 拒绝启动
- 外键默认开启（驱动默认 pragma 的回归守卫）

## 7. fs / config（未实现，留给后续）

原则：**统一流程契约，不统一存储**。

```rust
trait Step { fn id(&self) -> StepId; fn apply(&self, ctx: &Ctx) -> Result<Outcome>; }  // 幂等
```

| | SQL（已实现） | fs | config |
|---|---|---|---|
| 状态 | `database_migration_history` | `fs_migration_history` | `config_migration_history` |
| 原子性 | 事务 | 无 → 靠可重入 + 全量预检 | 单文件原子写 |
| 单元 | 一段 SQL | 动作词表 `EnsureDir` / `MoveFile` / `MoveDir` / `Quarantine` / `RunOnce` | 扁平 map 变换 |
| 失败 | 整批回滚 | 停在失败步骤 | 停在上一步，文件保持上一版 |
| 备份 | 形状变更时整库快照 | 受影响路径 → `.backup/<version>/` | `config.yaml.bak.<from>` |

fs 的硬约束：只能操作 `data/`、`config/` 下的子条目，**不得移动数据库文件本身**——移动库属于"路径重定位"，不是迁移。
config 的特殊性：用户配置是**相对默认值的稀疏 diff**（`filter_user_props`），迁移在扁平 key 空间做；版本号不能写进 `config.yaml`（与默认相同会被删掉）。

启动顺序（三域齐备后）：打开主库 → 账本收敛 → fs → config → sql。

## 8. 后续待办

1. 首个版本发布后观察 `app_meta.ledger_rebuilt_at`，满 6 个月 / 下个大版本删除 `ledger::rebuild` 与 `Kind::Legacy` 分支（§4.6）。
2. `last_migration_error` 是否要落库（当前只进日志）。
3. fs / config 两域的实现与表结构。
4. `checksums.json` 已删除；若还想留一份历史哈希，放到 `docs/` 而不是仓库根。
