use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateSource {
    GITHUB,
    CNB,
}

/// 幅度门槛：最少需要多大的数字变化才触发更新。
///
/// 由 `UpdateLevel` 承载：`None` 表示禁止所有更新。
///
/// | 门槛 | 1.2.1→1.2.2 | 1.2.1→1.3.0 | 1.2.1→2.0.0 |
/// |---|---|---|---|
/// | Major | 跳过 | 跳过 | 更新 |
/// | Minor | 跳过 | 更新 | 更新 |
/// | Patch | 更新 | 更新 | 更新 |
/// | Never | 跳过 | 跳过 | 跳过 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateLevel {
    /// 仅主版本
    Major,
    /// 次版本及以上
    Minor,
    /// 补丁及以上
    Patch,
    /// 停止更新
    Never,
}

/// 发布渠道：决定版本列表的可见性。
///
/// - **Stable**：只接收稳定版。latest 为预发布版本时一律跳。
/// - **Prerelease**：接收预发布版本，且预发布相关路径有两条特殊规则：
///   - 「预发布 → 预发布」是**通道内递进**（如 1.2.0-rc.1 → 1.2.0-rc.2）：进入通道时风险已被接受，跟随通道、不再询问，**不适用幅度门槛**；
///   - 「预发布 → 稳定版」是**逃逸通道**（如 1.2.0-rc.1 → 1.2.0）：始终更新，不适用幅度门槛与通道，否则用户会永远卡在预发布版本上。
///
/// 而「稳定版 → 预发布」是**通道切换**（如 1.2.0 → 1.3.0-rc.1）：需要通道为 Prerelease才可见，且仍按幅度门槛判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum UpdateChannel {
    /// 正式版
    Stable,
    /// 预发布版
    Prerelease,
}

/// 更新严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 普通更新：受用户设置的更新策略约束
    #[default]
    Normal,
    /// 重要更新：豁免用户的幅度门槛，通知所有用户
    Important,
    /// 紧急更新：豁免用户的幅度门槛，如不升级则无法使用
    /// 唯一可配合 `force=true` 的档位
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub enum UpdateDecision {
    /// 不更新
    Skip,
    /// 更新
    Update,
}

/// 安装形态，前端展示用（序列化为 `"nsis"` | `"portable"`，命令契约）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum UpdateKind {
    /// NSIS 安装包
    Nsis,
    /// 便携版 zip
    Portable,
}
