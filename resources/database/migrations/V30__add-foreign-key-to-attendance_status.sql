PRAGMA foreign_keys = OFF;

-- 统一加 1
UPDATE records
SET attendance_status = records.attendance_status + 1;

-- 验证数据
SELECT COUNT(*) AS orphan_count
FROM records r
WHERE r.attendance_status IS NOT NULL
  AND NOT EXISTS (SELECT 1
                  FROM attendance_status_definition s
                  WHERE s.id = r.attendance_status);

-- 处理孤儿数据
UPDATE records
SET attendance_status = 0
WHERE attendance_status IS NOT NULL
  AND NOT EXISTS (SELECT 1
                  FROM attendance_status_definition s
                  WHERE s.id = attendance_status);

-- 重建表添加外键
CREATE TABLE records_new
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
    FOREIGN KEY (student_id) REFERENCES students (id) ON DELETE RESTRICT,
    FOREIGN KEY (attendance_status) REFERENCES attendance_status_definition (id) ON DELETE RESTRICT
);

-- 复制所有数据
-- 由于未使用 AUTOINCREMENT ，不需要“拨回”计数器
INSERT INTO records_new
SELECT *
FROM records;

-- 删除旧表，重命名新表
DROP TABLE records;
ALTER TABLE records_new
    RENAME TO records;

-- 重建索引
CREATE INDEX IF NOT EXISTS idx_roll_call_at ON records (rollcall_at DESC);
CREATE INDEX IF NOT EXISTS idx_session_id ON records (session_id COLLATE NOCASE);

-- 重建触发器
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

-- 验证新约束
PRAGMA foreign_key_check;

PRAGMA foreign_keys = ON;
