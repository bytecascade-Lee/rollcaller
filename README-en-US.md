<!-- help:hide -->
<p align="center">
  <img src="docs/assets/logo.png" width="160" height="160" alt="Rollcaller">
</p>

<h1 align="center">Rollcaller</h1>

<p align="center">Classroom roll call, made automatic — pick a student at random, keep the record.</p>

<p align="center">
  <b>English</b> ｜ <a href="README.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/releases/latest"><img src="https://img.shields.io/github/v/release/bytecascade-Lee/rollcaller?include_prereleases&sort=semver&style=flat-square" alt="Latest release"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/blob/develop/LICENSE"><img src="https://img.shields.io/github/license/bytecascade-Lee/rollcaller?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20x64%20%7C%20ARM64-0078D6?style=flat-square&logo=windows&logoColor=white" alt="Platform">
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/actions/workflows/release.yaml"><img src="https://img.shields.io/github/actions/workflow/status/bytecascade-Lee/rollcaller/release.yaml?style=flat-square&label=release" alt="Release workflow"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/actions/workflows/sync-mirrors.yaml"><img src="https://img.shields.io/github/actions/workflow/status/bytecascade-Lee/rollcaller/sync-mirrors.yaml?style=flat-square&label=sync-mirrors" alt="Mirror sync"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/actions/workflows/quality-gate.yaml"><img src="https://img.shields.io/github/actions/workflow/status/bytecascade-Lee/rollcaller/quality-gate.yaml?style=flat-square&label=quality%20gate" alt="Quality Gate"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/commits/develop"><img src="https://img.shields.io/github/last-commit/bytecascade-Lee/rollcaller?style=flat-square" alt="Last commit"></a>
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/stargazers"><img src="https://img.shields.io/github/stars/bytecascade-Lee/rollcaller?style=flat-square" alt="Stars"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/forks"><img src="https://img.shields.io/github/forks/bytecascade-Lee/rollcaller?style=flat-square" alt="Forks"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/issues"><img src="https://img.shields.io/github/issues/bytecascade-Lee/rollcaller?style=flat-square" alt="Issues"></a>
  <a href="https://github.com/bytecascade-Lee/rollcaller/pulls"><img src="https://img.shields.io/github/issues-pr/bytecascade-Lee/rollcaller?style=flat-square" alt="Pull requests"></a>
</p>

<p align="center">
  <a href="https://github.com/bytecascade-Lee/rollcaller/releases"><img src="https://img.shields.io/github/downloads/bytecascade-Lee/rollcaller/total?style=flat-square" alt="Downloads"></a>
  <a href="https://cnb.cool/ordinary-glory/rollcaller"><img src="https://img.shields.io/badge/mirror-cnb.cool-1A7AF8?style=flat-square" alt="Mirror"></a>
</p>
<!-- help:show -->

A classroom roll call tool for teachers who need random questioning and attendance records: pick a student from the roster, save the result automatically, export the statistics whenever you need them. Everything stays on your machine — no network required.

## Features

| Feature            | Description                                                                                               |
|:-------------------|:----------------------------------------------------------------------------------------------------------|
| Random roll call   | Pick a student at random, one at a time or in a continuous run, with configurable round count and repeats |
| Voice announcement | Reads the picked name aloud, using either local speech or a cloud AI voice                                |
| Student management | Create, edit, delete, soft-delete and restore, search, sort, batch operations                             |
| Batch import       | Import a roster from Excel (`.xlsx` / `.xls`), choose the sheet, duplicates handled automatically         |
| Attendance records | Every pick is persisted, grouped by round, with batch edits for status and remarks                        |
| Data export        | Export students and records to Excel — all rows, the filtered set, or the selection                       |
| Portable mode      | Unzip and run; data lives next to the executable, ready for a USB stick                                   |
| Auto update        | Check and install new versions in-app, no manual download and overwrite                                   |
| Built-in help      | A help window with user guides, full-text search and anchor navigation                                    |

