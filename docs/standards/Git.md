# 项目 Git 工作流规范

> 本文档规定本仓库的分支模型、命名、合并与发布规则，所有参与者（含自动化）均应遵守。
> 文件位置：`docs/standards/Git.md`。
> 文中命令以本仓库实际配置为准：**远程名为 `github`**；新克隆默认的 `origin` 由 `just setup` 改名为 `github`（见 §1.1）。

---

## 一、仓库事实

| 项目         | 值                                                               |
|:-------------|:-----------------------------------------------------------------|
| 仓库         | `bytecascade-Lee/rollcaller`                                     |
| **默认分支** | **`develop`**                                                    |
| 生产分支     | `master`                                                         |
| **远程名**   | **`github`** → `git@github.com:bytecascade-Lee/rollcaller.git`   |
| 发布方式     | 在 `master` 用 `just bump` 打 `vX.Y.Z` tag → CI 构建、签名、发布 |
| 合并方式     | **只允许 Merge commit**                                          |

分支模型（单向汇入）：

```text
master        生产/发布线，只接受 develop 与 hotfix 的合并
  ↑
develop       日常集成线（默认分支），所有功能在此汇合
  ↑
feat/…        临时分支，合并后立即删除
```

### 1.1 本地配置

`just setup` 会将下面这些写进 **仓库级**配置，新机器克隆后执行一次即可：

| 键               | 值      | 作用                                        |
|:-----------------|:--------|:--------------------------------------------|
| `pull.ff`        | `only`  | 拉取只允许快进，需要合并时由人显式决定      |
| `merge.ff`       | `false` | 合并必须产生合并提交                        |
| `core.quotepath` | `false` | 中文路径不转义成八进制串                    |
| `fetch.prune`    | `true`  | 远程已删除的分支在 fetch 时一并清理本地引用 |

`pull.ff=only` 为必设项：`git pull` 会继承 `merge.ff=false`，使「本地无独有提交、仅落后远程」的情况也生成 `Merge branch 'develop' of ...` 合并提交；该提交一旦进入 `develop` 便无法移除（原则 1 禁止改写公共分支历史）。`pull.ff` 的优先级高于 `merge.ff`，设置此项后拉取恢复为纯快进。

`just setup` 另外会归一**远端拓扑**，并预建 `data/`、`release/Local/`、`release/CI-CD/` 等被忽略的运行期目录。这些目录不在版本控制内，缺失只会在运行时暴露。`just doctor` 是同一套检查的只读版本，仅报告，不修改。

远端拓扑按「从哪儿克隆」自动判定，不需要声明自己是协作者还是外部贡献者：

| 克隆来源                             | 归一动作                             | 拉取源   | 推送目标         |
|:-------------------------------------|:-------------------------------------|:---------|:-----------------|
| 规范仓库（`origin`）                 | `origin` 改名为 `github`             | `github` | `github`         |
| 自己的 fork（`origin`）              | 保留 `origin`，新增 `github`         | `github` | `origin`（fork） |
| CNB 镜像（`origin`）                 | `origin` 改名为 `cnb`，新增 `github` | `github` | `github`         |

三条规则：

- **拉取源恒为 `github`**。镜像需等待 CI 同步（1~2 分钟），从镜像拉取会使工作基于过时提交，推送时被拒绝，只能撤销重做。国内拉取较慢时可用 `git fetch cnb` 经镜像获取，但禁止将长期分支的上游设为 `cnb`。
- **镜像只读**。`cnb` 由 `sync-mirrors` 单向强制推送（`+refs/heads/*`），推送到镜像的内容会在下一次同步时被覆盖。
- **推送目标按身份分配**。本地存在 fork 远端时推送 fork，否则推送规范仓库。仅分叉工作流（拉取与推送不是同一远端）写入 `remote.pushDefault`；单远端不写，否则会覆盖临时分支 `git push -u` 记录的目标。

新增 `github` 远端但尚未执行 `git fetch github` 时，`git status` 会显示 `[gone]`（跟踪引用尚未建立），执行一次拉取即恢复。

图形客户端（RustRover）另有两点：

