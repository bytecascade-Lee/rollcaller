//! 自定义 serde 适配器

use crate::util::time_utils::ISO8601;
use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

/// `Timestamp` <-> 规范 ISO 8601
pub mod timestamp_iso8601 {
    use super::*;

    pub fn serialize<S>(value: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&ISO8601(value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Timestamp::from_str(&text).map_err(serde::de::Error::custom)
    }
}

/// `Option<Timestamp>` <-> 规范 ISO 8601 或 `null`
pub mod optional_timestamp_iso8601 {
    use super::*;

    struct ISOStr<'a>(&'a Timestamp);

    impl Serialize for ISOStr<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_str(&ISO8601(self.0))
        }
    }

    pub fn serialize<S>(value: &Option<Timestamp>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(timestamp) => serializer.serialize_some(&ISOStr(timestamp)),
            None => serializer.serialize_none(),
        }
    }

    /// 非法时间按 `None` 处理
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Timestamp>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = Option::<String>::deserialize(deserializer)?;
        Ok(text.and_then(|text| Timestamp::from_str(&text).ok()))
    }
}

/// `bool` <-> `0` / `1`
pub mod bool_i64 {
    use super::*;

    pub fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(i64::from(*value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        match i64::deserialize(deserializer)? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(serde::de::Error::custom(format!(
                "invalid boolean value: {other}, expected 0 or 1"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// 测试用的结构体，应用三个适配器
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(with = "timestamp_iso8601")]
        pub ts: Timestamp,

        #[serde(with = "optional_timestamp_iso8601")]
        pub opt_ts: Option<Timestamp>,

        #[serde(with = "bool_i64")]
        pub flag: bool,
    }

    #[test]
    fn test_struct_round_trip() {
        let original = TestStruct {
            ts: Timestamp::from_millisecond(1_700_000_000_000).unwrap(),
            opt_ts: Some(Timestamp::from_millisecond(1_700_000_000_001).unwrap()),
            flag: true,
        };

        let json = serde_json::to_value(&original).unwrap();
        let expected = serde_json::json!({
            "ts": "2023-11-14T22:13:20.000Z",
            "opt_ts": "2023-11-14T22:13:20.001Z",
            "flag": 1,
        });
        assert_eq!(json, expected);

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
        // 必填字段非法即报错
        let json = serde_json::json!({
            "ts": "not-a-date",
            "opt_ts": null,
            "flag": 1,
        });
        assert!(serde_json::from_value::<TestStruct>(json).is_err());

        // 可选字段非法按 None 处理
        let json = serde_json::json!({
            "ts": "2023-11-14T22:13:20.000Z",
            "opt_ts": "not-a-date",
            "flag": 1,
        });
        let parsed: TestStruct = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.opt_ts, None);
    }

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
