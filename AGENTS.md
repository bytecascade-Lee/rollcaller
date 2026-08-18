# Repository Guidelines

## Project Overview

Classroom roll-call (点名) desktop app built with Tauri v2 + SvelteKit 5 SPA. Backend in Rust with SQLite via RBatis ORM. Frontend in Svelte 5 runes mode, no SSR. IPC via Tauri commands.

---

## Architecture & Data Flow

**Three-layer Rust backend**: `cmd/` → `service/` → `repo/`

```
User action → Svelte component → invoke("command_name", args)
  → cmd/ (thin handler, gets DB pool, delegates)
    → service/ (business logic, transactions)
      → repo/ (SQL via #[py_sql] macros)
      → entity::crud! (auto-generated insert/select_by_map)
```

- **cmd/** — `#[tauri::command]` async fns. Get `database_pool::database().await`, delegate to service, convert errors to `String`.
- **service/** — Business logic, transaction management (acquire_begin/commit/rollback), complex decision trees (soft-delete conflict resolution).
- **repo/** — Raw SQL via `#[py_sql]` macros (MyBatis-style). Simple CRUD uses auto-generated methods from `crud!()` macro on entity structs.
- **common/entity/** — Dual-struct pattern: lightweight for INSERT (`id: Option<i64>`, no metadata), heavyweight for SELECT (`id: i64`, timestamps, soft-delete flags).
- **common/enums/** — Tagged enums (`#[serde(tag="type", content="data")]`) for complex results (StudentSingleCreateResult with 6 variants).

**Key data flow — rollcall**:

1. Frontend passes `student_ids: Vec<i64>` + `session_id: String`
2. Service picks one via `rand::random_range(0..n)`
3. `Record::insert` in a transaction → commit → JOIN query for `RollcallRecord`

---

## Key Directories

| Path                          | Purpose                                                               |
|-------------------------------|-----------------------------------------------------------------------|
| `backend/src/cmd/`            | Tauri command handlers (student, record, rollcall, import, app_paths) |
| `backend/src/service/`        | Business logic                                                        |
| `backend/src/repo/`           | SQL via `#[py_sql]` macros                                            |
| `backend/src/common/entity/`  | Rust structs with `#[ts(export)]` for type generation                 |
| `backend/src/common/enums/`   | Result enums for complex operations                                   |
| `backend/src/database/`       | DB pool singleton + migration bootstrap                               |
| `backend/src/config/`         | App paths, YAML config, logger                                        |
| `backend/src/util/`           | Serde helpers, YAML flatten, time utils                               |
| `backend/capabilities/`       | Tauri permission model                                                |
| `backend/resources/`          | Bundled configs, DB migrations                                        |
| `frontend/src/components/`    | Svelte 5 page components                                              |
| `frontend/src/routes/`        | SvelteKit SPA shell (+layout.svelte)                                  |
| `frontend/src/lib/types/`     | Auto-generated TS types from Rust (`ts-rs`)                           |
| `frontend/src/lib/constants/` | Auto-generated config constants                                       |

---

## Development Commands

```shell
# Run the full Tauri desktop app
cd ./backend && cargo tauri dev

# Check backend compilation only
cd ./backend && cargo check

# Frontend dev server (for UI work without Tauri)
cd ./frontend && pnpm dev

# Frontend type-check
cd ./frontend && pnpm check

# Build for production (Tauri)
cd ./backend && cargo tauri build
```

---

## Code Conventions & Common Patterns

### Backend (Rust)

- **Module organization**: Root-level aggregator files (`cmd.rs`, `service.rs`, `repo.rs`) declare `pub mod submodule;` — no nested `mod.rs` directories.
- **Entity dual pattern**:
    - `Student` / `Record` — lightweight, `id: Option<i64>`, for INSERT
    - `StudentTable` / `RecordTable` — all fields including timestamps + soft-delete, for SELECT
    - `RollcallRecord` — JOIN result type (records + students), not a table
- **Tagged enums**: Use `#[serde(tag = "type", content = "data")]` for richer return values than `Result<T, String>`.
- **Error handling**: Services return `anyhow::Result<T>`. Commands convert to `Result<T, String>` for Tauri IPC.
- **Async**: All service fns are `async`. DB pool is `Arc<RBatis>`.
- **Transactions**: Write operations use `rb.acquire_begin()` → `&mut RBatisTxExecutor` → `commit()`/`rollback()`. Read operations pass `&dyn Executor` or `&RBatis`.
-

**SQL**: Complex queries use `#[py_sql("...")]` macro with `#{param}` binding and `trim`, `for` directives. Simple queries use auto-generated `crud!()` methods: `insert()`, `insert_batch()`, `select_by_map()`, `update_by_map()`.

- **Timestamps**: `jiff::Timestamp`, serialized as millisecond `i64` via custom serde helpers in `util/serde_utils.rs`.
- **Random selection**: `rand::random_range(0..n)` (rand 0.10 API).
- **Imports**: `merge_imports = true` in rustfmt config.
- **Service fns take `rb: &RBatis`** as first param — cmd layer gets the pool and passes it in.

### Frontend (Svelte 5)

- **State management**: Svelte 5 runes exclusively (`$state`, `$derived`, `$effect`). No stores.
- **Component structure**: Single `.svelte` file per page, script first, then template, then scoped `<style>`.
- **Dialog pattern**: Fixed `overlay` + centered `dialog` div, toggled by `$state` string variable (`dialog = "add"` / `dialog = null`).
- **Tauri invoke**: `import { invoke } from "@tauri-apps/api/core"`. Always wrap in try/catch, errors arrive as strings.
- **File dialogs**: `import { open } from "@tauri-apps/plugin-dialog"` — only in StudentPage for Excel import.
- **Types**: Consumed via JSDoc `/** @type {import('$lib/types').TypeName[]} */`. Generated by `ts-rs` from Rust `#[ts(export)]` structs.
- **Path aliases**: `$components/` → `src/components/`, `$lib/` → `src/lib/`. Configured in both `vite.config.js` and `tsconfig.json`.

