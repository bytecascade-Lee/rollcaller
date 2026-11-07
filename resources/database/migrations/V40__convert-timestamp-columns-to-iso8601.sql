-- 时间列统一为 ISO 8601 文本
--
-- 注意：
-- 1. 毫秒转文本必须用 `/ 1000.0`，`/ 1000` 是整数除法，会丢失毫秒部分，最终变成 `.000`。
-- 2. `strftime` 对 `NULL` 返回 `NULL`，无需 `COALESCE`。

-- 出勤状态定义表
CREATE TABLE attendance_status_definition_new
(
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL DEFAULT '新状态',
    background TEXT    NOT NULL DEFAULT '#333333',
    color      TEXT    NOT NULL DEFAULT '#f0f0f0',
    remark     TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at TEXT
);

INSERT INTO attendance_status_definition_new (id, name, background, color, remark, is_deleted, deleted_at)
SELECT id,
       name,
       background,
       color,
       remark,
       is_deleted,
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', deleted_at / 1000.0, 'unixepoch')
FROM attendance_status_definition;

DROP TABLE attendance_status_definition;
ALTER TABLE attendance_status_definition_new
    RENAME TO attendance_status_definition;

-- 学生信息表
CREATE TABLE students_new
(
    id         INTEGER PRIMARY KEY,
    student_no TEXT UNIQUE       NOT NULL,
    name       TEXT              NOT NULL,
    created_at TEXT    DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT    DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    is_deleted INTEGER DEFAULT 0 NOT NULL,
    deleted_at TEXT    DEFAULT NULL
);

INSERT INTO students_new (id, student_no, name, created_at, updated_at, is_deleted, deleted_at)
SELECT id,
       student_no,
       name,
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', created_at / 1000.0, 'unixepoch'),
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', updated_at / 1000.0, 'unixepoch'),
       is_deleted,
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', deleted_at / 1000.0, 'unixepoch')
FROM students;

DROP TABLE students;
ALTER TABLE students_new
    RENAME TO students;

-- 点名记录表
CREATE TABLE records_new
(
    id                INTEGER PRIMARY KEY,
    student_id        INTEGER           NOT NULL,
    attendance_status INTEGER           NOT NULL,
    remark            TEXT    DEFAULT NULL,
    rollcall_at       TEXT              NOT NULL,
    session_id        TEXT              NOT NULL,
    created_at        TEXT    DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT    DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    is_deleted        INTEGER DEFAULT 0 NOT NULL,
    deleted_at        TEXT    DEFAULT NULL,
    FOREIGN KEY (student_id) REFERENCES students (id) ON DELETE RESTRICT,
    FOREIGN KEY (attendance_status) REFERENCES attendance_status_definition (id) ON DELETE RESTRICT
);

INSERT INTO records_new (id, student_id, attendance_status, remark, rollcall_at, session_id, created_at, updated_at,
                         is_deleted, deleted_at)
SELECT id,
       student_id,
       attendance_status,
       remark,
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', rollcall_at / 1000.0, 'unixepoch'),
       session_id,
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', created_at / 1000.0, 'unixepoch'),
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', updated_at / 1000.0, 'unixepoch'),
       is_deleted,
       STRFTIME('%Y-%m-%dT%H:%M:%fZ', deleted_at / 1000.0, 'unixepoch')
FROM records;

DROP TABLE records;
ALTER TABLE records_new
    RENAME TO records;

-- 重建索引
CREATE INDEX IF NOT EXISTS idx_student_no ON students (student_no COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_student_name ON students (name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_roll_call_at ON records (rollcall_at DESC);
CREATE INDEX IF NOT EXISTS idx_session_id ON records (session_id COLLATE NOCASE);

-- 重建触发器
CREATE TRIGGER IF NOT EXISTS trg_students_after_update
    AFTER UPDATE
    ON students
BEGIN
    UPDATE students
    SET updated_at = CASE
                         WHEN NEW.is_deleted = OLD.is_deleted
                             -- 未修改 is_deleted 字段
                             THEN STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')
                         ELSE updated_at
        END,
        deleted_at = CASE
                         WHEN NEW.is_deleted = 1 AND OLD.is_deleted = 0
                             -- 执行删除，设为删除时间
                             -- SQLite 的 `%f` 是「秒+毫秒」
                             THEN STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHEN NEW.is_deleted = 0 AND OLD.is_deleted = 1
                             -- 执行恢复，设为 NULL
                             THEN NULL
                         ELSE deleted_at
            END
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_records_after_update
    AFTER UPDATE
    ON records
BEGIN
    UPDATE records
    SET updated_at = CASE
                         WHEN NEW.is_deleted = OLD.is_deleted
                             -- 未修改 is_deleted 字段
                             THEN STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')
                         ELSE updated_at
        END,
        deleted_at = CASE
                         WHEN NEW.is_deleted = 1 AND OLD.is_deleted = 0
                             -- 执行删除，设为删除时间
                             THEN STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHEN NEW.is_deleted = 0 AND OLD.is_deleted = 1
                             -- 执行恢复，设为 NULL
                             THEN NULL
                         ELSE deleted_at
            END
    WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_a_s_d_after_update
    AFTER UPDATE
    ON attendance_status_definition
BEGIN
    UPDATE attendance_status_definition
    SET deleted_at = CASE
                         WHEN NEW.is_deleted = 1 AND OLD.is_deleted = 0
                             -- 执行删除，设为删除时间
                             THEN STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHEN NEW.is_deleted = 0 AND OLD.is_deleted = 1
                             -- 执行恢复，设为 NULL
                             THEN NULL
                         ELSE deleted_at
        END
    WHERE id = NEW.id;
END;
