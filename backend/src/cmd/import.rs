use crate::common::entity::import::ImportPreviewData;
use crate::common::enums::student::StudentBatchCreateResult;
use crate::database::database_pool;
use crate::service::import_service;
use std::collections::HashMap;
use std::path::Path;

/// 预览 Excel 文件（读取前 5 行）
#[tauri::command]
pub async fn preview_excel(file_path: String) -> Result<ImportPreviewData, String> {
    let path = Path::new(&file_path);
    import_service::preview(path).map_err(|e| e.to_string())
}

/// 导入 Excel 文件
///
/// `decisions` 用于处理「已删除但姓名不同」的冲突：
/// - 首次调用传空 map，若返回 `DecisionRequired`，前端展示冲突后由用户决策
/// - 二次调用传入用户决策
#[tauri::command]
pub async fn import_excel(
    file_path: String,
    header_rows: usize,
    column_mapping: HashMap<String, i32>,
    decisions: HashMap<String, bool>,
) -> Result<StudentBatchCreateResult, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| e.to_string())?;
    let path = Path::new(&file_path);
    import_service::import(&*rb, path, header_rows, column_mapping, decisions)
        .await
        .map_err(|e| e.to_string())
}
