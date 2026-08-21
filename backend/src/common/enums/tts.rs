use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[ts(export)]
pub enum TtsMode {
    Off,
    SystemNative,
    AIHttp,
    AIEmbedded,
    AICloud,
}
