> [en-US](README_en_US.md)

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
- 后端：Rust + Tauri 2.x
- 数据库：SQLite + RBatis
- 样式：Open Props + Phosphor

---

## 开发

### 环境要求

- Node.js 18+
- pnpm 8+
- Rust 1.70+
- Tauri CLI

### 启动开发服务

```bash
# 克隆项目
git clone https://github.com/bytecascade-Lee/rollcaller.git
cd rollcaller

# 安装前端依赖
cd frontend
pnpm install
cd ..

# 启动开发环境
cd ./backend && cargo tauri dev
```


### 构建

```bash
cd backend && cargo tauri build
```

构建产物位于 `backend/target/release/` 目录。


---

## 数据库迁移

迁移文件位于 `resources/migrations/`，命名格式：

```
V{version}__{description}.sql
```

示例：`V1__create_student_table.sql`

应用启动时会自动执行未运行的迁移。

---

## 配置

应用支持三种运行模式：

| 模式     | 说明                                                         |
|----------|--------------------------------------------------------------|
| 开发模式 | `cd ./backend && cargo tauri dev` 时自动启用，使用开发数据库 |
| 安装模式 | 通过安装包安装，数据位于系统标准目录                         |
| 便携模式 | 数据跟随应用目录                                             |

---

## 许可证

[MIT](LICENSE)

---

## 贡献

欢迎提交 [Issue](https://github.com/bytecascade-Lee/rollcaller/issues) 。
