> Translated by Deepseek v4 flash preview
> [简体中文](README.md)

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
- Backend: Rust + Tauri 2.x
- Database: SQLite + RBatis
- Styling: Open Props + Phosphor

---

## Development

### Requirements

- Node.js 18+
- pnpm 8+
- Rust 1.70+
- Tauri CLI

### Start Dev Server

```bash
# Clone the repository
git clone https://github.com/bytecascade-Lee/rollcaller.git
cd rollcaller

# Install frontend dependencies
cd frontend
pnpm install
cd ..

# Start the development environment
cd ./backend && cargo tauri dev
```

### Build

```bash
cd backend && cargo tauri build
```

Build artifacts are located in `backend/target/release/`.

---

## Database Migrations

Migration files are located in `resources/migrations/`, with the naming format:

```
V{version}__{description}.sql
```

Example: `V1__create_student_table.sql`

The application will automatically run pending migrations on startup.

---

## Configuration

The application supports three runtime modes:

| Mode          | Description                                                                              |
|---------------|------------------------------------------------------------------------------------------|
| Development   | Automatically enabled when running `cargo tauri dev`, uses a development database        |
| Installed     | Installed via installer package, data stored in system standard directories              |
| Portable      | Data stays within the application directory                                              |

---

## License

[MIT](LICENSE)

---

## Contributing

Issues are welcome. Feel free to submit an [Issue](https://github.com/bytecascade-Lee/rollcaller/issues).
