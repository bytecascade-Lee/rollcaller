-- 出勤状态定义表
CREATE TABLE IF NOT EXISTS attendance_status_definition
(
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL DEFAULT '新状态',
    background TEXT    NOT NULL DEFAULT '#333333',
    color      TEXT    NOT NULL DEFAULT '#f0f0f0',
    remark     TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER
);

CREATE TRIGGER IF NOT EXISTS trg_a_s_d_after_update
    AFTER UPDATE
    ON attendance_status_definition
BEGIN
    update attendance_status_definition
    SET deleted_at = CASE
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

-- 添加默认值
INSERT INTO attendance_status_definition (id, name, background, color)
VALUES (0, 'FALLBACK', '#333333', '#f0f0f0'),
       (1, '缺勤', '#e03131', '#ffffff'),
       (2, '出勤', '#37b24d', '#ffffff'),
       (3, '迟到', '#e8590c', '#ffffff'),
       (4, '早退', '#b8860b', '#ffffff'),
       (5, '请假', '#1971c2', '#ffffff');
