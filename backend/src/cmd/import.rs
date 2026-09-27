use crate::common::entity::import::SheetPreview;
use crate::common::enums::student::StudentBatchCreateResult;
use crate::database::database_pool;
use crate::service::import_service;
use std::collections::HashMap;
use std::path::Path;

/// 预览 Excel 文件：读取每张工作表的前 8 行
#[tauri::command]
pub async fn preview_excel(file_path: String) -> Result<Vec<SheetPreview>, String> {
    let path = Path::new(&file_path);
    import_service::preview(path).map_err(|e| e.to_string())
}

/// 导入 Excel 文件
///
/// `sheet_name` 指定要导入的工作表（按名字，原样回传预览时给出的值）；
/// `student_no_column` / `name_column` 为学号列、姓名列的序号（0 起）。
/// `decisions` 用于处理「已删除但姓名不同」的冲突：
/// - 首次调用传空 map，若返回 `DecisionRequired`，前端展示冲突后由用户决策
/// - 二次调用传入用户决策
#[tauri::command]
pub async fn import_excel(
    file_path: String,
    sheet_name: String,
    header_rows: usize,
    student_no_column: usize,
    name_column: usize,
    decisions: HashMap<String, bool>,
) -> Result<StudentBatchCreateResult, String> {
    let rb = database_pool::database()
        .await
        .map_err(|e| e.to_string())?;
    let path = Path::new(&file_path);
    import_service::import(&*rb, path, &sheet_name, header_rows, student_no_column, name_column, decisions)
        .await
        .map_err(|e| e.to_string())
}