- `Settings → Version Control → Git → Update method` **必须是 `Merge` 或 `Branch Default`**。选 `Rebase` 会改写 `develop`，直接违反原则 1。
- 若某次 Update 之后仍然多出 `Merge branch 'develop' of …` 提交，说明该版本 IDE 走的是「fetch + merge」两步，`pull.ff` 未参与；改用命令行 `git merge --ff-only github/develop` 复核。

---

## 二、核心原则

### 原则 1：公共分支永不变基

`master` 与 `develop` 的历史 **绝对不可改写**。禁止对它们执行 `rebase`、`commit --amend` 后强推、`reset` 后强推。发现公共分支历史需要"修正"时，只能用新的提交（`git revert`）向前修复。

### 原则 2：临时分支在推送前可自由变基

尚未推送到远程的临时分支，可以随意 `rebase`、`commit --amend`、整理本地提交； **一旦推送就不再变基**（除非确认无人依赖，且只用 `--force-with-lease`，绝不用 `--force`）。

### 原则 3：禁止压缩合并

一律使用 `git merge --no-ff`。本项目用 `git rev-list --count HEAD` 作为构建号来源之一（见 §九），squash 会让该计数严重失真。

### 原则 4：单向汇入，发布提交不回灌

`develop → master` 是唯一方向。 **`master` 上由发布产生的合并提交，绝不允许合回 `develop`**（禁止 back-merge）。原因：那些合并提交只属于发布线，带回 develop 会让两条历史长期互相纠缠。

需要把 `master` 上的修复带回 develop 时， **只能用 `cherry-pick` 复制提交**，绝不允许合并 `master` 或 `hotfix` 分支（见 §5.5）。

### 原则 5：不直接向公共分支提交

`master` 与 `develop` 只接受合并，不接受直接 `commit`。例外仅有 §十 列出的紧急恢复场景，且必须立即用后续提交补齐记录。

---

## 三、分支命名

### 3.1 核心分支（长期存在）

| 分支                  | 用途                 | 说明                                                                      |
|:----------------------|:---------------------|:--------------------------------------------------------------------------|
| `master`              | 生产 / 发布线        | 固定名，不可删除                                                          |
| `develop`             | 日常集成线、默认分支 | 固定名，不可删除                                                          |
| `hotfix/<tag>/<简述>` | 紧急修复             | `<tag>` 是**本次将要打的 tag**，如 `hotfix/v0.8.3/cnb-mirror-prune-error` |

`hotfix` 将 tag 写入分支名，使本次发布的版本号在分支名中直接可见，并区分同时存在的多个 hotfix。

### 3.2 临时分支

| 场景                           | 格式                   | 示例                                      |
|:-------------------------------|:-----------------------|:------------------------------------------|
| 默认（本地开发、单人仓库推送） | `<type>/<简述>`        | `feat/multi-sheet-import`                 |
| 多人协作，需要区分归属         | `<name>/<type>/<简述>` | `bytecascade-Lee/feat/multi-sheet-import` |

- `<type>` 取值：`feat`、`fix`、`refactor`、`docs`、`test`、`chore`
- `<name>` 建议用 GitHub 用户名（协作者同样加前缀区分）
- `<简述>` 用 kebab-case 英文，2~5 个词；不要中文、空格、下划线、`#`

分支名只是归属线索，不是权限边界：单人开发用扁平格式即可，只有确实需要区分是谁的分支时才加 `<name>` 前缀。

### 3.3 分支生命周期

| 分支                 | 何时删除                                                                        |
|:---------------------|:--------------------------------------------------------------------------------|
| 临时分支             | 合并进 `develop` 后立即删除（本地 + 远程）                                      |
| `hotfix/*`           | 合入 `master` 后立即删除（修复内容要么已在 `develop`，要么已 cherry-pick 回去） |
| `master` / `develop` | 永不删除                                                                        |

> 删除远程分支前确认 PR 已合并—— **PR 的 head 分支一旦删除，该 PR 就无法再被重新打开**（GitHub 需要 head 分支存在）。

---

