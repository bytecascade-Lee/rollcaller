use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use ts_rs::TS;

// `MacOS` 是平台名固定拼法，不因为以枚举名 `OS` 结尾就改名
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum OS {
    Windows,
    MacOS,
    Linux,
}

impl fmt::Display for OS {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            OS::Windows => "Windows",
            OS::MacOS => "MacOS",
            OS::Linux => "Linux",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X86_64,
    Arm64,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Arch::X86_64 => "X86_64",
            Arch::Arm64 => "Arm64",
        };
        write!(f, "{}", s)
    }
}
