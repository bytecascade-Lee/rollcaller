use sha2::Digest;

pub trait HashExt {
    fn sha256(&self) -> String;
}

impl HashExt for str {
    fn sha256(&self) -> String {
        hex::encode(sha2::Sha256::digest(self.as_bytes()))
    }
}

impl HashExt for [u8] {
    fn sha256(&self) -> String {
        hex::encode(sha2::Sha256::digest(self))
    }
}