## 四、标准开发流程（临时分支）

### 4.1 创建分支

```bash
# 始终从最新的 develop 起
git checkout develop
git pull --ff-only github develop
git checkout -b feat/multi-sheet-import
```

`--ff-only` 是安全阀：本地若存在独有提交，命令直接报错，不会生成合并提交。

### 4.2 开发与提交

```bash
git add .
git commit -m "feat(import): 支持选择任意工作表"
```

提交信息规范见 §七。可以分多次提交，保持每个提交语义单一。

### 4.3 同步最新的 develop

**未推送的分支**：直接变基，保持线性历史。

```bash
git fetch github develop
git rebase github/develop
```

**已推送的分支**——只能合并，禁止变基：

```bash
git fetch github develop
git merge github/develop -m "Merge branch 'develop' into feat/multi-sheet-import"
```

### 4.4 冲突处理

```bash
# 查看冲突文件
git status

# 手工解决后标记为已解决
git add <file>

# 变基途中
git rebase --continue
# 或放弃变基
git rebase --abort

# 合并途中
git commit            # 完成合并提交
# 或放弃合并
git merge --abort
```

解决冲突时**禁止为消除冲突而删除对方的改动**；确需丢弃时，在提交信息中说明原因。

### 4.5 合并回 develop

```bash
git checkout develop
git pull --ff-only github develop
git merge --no-ff feat/multi-sheet-import -m "Merge branch 'feat/multi-sheet-import' into develop"
git push github develop

# 清理
git branch -d feat/multi-sheet-import
git push github --delete feat/multi-sheet-import   # 若曾推送
```

走 Pull Request 时，同样必须使用 **Merge（不是 Squash、不是 Rebase）**。

---

## 五、紧急修复（hotfix）

适用场景：`master` 上存在必须立刻修的问题，且不能等下一次例行发布。

### 5.1 先判断缺陷在哪条线上

| 情况                                       | 做法                                     |
|:-------------------------------------------|:-----------------------------------------|
| 缺陷在 `develop` 与 `master` 上都存在      | 按 §5.2 先在 `develop` 修，再摘到 hotfix |
| 缺陷只影响已发布代码（`develop` 已重构掉） | 直接在 hotfix 上修，再按 §5.5 摘回       |

**缺陷只要在 `develop` 上同样存在，修复就先落在 `develop`**（唯一例外见 §5.5）。`develop` 是内容源头，先在此修复即可避免「把 hotfix 合回 develop」；后者在两条历史已分叉时必然把发布合并提交带进 `develop`（违反原则 4）。

### 5.2 在 develop 上修

```bash
git checkout develop
git pull --ff-only github develop
git checkout -b fix/cnb-mirror-prune-error

# 提交 1：只改代码，不夹带重构、格式化、改名
git add -A
git commit -m "fix(ci): 修正 CNB 镜像清理在空仓库下的报错"

# 提交 2：发布文档与版本索引
#    CHANGELOG.md、RELEASE_NOTES.md 新增章节，resources/update/versions.json 登记严重级别
git add CHANGELOG.md RELEASE_NOTES.md resources/update/versions.json
git commit -m "docs(release): 准备 v0.8.3 发布说明"

# 合回 develop
git checkout develop
git merge --no-ff fix/cnb-mirror-prune-error -m "Merge branch 'fix/cnb-mirror-prune-error' into develop"
git push github develop
```

两个提交分开，便于单独回退，或按需只摘取其中一个。

这两步都不动版本号：版本文件恒为占位值，真实版本号在构建期由 `build.py -v` 临时写入并还原（见 §6.1）；tag 由 `just bump` 在 `master` 上生成（见 §5.4）。

修复必须连带补上能触发该问题的测试或校验。

### 5.3 从 master 拉 hotfix 并摘取提交

```bash
git checkout master
git pull --ff-only github master
git checkout -b hotfix/v0.8.3/cnb-mirror-prune-error

# 显式列 SHA：修复提交 + 发布准备提交
git cherry-pick -x <fix-sha> <release-sha>
```

