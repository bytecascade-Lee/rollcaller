-- 将迁移历史中V0版本的description改为“initialize schema”
UPDATE migration_history
SET description = 'initialize schema'
WHERE version == 0;
