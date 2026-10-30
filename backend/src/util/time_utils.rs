use jiff::Timestamp;
use std::fmt;

/// 规范 ISO 8601 时间字符串
///
/// - UTC
/// - 恒为三位小数
/// - 采用 `Z` 后缀
pub struct ISO8601<'a>(pub &'a Timestamp);

impl fmt::Display for ISO8601<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// 当前时刻的规范 ISO 8601 串
///
/// 与序列化路径同源（都经 [`ISO8601`]），不要另写一套格式串。
pub fn current_iso_8601() -> String {
    ISO8601(&Timestamp::now()).to_string()
}

/// 当前毫秒时间戳
pub fn current_timestamp_millis() -> i64 {
    Timestamp::now().as_millisecond()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn iso8601_is_fixed_width() {
        // 整秒：默认 Display 会输出 `…:00Z`，Iso8601 必须补足 `.000`
        let ts = Timestamp::from_str("2026-10-27T01:34:23Z").unwrap();
        assert_eq!(ISO8601(&ts).to_string(), "2026-10-27T01:34:23.000Z");

        let ts = Timestamp::from_str("2026-10-27T01:34:23.158Z").unwrap();
        assert_eq!(ISO8601(&ts).to_string(), "2026-10-27T01:34:23.158Z");
    }

    /// 非 UTC 输入会被归一化为 UTC —— 这正是定长串能直接比较的前提
    #[test]
    fn iso8601_normalizes_offset() {
        let ts = Timestamp::from_str("2026-10-27T09:34:23.158+08:00").unwrap();
        assert_eq!(ISO8601(&ts).to_string(), "2026-10-27T01:34:23.158Z");
    }

    #[test]
    fn iso8601_round_trips() {
        let ts = Timestamp::from_millisecond(1_700_000_000_123).unwrap();
        let text = ISO8601(&ts).to_string();
        assert_eq!(Timestamp::from_str(&text).unwrap(), ts);
    }

    #[test]
    fn current_iso_8601_matches_canonical_shape() {
        let now = current_iso_8601();
        assert_eq!(now.len(), 24);
        assert!(now.ends_with('Z'));
        assert_eq!(&now[19..20], ".");
        Timestamp::from_str(&now).unwrap();
    }
}