⚠️ **禁止使用 `master..fix/xxx` 这类范围写法**：`fix/xxx` 从 `develop` 拉出，该差集包含 `develop` 上全部未发布提交，会一次性摘进 hotfix。必须显式列 SHA，或使用「合并前的 `develop` 基点..`fix/xxx`」。

`-x` 会在提交信息末尾留下 `(cherry picked from commit …)`，这是两条线之间唯一的可追溯线索，不得省略。

### 5.4 合 master 并发布

hotfix 同样是一次正式发布，按 §六 打 tag 触发 CI：

```bash
git checkout master
git pull --ff-only github master
git merge --no-ff hotfix/v0.8.3/cnb-mirror-prune-error -m "Merge branch 'hotfix/v0.8.3/cnb-mirror-prune-error'"
git push github master

# 先推 master 再打 tag：bump 会校验 HEAD 与 github/master 一致
just bump patch
git push github v0.8.3
```

`just bump` 会按仓库里最大的 tag 推算版本号，因此它算出的结果必须正好是 `v0.8.3`；先用 `just bump patch --dry-run` 确认。它只在本地产 tag，推送后才触发 CI（见 §6.1）。

### 5.5 例外：修复只能先在 master 线做

若缺陷只存在于已发布代码，先按 §5.4 发版，再把修复摘回 `develop`：

```bash
git checkout develop
git cherry-pick -x <hotfix-fix-sha>
```

此场景下发布说明章节与 `versions.json` 条目直接写在 hotfix 分支上，**不摘回 `develop`**：它们描述的是 `master` 线的那个版本，`develop` 下次发布时另有自己的章节与条目。

**任何情况下禁止 `git merge hotfix/...` 进 `develop`**：hotfix 从 `master` 拉出，其祖先链包含全部发布合并提交，合并它等价于把整段 `master` 历史带入 `develop`；不存在「只合分支本身」的做法。

### 5.6 清理

```bash
git branch -d hotfix/v0.8.3/cnb-mirror-prune-error
git push github --delete hotfix/v0.8.3/cnb-mirror-prune-error
git branch -d fix/cnb-mirror-prune-error
```

> 删除远程分支前确认 PR 已合并—— **PR 的 head 分支一旦删除，该 PR 就无法再被重新打开**。

---

## 六、版本发布

### 6.1 版本号规范

- 严格语义化版本：`MAJOR.MINOR.PATCH`，可选预发布段 `-alpha.1`、`-beta.2`、`-rc.1`
- **tag 形如 `v0.8.2`**（带 `v` 前缀）
- 构建元数据（`+build`）不进入 tag；`prepare` 会把它静默擦除

**版本号不写在仓库里的文件**。`pyproject.toml`、`backend/Cargo.toml`、`backend/tauri.conf.json5`、`frontend/package.json` 恒为占位值 `0.1.0-dev`；构建时 `build.py -v <version>` 把四处版本号与 `backend/Cargo.lock`、`uv.lock` 一并改写/同步，构建结束用 git 还原（因此本地构建要求这些文件相对 HEAD 干净）。因此本仓库不存在「提升版本号」这一动作，只存在「打 tag」。

tag 统一由 `just bump` 产出，基准是仓库里最大的 `v*` tag：

| 命令                                  | 结果                                                                                 |
|:--------------------------------------|:-------------------------------------------------------------------------------------|
| `just bump major` / `minor` / `patch` | 升对应位；当前是预发布态时，毕业到它的基线版本（`0.8.3-rc.2` + `patch` → `0.8.3`）   |
| `just bump prerelease`                | 稳定版 → `alpha.1`；`alpha` → `beta`；`beta` → `rc`，换档时序号重置；`rc` 是最后一档 |
| `just bump prerelease --serial`       | 同档进号：`alpha.1` → `alpha.2`                                                      |
| `just bump prerelease --base minor`   | 从稳定版进入预发布时改用下一个 minor 作基线（默认 patch）                            |

`--dry-run` 只打印将要打的 tag，不创建。`bump` **只创建 tag，不推送**——推送才会触发发布。

