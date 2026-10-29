use jiff::Timestamp;

/// 获取当前毫秒时间戳
pub fn current_timestamp_millis() -> i64 {
    Timestamp::now().as_millisecond()
}

pub fn current_iso_8601() -> String {
    Timestamp::now().strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}
