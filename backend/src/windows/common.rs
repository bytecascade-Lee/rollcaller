use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// 窗口种类
///
/// 既是 tauri 的窗口 label，也是导航体系的 host
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum WindowKind {
    App,
    Help,
}

impl WindowKind {
    /// 窗口 label / 路由 host
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Help => "help",
        }
    }

    /// 窗口载入的 HTML 入口
    pub const fn html(self) -> &'static str {
        match self {
            Self::App => "app.html",
            Self::Help => "help.html",
        }
    }

    /// 由窗口 label 或路由 host 反查；未知返回 `None`
    pub fn from_host(host: &str) -> Option<Self> {
        match host {
            "app" => Some(Self::App),
            "help" => Some(Self::Help),
            _ => None,
        }
    }
}

impl fmt::Display for WindowKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::WindowKind;

    const ALL: [WindowKind; 2] = [WindowKind::App, WindowKind::Help];

    /// serde 的值必须与 `as_str` 一致，否则 ts-rs 导出的联合类型对不上路由 host
    #[test]
    fn serde_value_matches_as_str() {
        for kind in ALL {
            let json = serde_json::to_value(kind).expect("枚举应当可序列化");
            assert_eq!(json, kind.as_str(), "serde 与 as_str 不一致");
        }
    }

    #[test]
    fn from_host_roundtrip() {
        for kind in ALL {
            assert_eq!(WindowKind::from_host(kind.as_str()), Some(kind));
            assert_eq!(WindowKind::from_host(&kind.to_string()), Some(kind));
        }
        assert_eq!(WindowKind::from_host("nope"), None);
    }
}