前置条件，任一不满足即拒绝执行：在 `master` 上（detached HEAD 也算失败）、已跟踪文件无未提交改动、`HEAD` 等于 `github/master`、同名 tag 尚不存在。

### 6.2 发布前检查清单

- [ ] `develop` 上所有目标功能已合入，工作区干净
- [ ] `just check` 通过（前端 `pnpm check` + 后端 `cargo check`）
- [ ] `CHANGELOG.md` 已新增本版本章节：`## 0.8.2 {#0-8-2}`，并更新顶部目录
- [ ] `RELEASE_NOTES.md` 已新增本版本章节：`## 0.8.2 {#v0-8-2}`（面向用户，只写可感知的变化），并更新顶部目录
- [ ] `resources/update/versions.json` 已登记本版本的 `severity`
- [ ] `master` 上尚无同名 tag（`just bump` 会自己拦住）

CI 的 `prepare` 阶段只解析 `RELEASE_NOTES.md`，要求章节**存在、唯一、非空**，任一不满足即失败（`--draft` 时跳过该校验）。`CHANGELOG.md` 与两份文档的顶部目录不参与校验，但缺失属文档缺陷。章节写法与内容要求见 `docs/standards/CHANGELOG_STANDARD.md` 与 `docs/standards/RELEASE_NOTES_STANDARD.md`。

`resources/update/versions.json` 是仓库维护的版本索引，CI 从里面读 `severity`；查不到会告警并按 `normal` 兜底，然后写进构建出的索引。

### 6.3 发布步骤

```bash
# 1. 发布文档与版本索引走一条临时分支（不直接向 develop 提交）
git checkout develop
git pull --ff-only github develop
git checkout -b docs/release-notes
#    改 CHANGELOG.md / RELEASE_NOTES.md / resources/update/versions.json
git add CHANGELOG.md RELEASE_NOTES.md resources/update/versions.json
git commit -m "docs(release): 准备 v0.8.2 发布说明"
git checkout develop
git merge --no-ff docs/release-notes -m "Merge branch 'docs/release-notes' into develop"
git push github develop
git branch -d docs/release-notes

# 2. develop 合入 master（三路合并）
git checkout master
git pull --ff-only github master
git merge --no-ff develop -m "release: 发布版本 v0.8.2"
git push github master

# 3. 在 master 的最新提交上生成 tag，再推送触发 CI
just bump patch          # 先 --dry-run 确认推算结果是 v0.8.2
git push github v0.8.2
```

**顺序不可交换**：先推 `master` 再执行 `bump`（后者要求 `HEAD` 与 `github/master` 一致）；先有提交再打 tag（tag 指向提交，打在未推送的提交上会使 CI 构建的代码没有任何分支引用）。版本号由 tag 决定，无需改动版本文件。

### 6.4 CI 会做什么

tag 推送后，`release.yaml` 依次执行：

| 阶段      | 内容                                                                                                    |
|:----------|:--------------------------------------------------------------------------------------------------------|
| `prepare` | 解析版本号（擦除 `+build`，允许预发布段）→ 校验 `RELEASE_NOTES.md` 章节（`--draft` 跳过）→ 生成构建矩阵 |
| `build`   | 按矩阵逐目标构建（当前为 x86_64 / aarch64 Windows）→ 打包 → 签名 + sha256 → 上传 `assets/rollcaller-*`  |
| `release` | 鉴权 → 发现产物 → 写版本索引与双平台清单 → 在 GitHub/CNB 创建或复用 tag → 双平台发布                    |

产物发现会检查「缺件、多余文件、命名不符」，任一不满足即失败；上传顺序固定为 产物 → 清单 → 版本索引，`.sig` / `.sha256` 只经 artifact 传递。**禁止用本地 `cargo tauri build` 的产物手工上传**：该做法绕过签名与发布说明校验。

`release` 阶段调用的 `ensure_tag` 对**已存在**的同名 tag 只记录并沿用，**不核对它指向哪个提交**。tag 推送的路径下这是合理的（tag 由人推、检出内容就是它指向的提交）；但反过来，不能用「手动触发」去重发一个已有 tag 的版本号，否则新构建的产物会挂到旧 tag 上，版本号和代码对不上。

