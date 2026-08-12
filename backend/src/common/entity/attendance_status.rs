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

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_bool_from_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_bool_to_i64")]
    #[ts(type = "number")]
    pub is_deleted: bool,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number | null")]
    pub deleted_at: Option<jiff::Timestamp>,
}

crud!(AttendanceStatus {}, "attendance_status_definition");
