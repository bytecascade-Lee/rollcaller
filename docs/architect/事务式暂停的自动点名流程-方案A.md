# 事务式暂停的自动点名流程（方案A）

> 本文档描述点名面板重构后的完整逻辑：以「事务式暂停」为核心的状态机设计、前后端接口契约与模块职责划分。

## 1. 设计目标

重构围绕三个核心诉求展开：

1. **拆分选人与写入**：`pick` 仅做随机选择（不写库），`create` 显式写入数据库。写入失败时前端可以撤销已展示的名字。
2. **事务式暂停**：用户点击「停止」不会立即中断流程，而是打上 `pendingStop` 标记；当前这一轮「选人 + 存库 +
   展示」事务必须完整走完，只在自然中断点（ShowDone）才真正停止。
3. **表格可信**：`recordStore` 只在写入成功后更新，杜绝「界面看到名字、数据库没有记录」的不一致。

## 2. 状态机设计

### 2.1 状态定义（`src/lib/types/RollcallPhase.ts`）

| 状态        | 含义                                      |
|-------------|-------------------------------------------|
| `Idle`      | 空闲，等待开始                            |
| `Animating` | 名字滚动动画中                            |
| `Picking`   | 随机选人 + 写入数据库（事务必须完整走完） |
| `Showing`   | 展示选中的学生                            |

### 2.2 事件定义（`src/lib/types/RollcallEvent.ts`）

`Start`、`AnimateDone`、`UserStop`、`PickDone`、`SaveSuccess`、`SaveFailed`、`ShowDone`

### 2.3 状态转换表

| 当前状态  | 事件        | 下一状态        | 动作                                                                                  |
|-----------|-------------|-----------------|---------------------------------------------------------------------------------------|
| Idle      | Start       | Animating       | 重置会话（新 `sessionId`、`completedTimes=0`、`pendingStop=false`），启动名字滚动动画 |
| Animating | AnimateDone | Picking         | 停止动画，调用 `pick` 获取随机学生                                                    |
| Animating | UserStop    | Picking（强制） | 同 AnimateDone，但设置 `pendingStop = true`                                           |
| Picking   | PickDone    | Showing         | 展示学生名字；调用 `create` 写入数据库                                                |
| Picking   | UserStop    | 忽略            | 选人 + 存库事务进行中，不做状态改变                                                   |
| Showing   | SaveSuccess | -               | `recordStore.upsert(record)`，`completedTimes+1`，启动 ShowDone 计时                  |
| Showing   | SaveFailed  | Idle            | 撤销当前展示的名字，`alert` 提示错误，重置状态                                        |
| Showing   | ShowDone    | 判断            | `pendingStop` 或 `completedTimes >= totalTimes` → Idle；否则 → Animating（下一轮）    |
| Showing   | UserStop    | -               | 仅设置 `pendingStop = true`，不影响当前展示计时                                       |

### 2.4 实现位置

- 状态机入口：`RollcallEngine.#dispatch(event)`，按当前 `phase` 分发事件。
- 异步事务：`#runPicking()`（fire-and-forget），完成 pick → create 后派发 `SaveSuccess` / `SaveFailed`。
- 所有对 `phase` 的修改都收敛在 `#dispatch` 中，页面只读状态。

## 3. 核心机制

### 3.1 事务式暂停（pendingStop）

- `UserStop` 永不直接回 Idle，只做两件事：置 `#pendingStop = true`；若处于 Animating，强制把动画推进到 Picking。
- `pendingStop` 只在 `ShowDone` 时被消费：若为真，展示结束后不再进入下一轮动画，直接 Idle。
- Picking 阶段收到的 `UserStop` 被忽略——事务（选人 + 存库）必须完整走完。

### 3.2 单次点名（totalTimes = 1）

- `#enterAnimating(false)`：不启动自动 `AnimateDone` 定时器，动画持续滚动， **等待用户点击停止**。
- 用户点击停止 = 手动触发 `AnimateDone`（`UserStop` 强制转入 Picking），随后完成选人、存库、展示、结束。

### 3.3 连续点名（totalTimes > 1）

- `#enterAnimating(true)`：动画滚动 `ANIM_DURATION`（1s）后自动派发 `AnimateDone`。
- 每轮 `ShowDone` 判断：未暂停且未完成全部次数 → 进入下一轮 Animating；否则 Idle。
- 用户可在任意一轮动画中提前停止（同单次，强制完成当前事务后结束）。

### 3.4 定时器管理

- `#animTimer`（80ms 名字滚动 interval）+ `#animTimeout`（自动 AnimateDone）。
- 离开 Animating 时统一 `#clearAnim()` 清理，无泄漏；展示计时 `#showTimeout` 在 ShowDone 后置空。
- 无需 `AbortController`：事务式暂停下不存在「丢弃已选结果」的场景。

## 4. 时序流程

### 4.1 单次点名（用户手动停止）

```
Start(Idle) → Animating（滚动） → UserStop → Picking（pendingStop=true）
→ pick 返回 studentId → create 写入 → Showing（展示名字）
→ SaveSuccess（upsert + completedTimes=1） → ShowDone → Idle
```

### 4.2 连续点名（自动完成 N 轮）

```
Start(Idle) → Animating(1s 自动) → Picking → Showing(1s)
→ ShowDone(未完成) → Animating → ... → 第 N 轮 Showing
→ ShowDone(completedTimes >= totalTimes) → Idle
```

