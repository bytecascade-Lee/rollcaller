use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use ts_rs::TS;

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
