use jiff;
use rbatis::crud;
use rbs;
use serde::{de::Unexpected::Option, Deserialize, Serialize};
use ts_rs::TS;

/// Record实体
///
/// 仅含自增主键 + 业务字段，用于 INSERT
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct Record {
    pub id: Option<i64>,
    pub student_id: i64,
    pub attendance_status: i64,
    pub remark: Option<String>,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    pub roll_call_at: jiff::Timestamp,
    pub session_id: String,
}

impl Record {
    pub fn new(
        student_id: i64,
        attendance_status: i64,
        remark: Option<&str>,
        roll_call_at: jiff::Timestamp,
        session_id: &str,
    ) -> Self {
        Self {
            id: None,
            student_id,
            attendance_status,
            remark: match remark {
                Some(_) => Some(remark.to_string()),
                None => None,
            },
            roll_call_at,
            session_id: session_id.to_string(),
        }
    }
}

/// Record表
///
/// 包含全部字段，用于 SELECT 返回给前端
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RecordTable {
    pub id: i64,
    pub student_id: i64,
    pub student_no: String,
    pub name: String,
    pub attendance_status: i64,
    pub remark: Option<String>,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub roll_call_at: jiff::Timestamp,

    pub session_id: String,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_bool_from_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_bool_to_i64")]
    #[ts(type = "number")]
    pub student_is_deleted: bool,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub created_at: jiff::Timestamp,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub updated_at: jiff::Timestamp,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_bool_from_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_bool_to_i64")]
    #[ts(type = "number")]
    pub is_deleted: bool,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub deleted_at: Option<jiff::Timestamp>,
}

crud!(RecordTable {}, "records");
crud!(Record {}, "records");
