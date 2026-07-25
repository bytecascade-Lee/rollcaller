use crate::common::entity::import::ImportPreviewData;
use crate::common::entity::student::Student;
use crate::common::enums::student::StudentBatchCreateResult;
use crate::service::student_service;
use anyhow::anyhow;
use calamine::{Reader, Xlsx};
use rbatis::RBatis;
use std::collections::HashMap;
use std::path::Path;

/// 预览：读取前 5 行原始数据，不解析表头/列映射
pub fn preview(path: &Path) -> anyhow::Result<ImportPreviewData> {
    let mut workbook: Xlsx<_> = calamine::open_workbook(path)?;
    let sheet = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow!("无法获取第一个工作表"))??;

    let total_rows = sheet.height();
    let total_columns = sheet.width();

    let mut rows_data = Vec::new();
    for (row_idx, row) in sheet.rows().enumerate() {
        if row_idx >= 5 {
            break;
        }
        let row_cells: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
        rows_data.push(row_cells);
    }

    Ok(ImportPreviewData {
        rows: rows_data,
        total_columns,
        total_rows,
    })
}

/// 导入：解析 Excel → 批量创建学生
///
/// `decisions` 用于处理「已删除但姓名不同」的冲突，键为学号，值为 `true`（覆写）/ `false`（保留）。
/// 首次导入传空 map，若返回 `DecisionRequired`，前端展示冲突后由用户决策，二次调用时传入。
pub async fn import(
    rb: &RBatis,
    path: &Path,
    header_rows: usize,
    column_mapping: HashMap<String, i32>,
    decisions: HashMap<String, bool>,
) -> anyhow::Result<StudentBatchCreateResult> {
    let mut workbook: Xlsx<_> = calamine::open_workbook(path)?;
    let sheet = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow!("无法获取第一个工作表"))??;

    let student_no_col = *column_mapping
        .get("student_no")
        .ok_or_else(|| anyhow!("缺少 student_no 的列映射"))?;
    let name_col = *column_mapping.get("name").ok_or_else(|| anyhow!("缺少 name 的列映射"))?;

    let mut students = Vec::new();

    for (row_idx, row) in sheet.rows().enumerate() {
        if row_idx < header_rows {
            continue;
        }

        let student_no = row.get(student_no_col as usize).map(|c| c.to_string()).unwrap_or_default();
        let name = row.get(name_col as usize).map(|c| c.to_string()).unwrap_or_default();

        if student_no.is_empty() && name.is_empty() {
            continue;
        }

        students.push(Student::new(&student_no, &name));
    }

    student_service::batch_create(rb, students, decisions).await
}
