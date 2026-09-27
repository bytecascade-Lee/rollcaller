use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 单张工作表的预览快照
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SheetPreview {
    /// 工作表名：Excel 内唯一，原样保留（不做 trim / 大小写 / 形式归一）
    pub name: String,
    /// 预览行切片，行数上限见 `service::import_service::PREVIEW_ROWS`；每行长度恒等于 `total_columns`
    pub rows: Vec<Vec<String>>,
    /// 该表真实的行数、列数（与 `rows` 的切片长度无关）
    pub total_rows: usize,
    pub total_columns: usize,
}
