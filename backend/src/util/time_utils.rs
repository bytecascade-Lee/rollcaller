use jiff::Timestamp;

/// 获取当前毫秒时间戳
pub fn current_timestamp_millis() -> i64 {
    Timestamp::now().as_millisecond()
}