use crate::common::entity::import::SheetPreview;
use crate::common::entity::student::Student;
use crate::common::enums::student::StudentBatchCreateResult;
use crate::service::student_service;
use anyhow::anyhow;
use calamine::{Data, Range, Reader, Xlsx};
use rbatis::RBatis;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// 单张工作表预览的行数上限
pub const PREVIEW_ROWS: usize = 5;

/// 当前只解析 xlsx；要支持 .xls/.ods 只需换成 `open_workbook_auto` + `Sheets`（同样实现 `Reader`）
type Workbook = Xlsx<BufReader<File>>;

fn open(path: &Path) -> anyhow::Result<Workbook> {
    Ok(calamine::open_workbook(path)?)
}

/// 读表：全流程唯一的取表入口，避免 preview 与 import 各自解析导致表错配
fn read_sheet(workbook: &mut Workbook, name: &str) -> anyhow::Result<Range<Data>> {
    workbook
        .worksheet_range(name)
        .map_err(|e| anyhow!("无法读取工作表「{name}」：{e}"))
}

/// 预览：逐表读取前 `PREVIEW_ROWS` 行原始数据，不解析表头/列映射
///
/// 返回顺序即工作簿内顺序；调用方按名字回传要导入的工作表，故不额外校验名字唯一性（Excel 已保证）。
pub fn preview(path: &Path) -> anyhow::Result<Vec<SheetPreview>> {
    let mut workbook = open(path)?;
    let names = workbook.sheet_names();
    if names.is_empty() {
        return Err(anyhow!("工作簿内没有任何工作表"));
    }

    let mut sheets = Vec::with_capacity(names.len());
    for name in names {
        let sheet = read_sheet(&mut workbook, &name)?;
        let rows = sheet
            .rows()
            .take(PREVIEW_ROWS)
            .map(|row| row.iter().map(|cell| cell.to_string()).collect())
            .collect();
        sheets.push(SheetPreview {
            name,
            rows,
            total_rows: sheet.height(),
            total_columns: sheet.width(),
        });
    }

    Ok(sheets)
}

/// 导入：解析指定工作表 → 批量创建学生
///
/// `decisions` 用于处理「已删除但姓名不同」的冲突，键为学号，值为 `true`（覆写）/ `false`（保留）。
/// 首次导入传空 map，若返回 `DecisionRequired`，前端展示冲突后由用户决策，二次调用时传入。
pub async fn import(
    rb: &RBatis,
    path: &Path,
    sheet_name: &str,
    header_rows: usize,
    student_no_column: usize,
    name_column: usize,
    decisions: HashMap<String, bool>,
) -> anyhow::Result<StudentBatchCreateResult> {
    let mut workbook = open(path)?;
    let sheet = read_sheet(&mut workbook, sheet_name)?;

    let mut students = Vec::new();

    for (row_idx, row) in sheet.rows().enumerate() {
        if row_idx < header_rows {
            continue;
        }

        let student_no = row.get(student_no_column).map(|c| c.to_string()).unwrap_or_default();
        let name = row.get(name_column).map(|c| c.to_string()).unwrap_or_default();

        if student_no.is_empty() && name.is_empty() {
            continue;
        }

        students.push(Student::new(&student_no, &name));
    }

    student_service::batch_create(rb, students, decisions).await
}
