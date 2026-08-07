use crate::common::entity::student::Student;
use crate::repo::record_repo;
use anyhow::{anyhow, Context};
use jiff::__jcore::bounds::const_check::i8;
use jiff::tz::TimeZone;
use rbatis::RBatis;
use rbs::value;
use rust_xlsxwriter::{Format, FormatAlign, Workbook};
use std::path::Path;

pub async fn export_students(rb: &RBatis, path: &Path, ids: Vec<i64>) -> anyhow::Result<()> {
    if ids.is_empty() {
        return Err(anyhow!("长度为0，没有待导出的学生。"));
    }
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
    let content_format = Format::new().set_font_name("微软雅黑").set_font_size(12);
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

pub async fn export_records(rb: &RBatis, path: &Path, ids: Vec<i64>) -> anyhow::Result<()> {
    if ids.is_empty() {
        return Err(anyhow!("长度为0，没有待导出的历史记录。"));
    }
    if path.exists() {
        return Err(anyhow!("文件已存在：{:#?}", path));
    }
    let mut tx = rb.acquire_begin().await?;
    let records = record_repo::select_by_ids(&mut tx, ids).await?;
    let mut workbook = Workbook::new();
    let header_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_font_name("微软雅黑")
        .set_font_size(14)
        .set_bold();
    let content_format = Format::new().set_font_name("微软雅黑").set_font_size(12);
    let mut worksheet = workbook.add_worksheet();

    worksheet.write_with_format(0, 0, "序号", &header_format)?;
    worksheet.write_with_format(0, 1, "学号", &header_format)?;
    worksheet.write_with_format(0, 2, "姓名", &header_format)?;
    worksheet.write_with_format(0, 3, "状态", &header_format)?;
    worksheet.write_with_format(0, 4, "备注", &header_format)?;
    worksheet.write_with_format(0, 5, "点名时间", &header_format)?;

    for (index, record) in records.iter().enumerate() {
        let row = index as u32 + 1;
        worksheet.write_with_format(row, 0, row, &content_format)?;
        worksheet.write_with_format(row, 1, &record.student_no, &content_format)?;
        worksheet.write_with_format(row, 2, &record.name, &content_format)?;
        worksheet.write_with_format(row, 3, get_status_from_code(record.attendance_status), &content_format)?;
        worksheet.write_with_format(row, 4, record.remark.clone().ok_or("").clone(), &content_format)?;
        worksheet.write_with_format(row, 5, &record.rollcall_at.to_zoned(TimeZone::system()).to_string(), &content_format)?;
    }

    worksheet.autofit();
    workbook.save(path).context("保存失败")?;

    Ok(())
}

fn get_status_from_code(code: i8) -> String {
    match code {
        0 => String::from("缺勤"),
        1 => String::from("出勤"),
        2 => String::from("迟到"),
        3 => String::from("早退"),
        4 => String::from("请假"),
        _ => String::from("未知")
    }
}
