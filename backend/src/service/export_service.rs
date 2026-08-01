use crate::common::entity::student::Student;
use anyhow::{anyhow, Context};
use rbatis::RBatis;
use rbs::value;
use rust_xlsxwriter::{Format, FormatAlign, Workbook};
use std::path::Path;

pub async fn export_students(rb: &RBatis, path: &Path, ids: Vec<i64>) -> anyhow::Result<()> {
    if path.exists() {
        return Err(anyhow!("文件已存在：{:#?}", path));
    }

    let mut students = Student::select_by_map(rb, value! {"id": ids}).await?;
    let mut workbook = Workbook::new();
    let header_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_font_name("微软雅黑")
        .set_font_size(14)
        .set_bold();
    let content_format = Format::new()
        .set_font_name("微软雅黑")
        .set_font_size(12);
    let mut worksheet = workbook.add_worksheet();

    worksheet.write_with_format(0, 0, "序号", &header_format)?;
    worksheet.write_with_format(0, 1, "学号", &header_format)?;
    worksheet.write_with_format(0, 2, "姓名", &header_format)?;

    for (index, student) in students.iter().enumerate() {
        let row = index as u32 + 1;
        worksheet.write_with_format(row, 0, row, &content_format)?;
        worksheet.write_with_format(row, 1, &student.student_no, &content_format)?;
        worksheet.write_with_format(row, 2, &student.name, &content_format)?;
    }

    worksheet.autofit();
    workbook.save(path).context("保存失败")?;

    Ok(())
}