### 6.5 构建失败怎么办

先看失败发生在哪个阶段，它决定「对外留下了多少痕迹」：

| 失败阶段                          | 已对外留下什么                                         |
|:----------------------------------|:-------------------------------------------------------|
| `prepare` 校验失败 / `build` 失败 | 只有一个裸 tag（无 Release 的 tag）；Release、清单、版本索引均未写入 |
| `release` 上传失败                | Release 与部分附件（上传顺序：产物 → 清单 → 版本索引） |
| GitHub 成功、CNB 失败             | 两端不一致的完整发布                                   |

**约定：tag 一经推送就不移动，也不复用该版本号；直接发下一个 patch 版本。**

不移动 tag 的理由：

- `git tag -f` 加强制推送只改服务器，已 fetch 过旧 tag 的机器不会同步，导致同名 tag 在不同机器上指向不同提交
- GitHub Release 绑定在 tag 上，附件、签名、清单都要重传，中途失败会留下半套产物
- 更新器按语义化版本比较，同版本不同二进制会导致客户端永远不再拉取

处理步骤：

1. 按 §五 在 `develop` 上修，把新章节写进两份文档
2. `RELEASE_NOTES.md`： **删掉未发布的 0.8.2 章节**，内容并入 0.8.3，开头注明「本版本包含原计划在 0.8.2 发布的内容」
3. `CHANGELOG.md`： **保留** 0.8.2 章节并标注「未发布（构建失败）」——它是内部史，不是对外索引
4. 合入 `master` 后用 `just bump patch` 打下一个版本（`bump` 以最大 tag 为基准，裸 tag `v0.8.2` 会使其算出 `v0.8.3`）
5. 该裸 tag `v0.8.2` 保留在仓库中即可，无需处理：它没有 Release、没有清单更新，客户端不会看到它

### 6.6 手动触发（备用）

仅在 tag 推送不可用时使用：`Actions → Release → Run workflow`

- `version`：语义化版本号（如 `0.8.3`），允许预发布段，携带的 `+build` 会被擦除
- `draft`：草稿发布，跳过 `RELEASE_NOTES.md` 校验

没有 `branch` 输入：运行的是 ref 下拉框中选中的提交，**CI 不校验它是否为 `master`**，手动触发前须自行确认。发布末尾会调用 `ensure_tag`：同名 tag 不存在时在检出提交上创建，已存在则原样沿用（见 §6.4 末段），因此不得用它重发已有 tag 的版本号。

### 6.7 合入即校验

`develop` / `master` 收到 push 会跑 `.github/workflows/quality-gate.yaml`：前端类型检查 -> `test.py`（codegen + 后端 `cargo test` + 前端单测）-> 生成物是否已提交。

- 纯文档改动（`**.md`、`docs/**`、`LICENSE`）不触发；同一分支来了新 push 会取消仍在跑的旧 run
- 只做校验、不产出任何可发布产物，与 §6.1 的发布链路互不影响
- 目标三元组写死为 `x86_64-pc-windows-msvc`：只有 x64 能在 `windows-latest` 上原生执行，arm64 交叉编译出来的测试二进制跑不起来


### 6.8 发布后

- 确认 GitHub Release 与 CNB Release 都已生成，产物齐全
- `dependabot.yml` 的配置会被重新读取（默认分支上的版本才是生效版本）
- **不要把 `master` 合回 `develop`**

---

## 七、提交信息规范

格式：

```text
<type>(<scope>): <中文简述>

- 要点一
- 要点二
```

- **type**：`feat`、`fix`、`refactor`、`docs`、`test`、`chore`、`perf`、`build`、`ci`
- **scope 必填**，用模块或目录名：
    - 后端：`import`、`update`、`config`、`db`、`cmd`
    - 前端：`updater`、`import`、`routing`、`student`
    - 脚本：`build`、`release`、`codegen`
    - CI / 仓库配置：`ci`、`deps`
