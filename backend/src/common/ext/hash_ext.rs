//! 哈希扩展
//!
//! 统一以 `AsRef<[u8]>` 为约束，`str`/`String`/`[u8]`/`Vec<u8>` 等类型均可直接调用。

use sha2::{Digest, Sha256};

/// sha256 便捷扩展
///
/// - [`sha256`](HashExt::sha256):十六进制小写字符串;
/// - [`sha256_bytes`](HashExt::sha256_bytes):原始 32 字节。
pub trait HashExt: AsRef<[u8]> {
    /// sha256 十六进制编码
    fn sha256(&self) -> String {
        hex::encode(self.sha256_bytes())
    }

    /// sha256 原始字节，恒为 32 字节
    fn sha256_bytes(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(&Sha256::digest(self.as_ref()));
        out
    }
}

impl<T: AsRef<[u8]> + ?Sized> HashExt for T {}
