use crate::util::time_utils::ISO8601;
use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

/// 序列化适配器：让 `collect_str` 直接走 [`ISO8601`]，不先落成 `String`
struct ISOStr<'a>(&'a Timestamp);

impl Serialize for ISOStr<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&ISO8601(self.0))
    }
}

/// 用于将 `jiff::Timestamp` 序列化为规范 ISO 8601 时间字符串
pub fn serialize_timestamp_to_iso_8601<S>(timestamp: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&ISO8601(timestamp))
}

/// 用于将 ISO 8601 字符串反序列化为 `jiff::Timestamp`
///
/// 接受任意 RFC 3339 / ISO 8601，统一归一化为 UTC
pub fn deserialize_timestamp_from_iso_8601<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
where
    D: Deserializer<'de>,
{
    let iso8601 = String::deserialize(deserializer)?;
    Timestamp::from_str(&iso8601).map_err(|e| serde::de::Error::custom(e.to_string()))
}

/// 用于将 `Option<jiff::Timestamp>` 序列化为规范 ISO 8601 串或 `null`
pub fn serialize_optional_timestamp_to_iso_8601<S>(
    opt_timestamp: &Option<Timestamp>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match opt_timestamp {
        Some(timestamp) => serializer.serialize_some(&ISOStr(timestamp)),
        None => serializer.serialize_none(),
    }
}

/// 用于将可选的 ISO 8601 字符串反序列化为 `Option<jiff::Timestamp>`
pub fn deserialize_optional_timestamp_from_iso_8601<'de, D>(deserializer: D) -> Result<Option<Timestamp>, D::Error>
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
    use serde::{Deserialize, Serialize};

    /// 测试用的结构体，应用序列化函数
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(deserialize_with = "deserialize_timestamp_from_iso_8601")]
        #[serde(serialize_with = "serialize_timestamp_to_iso_8601")]
        pub ts: Timestamp,

        #[serde(deserialize_with = "deserialize_optional_timestamp_from_iso_8601")]
        #[serde(serialize_with = "serialize_optional_timestamp_to_iso_8601")]
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
            "ts": "2023-11-14T22:13:20.000Z",
            "opt_ts": "2023-11-14T22:13:20.001Z",
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
            "ts": "2023-11-14T22:13:20.000Z",
            "opt_ts": null,
            "flag": 0,
        });
        assert_eq!(json, expected);

        let deserialized: TestStruct = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, original);
    }

    /// 读取必须宽容：无小数、任意小数位、带偏移都要能进
    #[test]
    fn test_deserialize_is_lenient() {
        let cases = [
            ("2023-11-14T22:13:20Z", "2023-11-14T22:13:20.000Z"),
            ("2023-11-14T22:13:20.1Z", "2023-11-14T22:13:20.100Z"),
            ("2023-11-14T22:13:20.123456Z", "2023-11-14T22:13:20.123Z"),
            ("2023-11-15T06:13:20+08:00", "2023-11-14T22:13:20.000Z"),
        ];
        for (input, expected) in cases {
            let json = serde_json::json!({ "ts": input, "opt_ts": null, "flag": 1 });
            let parsed: TestStruct = serde_json::from_value(json).unwrap();
            assert_eq!(ISO8601(&parsed.ts).to_string(), expected, "输入 {input}");
        }
    }

    #[test]
    fn test_deserialize_invalid_bool() {
        let json = serde_json::json!({
            "ts": "2023-11-14T22:13:20.000Z",
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
        // 必填字段非法即报错（不静默降级）
        let json = serde_json::json!({
            "ts": "not-a-date",
            "opt_ts": null,
            "flag": 1,
        });
        assert!(serde_json::from_value::<TestStruct>(json).is_err());

        // 可选字段非法按 None 处理（外部清单容错）
        let json = serde_json::json!({
            "ts": "2023-11-14T22:13:20.000Z",
            "opt_ts": "not-a-date",
            "flag": 1,
        });
        let parsed: TestStruct = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.opt_ts, None);
    }

    /// 旧格式（毫秒整数）不再被接受 —— 这是破坏性变更，故意让它在类型层暴露
    #[test]
    fn test_millisecond_integer_is_rejected() {
        let json = serde_json::json!({
            "ts": 1_700_000_000_000_i64,
            "opt_ts": null,
            "flag": 1,
        });
        assert!(serde_json::from_value::<TestStruct>(json).is_err());
    }
}
