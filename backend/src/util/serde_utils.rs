use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serializer};
use std::str::FromStr;

/// 用于将 `i64` 类型的Unix毫秒时间戳反序列化为 `jiff::Timestamp`
pub fn deserialize_timestamp_from_millisecond_i64<'de, D>(
    deserializer: D
) -> Result<Timestamp, D::Error>
where
    D: Deserializer<'de>,
{
    Timestamp::from_millisecond(i64::deserialize(deserializer)?)
        .map_err(|e| serde::de::Error::custom(e.to_string()))
}

/// 用于将 `jiff::Timestamp` 序列化为 `i64` 类型的Unix毫秒时间戳
pub fn serialize_timestamp_to_millisecond_i64<S>(
    timestamp: &Timestamp,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(timestamp.as_millisecond())
}

/// 用于将 `Opting<i64>` 类型的Unix毫秒时间戳反序列化为 `Option<jiff::Timestamp>`
pub fn deserialize_optional_timestamp_from_millisecond_i64<'de, D>(
    deserializer: D,
) -> Result<Option<Timestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(deserializer)? {
        Some(millis) => {
            Timestamp::from_millisecond(millis)
                .map(Some)
                .map_err(|e| serde::de::Error::custom(e.to_string()))
        }
        None => Ok(None),
    }
}

/// 用于将 `Option<jiff::Timestamp>` 序列化为 `Opting<i64>` 类型的Unix毫秒时间戳反
pub fn serialize_optional_timestamp_to_millisecond_i64<S>(
    opt_timestamp: &Option<Timestamp>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match opt_timestamp {
        Some(timestamp) => {
            serializer.serialize_some(&timestamp.as_millisecond())
        }
        None => serializer.serialize_none(),
    }
}

pub fn deserialize_optional_timestamp_from_iso_8601<'de, D>(
    deserializer: D,
) -> Result<Option<Timestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    // 反序列化为 String（而非借用 &str）：兼容来自 owned Value（serde_json::from_value）的输入；
    // 非法日期按 None 处理（容错，不阻塞整条清单解析）
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(iso8601) => Ok(Timestamp::from_str(&iso8601).ok()),
        None => Ok(None),
    }
}

pub fn serialize_optional_timestamp_to_iso_8601<S>(
    opt_timestamp: &Option<Timestamp>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    match opt_timestamp {
        Some(timestamp) => serializer.serialize_some(&timestamp.to_string()),
        None => serializer.serialize_none(),
    }
}

/// 用于将 `i64` 类型的`0`或`1`反序列化为 `bool`
pub fn deserialize_bool_from_i64<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value: i64 = i64::deserialize(deserializer)?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(serde::de::Error::custom(format!(
            "invalid boolean value: {}, expected 0 or 1",
            value
        ))),
    }
}

/// 用于将 `bool` 序列化为 `i64` 类型的`0`或`1`
pub fn serialize_bool_to_i64<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let int_value = if *value { 1 } else { 0 };
    serializer.serialize_i64(int_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    /// 测试用的结构体，应用序列化函数
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(deserialize_with = "deserialize_timestamp_from_millisecond_i64")]
        #[serde(serialize_with = "serialize_timestamp_to_millisecond_i64")]
        pub ts: Timestamp,

        #[serde(deserialize_with = "deserialize_optional_timestamp_from_millisecond_i64")]
        #[serde(serialize_with = "serialize_optional_timestamp_to_millisecond_i64")]
        pub opt_ts: Option<Timestamp>,

        #[serde(deserialize_with = "deserialize_bool_from_i64")]
        #[serde(serialize_with = "serialize_bool_to_i64")]
        pub flag: bool,
    }

    #[test]
    fn test_struct_round_trip() {
        let original = TestStruct {
            ts: Timestamp::from_millisecond(1_700_000_000_000).unwrap(),
            opt_ts: Some(Timestamp::from_millisecond(1_700_000_000_001).unwrap()),
            flag: true,
        };

        // 序列化 -> JSON（这里会调用 serialize_with 函数）
        let json = serde_json::to_value(&original).unwrap();

        let expected = serde_json::json!({
            "ts": 1_700_000_000_000_i64,
            "opt_ts": 1_700_000_000_001_i64,
            "flag": 1,
        });
        assert_eq!(json, expected);

        // 反序列化回来（这里会调用 deserialize_with 函数）
        let deserialized: TestStruct = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_struct_with_none() {
        let original = TestStruct {
            ts: Timestamp::from_millisecond(1_700_000_000_000).unwrap(),
            opt_ts: None,
            flag: false,
        };

        let json = serde_json::to_value(&original).unwrap();
        let expected = serde_json::json!({
            "ts": 1_700_000_000_000_i64,
            "opt_ts": null,
            "flag": 0,
        });
        assert_eq!(json, expected);

        let deserialized: TestStruct = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_deserialize_invalid_bool() {
        let json = serde_json::json!({
            "ts": 1_700_000_000_000_i64,
            "opt_ts": null,
            "flag": 42,  // 非法值
        });

        let result: Result<TestStruct, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid boolean value: 42"));
    }

    #[test]
    fn test_deserialize_invalid_timestamp() {
        // 测试反序列化非法时间戳（超出范围）
        let json = serde_json::json!({
            "ts": i64::MAX,  // 超出 jiff 支持范围
            "opt_ts": null,
            "flag": 1,
        });

        let result: Result<TestStruct, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