- **简述与正文一律用中文**，type 与 scope 保持英文
- 正文逐条用 `-`，说明 **为什么**改，而不是罗列改了哪些文件
- 破坏性变更加 footer：`BREAKING CHANGE: <影响说明>`

示例：

```text
fix(import): 切换工作表后重置列映射与冲突决策

- 复用预览时的 resetMapping 语义，避免沿用上一张表的列配置
- 空表不再渲染表头箭头
```

---

## 八、Pull Request 与合并方式

### 8.1 合并方式

仓库已关闭 Squash / Rebase，仅保留 **Merge commit** 一种合并方式。该限制由平台配置强制，不是建议：

| 方式                  | 结论        | 原因                                         |
|:----------------------|:------------|:---------------------------------------------|
| Create a merge commit | ✅ 唯一允许 | 保留完整历史与提交计数                       |
| Squash and merge      | ❌ 已禁用   | 提交被压缩，`rev-list --count` 失真          |
| Rebase and merge      | ❌ 已禁用   | 改写提交哈希，且对含合并提交的分支行为不透明 |

PR 标题遵循 §七 的提交信息格式；描述写清动机、影响面和验证方式。

### 8.2 Dependabot PR

自动化开的 PR 同样受本规范约束，另有几条专属约定：

- **base 恒为默认分支 `develop`**，无法通过 `target-branch` 改变（安全更新固定打默认分支）
- ⚠️ **禁止 `gh pr edit --base`**。修改 base 会使 PR 被标记为「被 Dependabot 以外的人编辑过」，此后 Dependabot 拒绝更新该 PR，也不会为同一依赖另开 PR
- 合并方式同样为 **Merge**，不得为追求「历史干净」而 squash
- 关闭 Dependabot PR 后 **不会自动重建**；需要重新生成时，走 `Settings → Advanced Security → Dependabot security updates` 关掉再打开
- `glib` / `lru` 在当前依赖约束下达不到修复版本，会长期留在告警列表里，等上游升级
- `open-pull-requests-limit: 0` 表示 **关闭版本更新、保留安全更新**；安全更新 PR 不受该限制约束

---

## 九、提交计数与构建号

`git rev-list --count HEAD` 是构建元数据中的 `commit_count` 字段，用于诊断与版本串展示。它 **不是**语义化版本号，不要用它推导版本。

完整的构建元数据形如 `<ref_name>.<计数>.<短哈希>`：分支构建用分支名， **tag 构建（`GITHUB_REF_TYPE=tag`）与 detached HEAD 一律记作 `detached`**，本地构建用当前分支名。

| 操作                  | 计数变化                     |
|:----------------------|:-----------------------------|
| 普通提交              | +1                           |
| 三路合并（`--no-ff`） | +（被合并分支提交数 + 1）    |
| 快进合并              | +（被合并分支提交数）        |
| 变基到新基            | 取决于新基的提交数，可能增减 |
| `reset --hard` 回退   | 减少                         |
| Squash 合并           | 大幅减少（已禁用）           |

---

## 十、错误恢复

| 场景                                 | 处理                                                                       |
|:-------------------------------------|:---------------------------------------------------------------------------|
| 在 `develop` 上误提交，**尚未推送**  | `git reset --soft HEAD~1` 撤回提交、保留改动，改到临时分支重新提交         |
| 在公共分支误提交，**已推送**         | `git revert <sha>` 生成反向提交；**绝不 force push**                       |
| 临时分支 rebase 后推不上去           | 确认无他人依赖后 `git push --force-with-lease`；公共分支禁止               |
| tag 打错但**未推送**                 | `git tag -d v0.8.3` 后重新打                                               |
| tag 已推送、构建失败但**无 Release** | 不要删 tag，按 §6.5 发下一个 patch 版本                                    |
| tag 已推送、Release 已生成           | **不要删 tag**，改发下一个 patch 版本并在发布说明里注明                    |
| 误把 `hotfix` 合进 `develop`         | 用 `git revert -m 1 <merge-sha>` 撤销该合并，再按 §5.5 用 cherry-pick 重做 |
| 误把 `master` 合进 `develop`         | 用 `git revert -m 1 <merge-sha>` 撤销该合并，而不是 reset                  |
| 误删了 PR 的 head 分支               | 无法挽回：PR 已关闭/无法重开，只能重建分支后开新 PR                        |

