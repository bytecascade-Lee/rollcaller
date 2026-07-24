-- 学生信息表
CREATE TABLE IF NOT EXISTS students
(
    id         INTEGER PRIMARY KEY,
    student_no TEXT UNIQUE       NOT NULL,
    name       TEXT              NOT NULL,
    created_at INTEGER DEFAULT (CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)),
    updated_at INTEGER DEFAULT (CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)),
    is_deleted INTEGER DEFAULT 0 NOT NULL,
    deleted_at INTEGER DEFAULT NULL
);

-- 点名记录表
CREATE TABLE IF NOT EXISTS records
(
    id                INTEGER PRIMARY KEY,
    student_id        INTEGER           NOT NULL,
    attendance_status INTEGER           NOT NULL,
    remark            TEXT    DEFAULT NULL,
    rollcall_at       INTEGER           NOT NULL,
    session_id        TEXT              NOT NULL,
    created_at        INTEGER DEFAULT (CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)),
    updated_at        INTEGER DEFAULT (CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)),
    is_deleted        INTEGER DEFAULT 0 NOT NULL,
    deleted_at        INTEGER DEFAULT NULL,
    FOREIGN KEY (student_id) REFERENCES students (id) ON DELETE CASCADE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_student_no ON students (student_no COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_student_name ON students (name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_roll_call_at ON records (rollcall_at DESC);
CREATE INDEX IF NOT EXISTS idx_session_id ON records (session_id COLLATE NOCASE);

-- 创建触发器
CREATE TRIGGER IF NOT EXISTS trg_students_after_update
    AFTER UPDATE
    ON students
BEGIN
    UPDATE students
    SET updated_at = CASE
                         WHEN NEW.is_deleted = OLD.is_deleted
                             -- 未修改 is_deleted 字段
                             THEN CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)
                         ELSE updated_at
        END,
        deleted_at = CASE
                         WHEN NEW.is_deleted = 1 AND OLD.is_deleted = 0
                             -- 执行删除，设为删除时间
                             THEN CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)
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
                             THEN CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)
                         ELSE updated_at
        END,
        deleted_at = CASE
                         WHEN NEW.is_deleted = 1 AND OLD.is_deleted = 0
                             -- 执行删除，设为删除时间
                             THEN CAST(UNIXEPOCH('subsec') * 1000 AS INTEGER)
                         WHEN NEW.is_deleted = 0 AND OLD.is_deleted = 1
                             -- 执行恢复，设为 NULL
                             THEN NULL
                         ELSE deleted_at
            END
    WHERE id = NEW.id;
END;
