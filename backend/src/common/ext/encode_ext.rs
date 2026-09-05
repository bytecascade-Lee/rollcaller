//! base64 编解码扩展
//!
//! 以 `AsRef<[u8]>` 为约束，`str`/`String`/`[u8]`/`Vec<u8>` 等类型均可直接调用。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::error::Error;

/// base64 便捷扩展
///
/// - [`base64_encode`](Base64Ext::base64_encode):编码为 STANDARD base64 字符串;
/// - [`base64_decode`](Base64Ext::base64_decode):解码回原始字节,输入非法时返回错误。
pub trait Base64Ext: AsRef<[u8]> {
    /// base64 编码(STANDARD,带填充)
    fn base64_encode(&self) -> String {
        STANDARD.encode(self.as_ref())
    }

    /// base64 解码为 String，失败时返回 [`Error`]
    fn base64_decode(&self) -> Result<String, Box<dyn Error>> {
        let bytes = self.base64_decode_bytes()?;
        Ok(String::from_utf8(bytes)?)
    }

    /// base64 解码为 Vec<u8>，输入不是合法 base64 时返回 [`DecodeError`](base64::DecodeError)
    fn base64_decode_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        STANDARD.decode(self.as_ref())
    }
}

impl<T: AsRef<[u8]> + ?Sized> Base64Ext for T {}
