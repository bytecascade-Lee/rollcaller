use jiff::Timestamp;
use rbatis::crud;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Student实体
///
/// 仅含自增主键 + 业务字段，用于 INSERT
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, TS)]
#[ts(export)]
pub struct Student {
    /// INSERT 时传 None 由 SQLite 自增
    /// UPDATE 时传 Some(id)
    pub id: Option<i64>,
    pub student_no: String,
    pub name: String,
}

impl Student {
    pub fn new(student_no: &str, name: &str) -> Self {
        Self {
            id: None,
            student_no: student_no.to_string(),
            name: name.to_string(),
        }
    }
}

/// Student表
///
/// 包含全部字段，用于 SELECT 返回给前端
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StudentTable {
    pub id: i64,
    pub student_no: String,
    pub name: String,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub created_at: Timestamp,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_timestamp_to_millisecond_i64")]
    #[ts(type = "number")]
    pub updated_at: Timestamp,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_bool_from_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_bool_to_i64")]
    #[ts(type = "number")]
    pub is_deleted: bool,

    #[serde(deserialize_with = "crate::util::serde_utils::deserialize_optional_timestamp_from_millisecond_i64")]
    #[serde(serialize_with = "crate::util::serde_utils::serialize_optional_timestamp_to_millisecond_i64")]
    #[ts(type = "number | null")]
    pub deleted_at: Option<Timestamp>,
}

crud!(Student {}, "students");
crud!(StudentTable {}, "students");