### 4.3 中途停止（连续点名第 k 轮动画中点击停止）

```
Animating + UserStop（pendingStop=true，强制） → Picking → Showing
→ SaveSuccess → ShowDone → 发现 pendingStop → Idle
```

第 k 轮结果仍被完整随机、展示并写入数据库后，流程才彻底结束。

### 4.4 保存失败

```
Picking → pick 成功 → Showing（展示名字） → create 失败
→ SaveFailed → #undoShow（名字恢复"等待点名"、phase=Idle） → alert("点名保存失败：…")
```

## 5. 后端接口契约

| 命令                   | 签名                                                                              | 语义                                                          |
|------------------------|-----------------------------------------------------------------------------------|---------------------------------------------------------------|
| `pick`                 | `fn pick(ids: Vec<i64>) -> Result<i64, String>`                                   | 纯随机选择（含权重语义），返回选中学生 id，**不写入任何记录** |
| `record_single_create` | `async fn record_single_create(record: Record) -> Result<RollcallRecord, String>` | 写入展示记录，返回完整 `RollcallRecord`；失败抛出错误         |

前端封装（`src/lib/commands/`）：

- `RollcallCommand.pick(ids: bigint[])` → invoke `"pick"`，参数键 `ids`（与后端参数名一致）。
- `RecordCommand.create(record: Record)` → invoke `"record_single_create"`，参数键 `record`。

新增记录字段约定：

- `id: null`（由 SQLite 自增）
- `attendance_status: 1`（出勤）
- `remark: null`
- `rollcall_at: Date.now()`（毫秒时间戳，后端反序列化为 `jiff::Timestamp`）
- `session_id`：每轮点名会话的 uuid（8 位，来自 `UuidUtils.uuid()`）

### 5.1 自增 id 获取（重要陷阱）

RBatis `crud!` 生成的 `insert` 接收 **不可变引用**（`table: &Self`）， **不会把自增 id 回填到结构体**，新 id 只存在于返回的
`ExecResult.last_insert_id`。因此：

```rust
// ✓ 正确：从 ExecResult 取新 id
let result = Record::insert(rb, & record).await?;
let id = result.last_insert_id.as_i64().ok_or_else( | | anyhow!("插入失败")) ?;
let records = record_repo::select_by_ids( & mut tx, vec![id]).await?;

// ✗ 错误：insert 后直接读 record.id（必然为 None，报"插入失败"）
// Record::insert(rb, &record).await?;
// let id = record.id.ok_or_else(|| anyhow!("插入失败"))?;
```

## 6. 写入时机与失败回退

1. `pick` 返回的 `studentId` 仅用于 UI 展示（从 `studentStore` 反查姓名）。
2. `create` 成功返回 `RollcallRecord` 后，才执行 `recordStore.upsert(record)`（表格刷新）。
3. 若保存失败：`#undoShow()` 将当前展示名字撤销为「等待点名」，通过 `alert` 告知失败原因，状态回到 Idle。

## 7. 模块职责划分

| 模块                                               | 职责                                                                                                                                                                    |
|----------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `RollcallEngine.svelte.ts`                         | 状态机、事务编排（pick + create）、计时器、`pendingStop` 标记；暴露 `phase / currentName / totalTimes / completedTimes / isRolling` 与 `toggle() / updateTotalTimes(n)` |
| `RollcallPage.svelte`                              | 纯展示层：从 store 派生 `display`、`groupInfo`，绑定引擎状态渲染；`$effect` 中加载两个 store                                                                            |
| `recordStore.svelte.ts`                            | 记录数据源，`upsert` 只在保存成功后调用                                                                                                                                 |
| `types/RollcallPhase.ts`、`types/RollcallEvent.ts` | 状态机契约（枚举）                                                                                                                                                      |

`isRolling` 由 `phase !== Idle` 派生，覆盖 Animating / Picking / Showing 全流程；`display` / `groupInfo` 为页面级展示派生量，保留在
Page 中。

## 8. 关键常量

| 常量            | 值     | 用途                          |
|-----------------|--------|-------------------------------|
| `ROLL_INTERVAL` | 80ms   | 名字滚动切换间隔              |
| `ANIM_DURATION` | 1000ms | 连续点名动画自动结束时长      |
| `SHOW_DURATION` | 1000ms | 结果展示时长（ShowDone 延迟） |

## 9. 文件索引

| 文件                                                 | 角色                                  |
|------------------------------------------------------|---------------------------------------|
| `frontend/src/lib/types/RollcallPhase.ts`            | 状态枚举                              |
| `frontend/src/lib/types/RollcallEvent.ts`            | 事件枚举                              |
| `frontend/src/lib/services/RollcallEngine.svelte.ts` | 状态机引擎（核心）                    |
| `frontend/src/pages/RollcallPage.svelte`             | 点名面板页面                          |
| `frontend/src/lib/commands/rollcall.ts`              | `pick` 命令封装                       |
| `frontend/src/lib/commands/record.ts`                | `create` / `list` / `update` 命令封装 |
| `backend/src/cmd/rollcall.rs`                        | `pick` Tauri 命令                     |
| `backend/src/cmd/record.rs`                          | `record_single_create` Tauri 命令     |
| `backend/src/service/record_service.rs`              | `create`（含 last_insert_id 获取）    |
| `backend/src/service/rollcall_service.rs`            | `pick` 纯随机实现                     |
