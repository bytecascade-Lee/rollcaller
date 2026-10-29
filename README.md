<!-- help:hide -->
<p align="center">
  <img src="docs/assets/logo.png" width="160" height="160" alt="Rollcaller">
</p>

<h1 align="center">Rollcaller</h1>

<p align="center">课堂自动点名工具 · 随机抽人，自动记考勤</p>

<p align="center">
  <a href="README-en-US.md">English</a> ｜ <b>简体中文</b>
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/releases/latest"><img src="https://img.shields.io/github/v/release/bytecascade-Lee/rollcaller?include_prereleases&sort=semver&style=flat-square" alt="最新版本"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/blob/develop/LICENSE"><img src="https://img.shields.io/github/license/bytecascade-Lee/rollcaller?style=flat-square" alt="许可证"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20x64%20%7C%20ARM64-0078D6?style=flat-square&logo=windows&logoColor=white" alt="支持平台">
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/actions/workflows/release.yaml"><img src="https://img.shields.io/github/actions/workflow/status/bytecascade-Lee/rollcaller/release.yaml?style=flat-square&label=release" alt="发布流水线"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/actions/workflows/sync-mirrors.yaml"><img src="https://img.shields.io/github/actions/workflow/status/bytecascade-Lee/rollcaller/sync-mirrors.yaml?style=flat-square&label=sync-mirrors" alt="镜像同步"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/actions/workflows/quality-gate.yaml"><img src="https://img.shields.io/github/actions/workflow/status/bytecascade-Lee/rollcaller/quality-gate.yaml?style=flat-square&label=quality%20gate" alt="质量闸门"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/commits/develop"><img src="https://img.shields.io/github/last-commit/bytecascade-Lee/rollcaller?style=flat-square" alt="最近提交"></a>
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/stargazers"><img src="https://img.shields.io/github/stars/bytecascade-Lee/rollcaller?style=flat-square" alt="Star"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/forks"><img src="https://img.shields.io/github/forks/bytecascade-Lee/rollcaller?style=flat-square" alt="Fork"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/issues"><img src="https://img.shields.io/github/issues/bytecascade-Lee/rollcaller?style=flat-square" alt="议题"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/pulls"><img src="https://img.shields.io/github/issues-pr/bytecascade-Lee/rollcaller?style=flat-square" alt="拉取请求"></a>
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/releases"><img src="https://img.shields.io/github/downloads/bytecascade-Lee/rollcaller/total?style=flat-square" alt="下载量"></a>
  <a href="https://cnb.cool/ordinary-glory/rollcaller"><img src="https://img.shields.io/badge/mirror-cnb.cool-1A7AF8?style=flat-square" alt="国内镜像"></a>
</p>
<!-- help:show -->

课堂自动点名工具，面向需要随机提问和考勤记录的老师：从名单里随机抽人、自动记录结果、随时导出统计。全部数据保存在本机，无需联网即可使用。

## 功能特性

| 功能     | 说明                                                                   |
|:---------|:-----------------------------------------------------------------------|
| 随机点名 | 从学生名单中随机抽取，支持单次点名与连续点名，可设定次数、是否允许重复 |
| 语音播报 | 点名后朗读学生姓名，支持本地语音与云端 AI 两种模式                     |
| 学生管理 | 增删改查、软删除与恢复、搜索排序、批量操作                             |
| 批量导入 | 从 Excel（`.xlsx` / `.xls`）导入名单，可选工作表，自动处理重复学号     |
| 考勤记录 | 点名结果自动保存，按轮次分组查看，支持批量修改状态与备注               |
| 数据导出 | 学生名单与考勤记录导出为 Excel，可按全部、筛选结果或选中项导出         |
| 便携模式 | 解压即用，数据存放于程序目录，便于随 U 盘携带                          |
| 自动更新 | 应用内检查并安装新版本，无需手动下载覆盖                               |
| 内置帮助 | 帮助窗口提供使用文档、全文搜索与锚点导航                               |

## 下载安装