---

## 十一、不应提交的内容

以下路径不纳入版本控制，**禁止用 `git add -f` 强制纳入**：

| 路径                                       | 依据         | 原因                                     |
|:-------------------------------------------|:-------------|:-----------------------------------------|
| `env/`                                     | `.gitignore` | 含密钥与签名材料                         |
| `data/`                                    | `.gitignore` | 运行期数据（数据库、导入样例、日志）     |
| `release/`                                 | `.gitignore` | 本地发布产物与 CI 日志归档               |
| `assets/`                                  | `.gitignore` | CI 构建产物汇总目录                      |
| `docs/changes/`、`docs/diff/`              | `.gitignore` | 提交信息与 diff 导出的中间产物           |
| `backend/target/`                          | `.gitignore` | Rust 构建产物                            |
| `backend/gen/`                             | `.gitignore` | 代码生成物与 `gen/build/build_info.json` |
| `.cargo/config.toml`                       | `.gitignore` | 本机生成，含 ts-rs 导出目录              |
| `resources/tauri/*`                        | `.gitignore` | 本地解包的 tauri-cli                     |
| `frontend/node_modules/`、`frontend/dist/` | `.gitignore` | 依赖与前端产物                           |
| `backend/tauri.conf.json5.local`           | `.gitignore` | 本机专属覆盖                             |

行尾策略由 `.gitattributes` 首行 `* text=auto eol=lf` 定义：**索引与工作区统一为 LF**，不随平台或 `core.eol` 变化；例外仅为 `*.bat` / `*.cmd` / `*.ps1`（显式 `eol=crlf`）。新增文件不得混入 CRLF。

---

## 十二、命令速查

| 场景                                            | 命令                                                                   |
|:------------------------------------------------|:-----------------------------------------------------------------------|
| 克隆（默认检出 develop）                        | `git clone git@github.com:bytecascade-Lee/rollcaller.git`              |
| 初始化环境                                      | `just setup`（等价：`uv run ./scripts/setup.py`）                      |
| 体检环境                                        | `just doctor`（只读）                                                  |
| 同步 develop（安全）                            | `git pull --ff-only github develop`                                    |
| 从 develop 建分支                               | `git checkout -b feat/xxx develop`                                     |
| 未推送分支同步 develop                          | `git rebase github/develop`                                            |
| 已推送分支同步 develop                          | `git merge github/develop`                                             |
| 合并功能到 develop                              | `git merge --no-ff feat/xxx -m "Merge branch 'feat/xxx' into develop"` |
| 创建 hotfix                                     | `git checkout -b hotfix/v0.8.3/xxx master`                             |
| 合并 hotfix 到 master                           | `git merge --no-ff hotfix/v0.8.3/xxx`                                  |
| 修复摘到 hotfix                                 | `git cherry-pick -x <sha> <sha>`（**显式列 SHA，禁用范围写法**）       |
| 修复摘回 develop（例外）                        | `git cherry-pick -x <sha>`（**永不 merge hotfix 进 develop**）         |
| 预览下一个版本号                                | `just bump patch --dry-run`（档位可换 major / minor / prerelease）     |
| 打 tag                                          | `just bump patch`（在 `master` 上，只创建不推送）                      |
| 本地构建                                        | `just build 0.8.2 x86_64-pc-windows-msvc`                              |
| 本地发布（驻留更新服务，需先构建同版本）        | `just release 0.8.2`                                                   |
| 推 tag 触发发布                                 | `git push github v0.8.2`                                               |
| 删除远程分支                                    | `git push github --delete <branch>`                                    |
| 查看提交计数                                    | `git rev-list --count HEAD`                                            |
| 查看拓扑                                        | `git log --oneline --graph --all`                                      |

本地 `just build` / `just release` 同样要签名：缺 `TAURI_SIGNING_PRIVATE_KEY` 或 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 会直接失败，密钥材料放在 `env/`（不纳入版本控制）。
