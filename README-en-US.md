*[简体中文](README.md)*<br>
*This README file was translated by Deepseek v4 flash preview.*

# Rollcaller

A classroom random roll call tool built with Tauri + Svelte.

---

## Features

- **Random Roll Call**: Randomly pick a student from the list, supports single and continuous roll call modes
- **Student Management**: CRUD operations, soft delete and restore, batch operations
- **Attendance Records**: Automatically save roll call results, support batch update of status and remarks
- **Import & Export**: Import student lists from Excel, export student data and attendance records
- **Portable Mode**: Extract and run, data stays within the application directory

---

## Tech Stack

- Frontend: Svelte 5 + TypeScript + Vite + pnpm
- Backend: Rust + Tauri 2.11
- Database: SQLite + RBatis
- Styling: Open Props + Phosphor
- Scripts & Build: Python (managed by uv) + just

---

## Development

### Requirements

| Tool                             | Minimum | Purpose                        |
|:---------------------------------|:--------|:-------------------------------|
| [git](https://git-scm.com/)      | 2.30    | Version control                |
| [uv](https://docs.astral.sh/uv/) | 0.5     | Python and script dependencies |
| [just](https://just.systems/)    | 1.0     | Single task entry point        |
| [Node.js](https://nodejs.org/)   | 22      | Frontend build                 |
| [pnpm](https://pnpm.io/)         | 11      | Frontend dependencies          |
| [Rust](https://rustup.rs/)       | 1.80    | Backend and Tauri              |

The Python interpreter (3.14) is downloaded by `uv sync`, no separate installation needed. `just doctor` checks every row above, plus the repository-level git config and installed dependencies.

### Setup (with just)

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller
just setup
```

`just setup` runs: toolchain check → write repository-level git config → `pnpm install --frozen-lockfile` → `uv sync` → refresh the script import hook. It is idempotent, so it is safe to run again after rebuilding the virtual environment.

The git config it writes (`--local`, never global) is documented in [Git.md](Git.md) §1.1. In particular, `pull.ff=only` governs how pulls behave — do not change it back.

### Setup (without just)

Run the equivalent commands in the same order as `just setup`:

```bash
git clone git@github.com:bytecascade-Lee/rollcaller.git
cd rollcaller

# 1. Repository-level git config
git config --local pull.ff only           # pulls may only fast-forward
git config --local merge.ff false         # merges must create a merge commit
git config --local core.quotepath false   # keep CJK paths unescaped
git config --local fetch.prune true       # prune deleted remote branches

# 2. Python dependencies (creates .venv from uv.lock)
uv sync

# 3. Script import hook
cp scripts/tool/sitecustomize.py .venv/sitecustomize.py

# 4. Frontend dependencies
cd frontend && pnpm install --frozen-lockfile && cd ..
```

`sitecustomize.py` is a CPython startup hook that injects `<repo>/scripts` into `sys.path`, so scripts under `scripts/` can `from cli import git` or `from common.logger import info` directly. `.venv/` is not version-controlled —
**copy it again after rebuilding the virtual environment** (`just setup` does this for you).

> On Windows PowerShell, use `Copy-Item` instead of `cp`.

### Common commands

| Purpose            | With just            | Without just                                             |
|:-------------------|:---------------------|:---------------------------------------------------------|
| Check environment  | `just doctor`        | `uv run scripts/setup.py --check`                        |
| Set up environment | `just setup`         | `uv run scripts/setup.py`                                |
| Start dev server   | `just dev`           | `uv run scripts/dev.py`                                  |
| Frontend only      | `just dev-fe`        | `cd frontend && pnpm dev`                                |
| Type check         | `just check`         | `cd frontend && pnpm check`, `cd backend && cargo check` |
| Format             | `just fmt`           | `cd backend && cargo fmt`                                |
| Test               | `just test`          | `cd frontend && pnpm test`, `cd backend && cargo test`   |
| Local build        | `just build 0.8.3`   | `uv run scripts/build.py -v 0.8.3 -m local`              |
| Local package      | `just release 0.8.3` | `uv run scripts/release.py -v 0.8.3 -m local`            |

Every entry point lives under `scripts/`; call it with `uv run`, no need to activate the virtual environment. `just` is only a shortcut — each recipe in `justfile` maps literally to the right column above.

### Build outputs

- Local builds and packages: `release/Local/`
- Official releases: a tag on `master` triggers CI to build, sign and publish — see [Git.md](Git.md) §6

---

## Database Migrations

Migration files are located in `resources/migrations/`, with the naming format:

```
V{version}__{description}.sql
```

Example: `V1__create-student-table.sql`

The application will automatically run pending migrations on startup.

---

## Configuration

The application supports three runtime modes:

| Mode        | Description                                                          |
|-------------|----------------------------------------------------------------------|
| Development | Automatically enabled by `just dev`, uses a development database     |
| Installed   | Installed via installer package, data stored in system standard dirs |
| Portable    | Data stays within the application directory                          |

---

## License

[MIT](LICENSE)

---

## Contributing

Issues are welcome. Feel free to submit an [Issue](https://github.com/bytecascade-Lee/rollcaller/issues).
