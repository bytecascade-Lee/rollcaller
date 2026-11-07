use rbatis::crud;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 出勤状态表
///
/// 含有全部字段
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AttendanceStatus {
    pub id: i8,
    pub name: String,
    pub background: String,
    pub color: String,
    pub remark: Option<String>,

    #[serde(with = "crate::util::serde_utils::bool_i64")]
    #[ts(type = "number")]
    pub is_deleted: bool,

    #[serde(with = "crate::util::serde_utils::optional_timestamp_iso8601")]
    #[ts(type = "string | null")]
    pub deleted_at: Option<jiff::Timestamp>,
}

crud!(AttendanceStatus {}, "attendance_status_definition");