## Download

Get the [latest release](https://github.com/bytecascade-Lee/rollcaller/releases/latest):

| Edition   | File name                                          | Notes                                                           |
|:----------|:---------------------------------------------------|:----------------------------------------------------------------|
| Installer | `rollcaller-<version>-windows-<arch>-setup.exe`    | Standard installation, data stored in the system user directory |
| Portable  | `rollcaller-<version>-windows-<arch>-portable.zip` | Unzip and run, data stays in the program directory              |

- Supports recent Windows 10 and Windows 11, for both x64 and ARM64
- Requires the WebView2 runtime: the installer offers to set it up during installation, while the portable build needs a [manual install](https://developer.microsoft.com/zh-cn/microsoft-edge/webview2/#download-section)
- Every artifact ships with a `.sha256` checksum and a `.sig` signature
- For users in mainland China, the source code and releases are also mirrored to [CNB](https://cnb.cool/ordinary-glory/rollcaller); actual released products are subject to GitHub Releases

## Tech stack

| Layer           | Technology                                                                                |
|:----------------|:------------------------------------------------------------------------------------------|
| Desktop client  | [Tauri 2](https://tauri.app/) (Rust)                                                      |
| Frontend        | [Svelte 5](https://svelte.dev/) + TypeScript + [Vite](https://vite.dev/)                  |
| Data            | SQLite + [RBatis](https://github.com/rbatis/rbatis)                                       |
| Styling         | [Open Props](https://open-props.style/) + [Phosphor Icons](https://phosphoricons.com/)    |
| Scripts & build | Python 3.14 (managed by [uv](https://docs.astral.sh/uv/)) + [just](https://just.systems/) |

## Development

### Requirements

| Tool                             | Minimum | Purpose                        |
|:---------------------------------|:--------|:-------------------------------|
| [git](https://git-scm.com/)      | 2.30    | Version control                |
| [Rust](https://rustup.rs/)       | 1.80    | Backend and Tauri              |
| [Node.js](https://nodejs.org/)   | 22      | Frontend build                 |
| [pnpm](https://pnpm.io/)         | 11      | Frontend dependencies          |
| [uv](https://docs.astral.sh/uv/) | 0.5     | Python and script dependencies |
| [just](https://just.systems/)    | 1.0     | Single task entry point        |

The Python interpreter is downloaded by `uv sync`, no separate installation needed. `just doctor` checks every row above, plus the remote topology, the repository-level git config and installed dependencies.

### Set up with just

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller
just setup
```

Without an SSH key configured, use `https://github.com/bytecascade-Lee/rollcaller.git` instead.

`just setup` runs: toolchain check → normalize remotes (pull source `github`, push target depends on whether you have a fork) → write repository-level git config → fill in `backend/.cargo/config.toml` from its template (left untouched if present) → `pnpm install --frozen-lockfile` → `uv sync` → refresh the script import hook. It is idempotent, so it is safe to run again after rebuilding the virtual environment.

The git config it writes (`--local`, never global) is documented in [Git.md](https://github.com/bytecascade-Lee/rollcaller/blob/develop/Git.md) §1.1. In particular, `pull.ff=only` governs how pulls behave — do not change it back.

### Set up with the Python script

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller
uv run ./scripts/setup.py
```

It relies on the same setup mechanism as the `just` command — `just setup` only wraps this script's invocation into a recipe.

### Set up step by step

Run the equivalent commands one by one, in the same order as `just setup`:

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller

# 1. Normalize remotes: the canonical repo is always named github
#    Full rules for fork / CNB-mirror clones are in Git.md §1.1
git remote rename origin github

# 2. Repository-level git config
git config --local pull.ff only           # pulls may only fast-forward
git config --local merge.ff false         # merges must create a merge commit
git config --local core.quotepath false   # keep CJK paths unescaped
git config --local fetch.prune true       # prune deleted remote branches

# 3. Local cargo config (ts-rs export dir and development database file name)
cp backend/.cargo/config.toml.example backend/.cargo/config.toml

# 4. Python dependencies (creates .venv from uv.lock)
uv sync

# 5. Script import hook
cp scripts/tool/sitecustomize.py .venv/sitecustomize.py

# 6. Frontend dependencies
cd frontend && pnpm install --frozen-lockfile && cd ..
```

`sitecustomize.py` is a hook CPython loads at startup; it injects `<repo>/scripts` into `sys.path`, and **must be copied again after rebuilding the virtual
environment** (`just setup` does it for you).

> On Windows PowerShell, use `Copy-Item` instead of `cp`.

### Common commands

| Purpose             | With just                                 | Without just                                                                             |
|:--------------------|:------------------------------------------|:-----------------------------------------------------------------------------------------|
| Check environment   | `just doctor`                             | `uv run scripts/setup.py --check`                                                        |
| Set up environment  | `just setup`                              | `uv run scripts/setup.py`                                                                |
| Start dev server    | `just dev x86_64-pc-windows-msvc`         | `uv run scripts/dev.py -t x86_64-pc-windows-msvc`                                        |
| Frontend dev only   | `just dev-fe`                             | `cd frontend && pnpm dev`                                                                |
| Type check          | `just check x86_64-pc-windows-msvc`       | `cd frontend && pnpm check`, `cd backend && cargo check --target x86_64-pc-windows-msvc` |
| Format              | `just fmt`                                | `cd backend && cargo fmt`                                                                |
| Test                | `just test x86_64-pc-windows-msvc`        | `uv run scripts/test.py -t x86_64-pc-windows-msvc`                                       |
| Frontend tests only | `just test-fe`                            | `cd frontend && pnpm test`                                                               |
| Export changes      | `just export`                             | `uv run scripts/tool/export_git_diff.py`                                                 |
| Local build         | `just build 0.8.3 x86_64-pc-windows-msvc` | `uv run scripts/build.py -v 0.8.3 -t x86_64-pc-windows-msvc -m local`                    |
| Local release       | `just release 0.8.3`                      | `uv run scripts/release.py -v 0.8.3 -m local`                                            |

> `dev` / `build` / `test` / `check` all require an explicit target triple; the scripts never fall back to the host.

Every entry point lives under `scripts/`; run it with `uv run`, no need to activate the virtual environment.

### Build outputs

- Local builds and releases: `release/Local/`
- Official releases: a tag on `master` triggers CI to build, sign and publish — see [Git.md](https://github.com/bytecascade-Lee/rollcaller/blob/develop/Git.md) §6

## Configuration

The application supports three runtime modes:

| Mode        | Description                                                          |
|:------------|:---------------------------------------------------------------------|
| Development | Automatically enabled by `just dev`, uses a development database     |
| Installed   | Installed via installer package, data stored in system standard dirs |
| Portable    | Data stays within the application directory                          |

## Project layout

```text
rollcaller/
├─ backend/      Rust backend (cmd → service → repo) and Tauri config
├─ frontend/     Svelte 5 frontend (components, routing, generated types)
├─ resources/    Runtime resources: help docs, icons and the like
├─ scripts/      Build, release and codegen scripts (Python)
├─ docs/         Architecture and decision records
└─ justfile      Task entry point
```

## Contributing

[Issues](https://github.com/bytecascade-Lee/rollcaller/issues) and pull requests are welcome.

- The default branch is `develop`
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
- Make sure `just check <triple>` and `just test <triple>` both pass before opening a PR; CI repeats them on every merge into `develop` / `master` (Quality Gate)
- Branch model and release flow: see [Git.md](https://github.com/bytecascade-Lee/rollcaller/blob/develop/Git.md)

## License

Released under the [MIT](LICENSE) license.

---

*This document is a translation of the Chinese [README.md](README.md); the Chinese version is authoritative.*