前往 [最新版本](https://github.com/bytecascade-Lee/rollcaller/releases/latest) 下载：

| 版本   | 文件名                                          | 说明                             |
|:-------|:------------------------------------------------|:---------------------------------|
| 安装版 | `rollcaller-<版本>-windows-<架构>-setup.exe`    | 标准安装，数据存放在系统用户目录 |
| 便携版 | `rollcaller-<版本>-windows-<架构>-portable.zip` | 解压即用，数据存放于程序目录     |

- 支持 Windows 10 与 Windows 11（64 位），提供 x64 与 ARM64 两种架构
- 依赖 WebView2 运行时：安装版会在安装过程中引导安装，便携版需自行[下载安装](https://developer.microsoft.com/zh-cn/microsoft-edge/webview2/#download-section)
- 每个产物在构建阶段生成 `.sha256` 校验值与 `.sig` 签名，供应用内更新校验完整性与来源
- 国内用户可通过 [CNB 镜像](https://cnb.cool/ordinary-glory/rollcaller) 克隆源码或下载，实际发布产物以 GitHub Releases 为准

## 技术栈

| 层次       | 技术                                                                                   |
|:-----------|:---------------------------------------------------------------------------------------|
| 桌面客户端 | [Tauri 2](https://tauri.app/)（Rust）                                                  |
| 前端       | [Svelte 5](https://svelte.dev/) + TypeScript + [Vite](https://vite.dev/)               |
| 数据       | SQLite + [RBatis](https://github.com/rbatis/rbatis)                                    |
| 样式       | [Open Props](https://open-props.style/) + [Phosphor Icons](https://phosphoricons.com/) |
| 脚本与构建 | Python 3.14（[uv](https://docs.astral.sh/uv/) 管理）+ [just](https://just.systems/)    |

## 开发

### 环境要求

| 工具                             | 最低版本 | 用途                   |
|:---------------------------------|:---------|:-----------------------|
| [git](https://git-scm.com/)      | 2.30     | 版本控制               |
| [Rust](https://rustup.rs/)       | 1.80     | 后端与 Tauri           |
| [Node.js](https://nodejs.org/)   | 22       | 前端构建               |
| [pnpm](https://pnpm.io/)         | 12       | 前端依赖               |
| [uv](https://docs.astral.sh/uv/) | 0.5      | 管理 Python 与脚本依赖 |
| [just](https://just.systems/)    | 1.0      | 统一任务入口           |

Python 解释器由 `uv sync` 自动下载，无需单独安装。`just doctor` 会逐条核对上表，并检查远端拓扑、仓库级 git 配置与依赖是否就位。

### 使用 just 初始化

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller
just setup
```

未配置 SSH key 时，请使用 `https://github.com/bytecascade-Lee/rollcaller.git` 克隆仓库。

`just setup` 依次完成：体检工具链 -> 归一远端（拉取源 `github`，推送目标按是否 fork 判定） -> 设置仓库级 git 配置 -> 按模板补齐 `backend/.cargo/config.toml`（已存在则不动） -> `pnpm install --frozen-lockfile` -> `uv sync` -> 刷新脚本导入钩子。该命令幂等，重建虚拟环境后可再次执行。

写入的 git 配置见 [Git.md](https://github.com/bytecascade-Lee/rollcaller/blob/develop/docs/standards/Git.md) §1.1，其中 `pull.ff=only` 决定拉取行为，不要随意修改。

### 使用 Python 脚本初始化

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller
uv run ./scripts/setup.py
```

该脚本与 just 命令的初始化机制是一致的，just 仅将该脚本的调用封装为命令。

### 逐步初始化

逐条执行等价命令即可，顺序与 `just setup` 一致：

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller

# 1. 远端归一：规范仓库固定叫 github
#    从 fork 或 CNB 镜像克隆时的完整规则见 docs/standards/Git.md §1.1
git remote rename origin github

# 2. 仓库级 git 配置
git config --local pull.ff only           # 拉取只允许快进，需要合并时由人显式决定
git config --local merge.ff false         # 合并必须产生合并提交
git config --local core.quotepath false   # 中文路径不转义
git config --local fetch.prune true       # 远程删除的分支顺带清理

# 3. cargo 本地配置（ts-rs 导出目录与开发库文件名）
cp backend/.cargo/config.toml.example backend/.cargo/config.toml

# 4. Python 依赖（创建 .venv 并按 uv.lock 安装）
uv sync

# 5. 脚本导入钩子
cp scripts/tool/sitecustomize.py .venv/sitecustomize.py

# 6. 前端依赖
cd frontend && pnpm install --frozen-lockfile && cd ..
```

`sitecustomize.py` 是 CPython 启动时自动加载的钩子，用于注入 `<项目根>/scripts` 到 `sys.path`， **重建虚拟环境后需要重新复制一次**（`just setup` 会自动完成复制）。

> Windows PowerShell 下把 `cp` 换成 `Copy-Item`。

### 常用命令

| 目的             | 有 just                                   | 没有 just                                                                                |
|:-----------------|:------------------------------------------|:-----------------------------------------------------------------------------------------|
| 体检环境         | `just doctor`                             | `uv run scripts/setup.py --check`                                                        |
| 初始化环境       | `just setup`                              | `uv run scripts/setup.py`                                                                |
| 启动开发环境     | `just dev x86_64-pc-windows-msvc`         | `uv run scripts/dev.py -t x86_64-pc-windows-msvc`                                        |
| 仅前端开发服务器 | `just dev-fe`                             | `cd frontend && pnpm dev`                                                                |
| 类型检查         | `just check x86_64-pc-windows-msvc`       | `cd frontend && pnpm check`、`cd backend && cargo check --target x86_64-pc-windows-msvc` |
| 格式化           | `just fmt`                                | `cd backend && cargo fmt`                                                                |
| 测试             | `just test x86_64-pc-windows-msvc`        | `uv run scripts/test.py -t x86_64-pc-windows-msvc`                                       |
| 仅前端测试       | `just test-fe`                            | `cd frontend && pnpm test`                                                               |
| 导出变更         | `just export`                             | `uv run scripts/tool/export_git_diff.py`                                                 |
| 本地构建         | `just build 0.8.3 x86_64-pc-windows-msvc` | `uv run scripts/build.py -v 0.8.3 -t x86_64-pc-windows-msvc -m local`                    |
| 本地发布         | `just release 0.8.3`                      | `uv run scripts/release.py -v 0.8.3 -m local`                                            |

> `dev` / `build` / `test` / `check` 都必须显式传目标三元组，脚本不会拿宿主三元组兜底。

所有脚本入口均在 `scripts/` 目录下，使用 `uv run` 即可运行，无需手动激活虚拟环境。

### 构建产物

- 本地构建与发布：`release/Local/`
- 正式发布：由 `master` 上的 tag 触发 CI 构建、签名并发布，见 [Git.md](https://github.com/bytecascade-Lee/rollcaller/blob/develop/docs/standards/Git.md) §六

## 配置

应用支持三种运行模式：

| 模式     | 说明                                  |
|:---------|:--------------------------------------|
| 开发模式 | `just dev` 时自动启用，使用开发数据库 |
| 安装模式 | 通过安装包安装，数据位于系统标准目录  |
| 便携模式 | 数据跟随应用目录                      |

## 目录结构

```text
rollcaller/
├─ backend/      Rust 后端（cmd → service → repo 三层）与 Tauri 配置
├─ frontend/     Svelte 5 前端（组件、路由、自动生成的类型）
├─ resources/    运行期资源：帮助文档、图标等
├─ scripts/      构建、发布与代码生成脚本（Python）
├─ docs/         架构与决策文档
└─ justfile      任务入口
```

## 贡献

欢迎提交 [Issue](https://github.com/bytecascade-Lee/rollcaller/issues) 与 [Pull Request](https://github.com/bytecascade-Lee/rollcaller/pulls)。

- 默认分支为 `develop`
- 提交信息遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)；`CHANGELOG.md` 与 `RELEASE_NOTES.md` 的写法见 [docs/standards](https://github.com/bytecascade-Lee/rollcaller/tree/develop/docs/standards)
- 提交 PR 前请确保 `just check <三元组>` 与 `just test <三元组>` 均通过；合入 `develop` / `master` 后 CI 会重新执行一遍（Quality Gate）
- 分支模型与发布流程见 [Git.md](https://github.com/bytecascade-Lee/rollcaller/blob/develop/docs/standards/Git.md)

## 许可证

本项目基于 [MIT](LICENSE) 许可证开源。