### DB Schema (`resources/database/migrations/`)

- Flyway-style: `V<version>__<description>.sql`
- Students table: `id`, `student_no`, `name`, timestamps, soft-delete
- Records table: `id`, `student_id` (FK), `attendance_status`, `remark`, `rollcall_at`, `session_id`, timestamps, soft-delete

---

## Important Files

| File                                                        | Role                                  |
|-------------------------------------------------------------|---------------------------------------|
| `backend/src/main.rs` + `lib.rs`                            | App entry + Tauri bootstrap           |
| `backend/src/database/database_pool.rs`                     | DB connection singleton               |
| `backend/src/common/entity/student.rs`                      | Student + StudentTable                |
| `backend/src/common/entity/record.rs`                       | Record + RecordTable + RollcallRecord |
| `backend/src/common/enums/student.rs`                       | Create/update result enums            |
| `backend/src/service/student_service.rs`                    | Core student CRUD + conflict logic    |
| `backend/src/service/rollcall_service.rs`                   | Random pick + save                    |
| `backend/src/service/import_service.rs`                     | Excel import (calamine)               |
| `backend/src/repo/student_repo.rs`                          | py_sql for update/delete/restore      |
| `backend/src/repo/record_repo.rs`                           | py_sql JOIN queries                   |
| `backend/src/cmd/import.rs`                                 | preview_excel + import_excel commands |
| `backend/capabilities/default.json`                         | Tauri permissions                     |
| `backend/resources/database/migrations/V0__init_schema.sql` | Initial schema                        |
| `frontend/src/routes/+layout.svelte`                        | SPA shell with sidebar nav            |
| `frontend/src/pages/StudentManagementPage.svelte`           | Student CRUD + import wizard          |
| `frontend/src/pages/RollcallPage.svelte`                    | Rollcall state machine                |
| `frontend/src/pages/RecordHistoryPage.svelte`               | Past records table                    |

---

## Runtime/Tooling Preferences

| Requirement      | Value                                    |
|------------------|------------------------------------------|
| Rust edition     | 2021                                     |
| Tauri            | v2.11                                    |
| Frontend runtime | SvelteKit SPA (no SSR)                   |
| Package manager  | pnpm                                     |
| Frontend build   | Vite 6                                   |
| Database         | SQLite via RBatis 4.x                    |
| Excel parsing    | calamine                                 |
| Excel writing    | rust_xlsxwriter                          |
| Type bridge      | ts-rs 12 (Rust → TS)                     |
| Date/time        | jiff                                     |
| Random           | rand 0.10 (`random_range`)               |
| Dev server port  | 14650                                    |
| DB path          | `data/data/sqlite-develop.db` (dev mode) |

---

## Testing & QA

- Rust: `cargo test` in `backend/`
- Frontend: `pnpm check` in `frontend/` (runs svelte-kit sync + svelte-check)
- No unit test framework established yet — codebase currently relies on manual smoke testing
- Tauri capabilities/permissions must be updated in `backend/capabilities/default.json` when adding new plugins
- ts-rs types are regenerated at compile time — `cargo check` will regenerate `frontend/src/lib/types/*.ts`
