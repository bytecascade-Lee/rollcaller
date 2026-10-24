*[en-US](README-en-US.md)*

# Rollcaller

课堂自动点名工具，基于 Tauri + Svelte 构建。

---

## 功能

- **随机点名**：从学生列表中随机抽取，支持单次和连续点名
- **学生管理**：增删改查、软删除与恢复、批量操作
- **考勤记录**：自动保存点名结果，支持批量修改状态和备注
- **导入导出**：Excel 导入学生名单，导出学生数据和考勤记录
- **便携模式**：解压即用，数据跟随软件目录

---

## 技术栈

- 前端：Svelte 5 + TypeScript + Vite + pnpm
- 后端：Rust + Tauri 2.11
- 数据库：SQLite + RBatis
- 样式：Open Props + Phosphor
- 脚本与构建：Python（由 uv 管理）+ just

---

## 开发

### 环境要求

| 工具                             | 最低版本 | 用途                   |
|:---------------------------------|:---------|:-----------------------|
| [git](https://git-scm.com/)      | 2.30     | 版本控制               |
| [uv](https://docs.astral.sh/uv/) | 0.5      | 管理 Python 与脚本依赖 |
| [just](https://just.systems/)    | 1.0      | 统一任务入口           |
| [Node.js](https://nodejs.org/)   | 22       | 前端构建               |
| [pnpm](https://pnpm.io/)         | 11       | 前端依赖               |
| [Rust](https://rustup.rs/)       | 1.80     | 后端与 Tauri           |

Python 解释器（3.14）由 `uv sync` 自动下载，不需要单独安装。`just doctor` 会逐条核对上表，并检查仓库级 git 配置与依赖是否就位。

### 初始化（有 just）

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller
just setup
```

`just setup` 依次完成：体检工具链 → 写仓库级 git 配置 → `pnpm install --frozen-lockfile` → `uv sync` → 刷新脚本导入钩子。它是幂等的，重建虚拟环境后可以再跑一次。

写入的 git 配置（`--local`，不影响其他仓库）见 [Git.md](Git.md) §1.1，其中 `pull.ff=only` 决定拉取行为，不要随手改回默认。

### 初始化（没有 just）

逐条执行等价命令即可，顺序与 `just setup` 一致：

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller

# 1. 仓库级 git 配置
git config --local pull.ff only           # 拉取只允许快进，需要合并时由人显式决定
git config --local merge.ff false         # 合并必须产生合并提交
git config --local core.quotepath false   # 中文路径不转义
git config --local fetch.prune true       # 远程删除的分支顺带清理

# 2. Python 依赖（创建 .venv 并按 uv.lock 安装）
uv sync

# 3. 脚本导入钩子
cp scripts/tool/sitecustomize.py .venv/sitecustomize.py

# 4. 前端依赖
cd frontend && pnpm install --frozen-lockfile && cd ..
```

`sitecustomize.py` 是 CPython 启动时自动加载的钩子，它把 `<项目根>/scripts` 注入 `sys.path`，因此 `scripts/` 下的脚本可以直接 `from cli import git`、`from common.logger import info`。`.venv/` 不进版本控制，
**重建虚拟环境后需要重新复制一次**（`just setup` 会自动做）。

> Windows PowerShell 下把 `cp` 换成 `Copy-Item`。

### 常用命令

| 目的             | 有 just              | 没有 just                                                |
|:-----------------|:---------------------|:---------------------------------------------------------|
| 体检环境         | `just doctor`        | `uv run scripts/setup.py --check`                        |
| 初始化环境       | `just setup`         | `uv run scripts/setup.py`                                |
| 启动开发环境     | `just dev`           | `uv run scripts/dev.py`                                  |
| 仅前端开发服务器 | `just dev-fe`        | `cd frontend && pnpm dev`                                |
| 类型检查         | `just check`         | `cd frontend && pnpm check`、`cd backend && cargo check` |
| 格式化           | `just fmt`           | `cd backend && cargo fmt`                                |
| 测试             | `just test`          | `cd frontend && pnpm test`、`cd backend && cargo test`   |
| 本地构建         | `just build 0.8.3`   | `uv run scripts/build.py -v 0.8.3 -m local`              |
| 本地出包         | `just release 0.8.3` | `uv run scripts/release.py -v 0.8.3 -m local`            |

所有脚本入口都在 `scripts/` 下，用 `uv run` 调用即可，不需要手动激活虚拟环境。`just` 只是这些命令的快捷方式，`justfile` 里的每条 recipe 都能逐字翻译成上表右列的形式。

### 构建产物

- 本地构建与出包：`release/Local/`
- 正式发布：由 `master` 上的 tag 触发 CI 构建、签名并发布，见 [Git.md](Git.md) §六

---

## 数据库迁移

迁移文件位于 `resources/migrations/`，命名格式：

```
V{version}__{description}.sql
```

示例：`V1__create-student-table.sql`

应用启动时会自动执行未运行的迁移。

---

## 配置

应用支持三种运行模式：

| 模式     | 说明                                  |
|:---------|:--------------------------------------|
| 开发模式 | `just dev` 时自动启用，使用开发数据库 |
| 安装模式 | 通过安装包安装，数据位于系统标准目录  |
| 便携模式 | 数据跟随应用目录                      |

---

## 许可证

[MIT](LICENSE)

---

## 贡献

欢迎提交 [Issue](https://github.com/bytecascade-Lee/rollcaller/issues) 。
