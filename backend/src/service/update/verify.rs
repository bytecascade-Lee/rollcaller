//! 下载产物的签名验证（minisign / Ed25519）与 sha256 完整性校验
//!
//! # 双层内容约定
//!
//! `manifest.signature` 与公钥文件 [`ROLLCALLER_UPDATE_PUBKEY`] 的原始内容都是 **base64(minisign 文本)**：
//!
//! - minisign 的 `.sig` 文件全文（含 `untrusted comment:` 头）经 base64 编码后存入 [`Artifact::signature`]；
//! - minisign 的 `.pub` 文件全文经 base64 编码后作为公钥内容。
//!
//! 因此验签前必须先用 STANDARD base64 解码，得到 minisign 文本后再交给 `minisign-verify` 解析。
//!
//! 注意：tauri-cli ≥2.11 的 `signer generate/sign` **产出的 `.pub` / `.sig` 文件本身就已
//! 是 base64(minisign 文本)**（与本模块约定的清单字段同格式），可直接复制/填入，无需再编码；
//! 旧版 CLI 产出明文 minisign 文本，才需要手动 base64。参见下方互操作步骤。
//!
//! # 签名必填（fail closed）
//!
//! 走本模块组合入口（[`verify_artifact`] / [`verify_artifact_path`]）的产物**必须带签名**：
//! `signature` 为空即拒绝，不再"空则跳过 minisign、只验 sha256"。
//!
//! 原因：清单（含其 `sha256` / `size` / `url`）本身没有认证，唯一能证明"这些字节确实是发布方
//! 发布的"的东西就是 minisign 签名——没有私钥无法伪造。若允许空签名跳过验签，则一份被篡改的
//! 清单（例如本地 `cache/update/{source}/{version}.json` 被改写、`signature` 清空）
//! 就能把 `sha256` 换成攻击者自己的产物并一路通过校验，整条信任链失效。
//!
//! 因此"只验完整性"保留为**独立原语** [`verify_sha256`]，供确实无签名的调用方（Go updater
//! 更新器：扁平清单、仅 sha256）直接使用，不再经由组合入口的空签名分支。
//!
//! # 互操作
//!
//! 1. 生成密钥对：`tauri signer generate --ci -p <密码> -w <名称>.key`，产出 `<名称>.key` 与 `<名称>.key.pub`；
//! 2. 将 `<名称>.key.pub` 的**完整内容**（已是 base64(minisign 公钥文本)）赋值给 [`ROLLCALLER_UPDATE_PUBKEY`]；
//! 3. 对产物签名：`tauri signer sign <文件> -f <名称>.key -p <密码>`，产出 `<文件>.sig`；
//! 4. 将 `<文件>.sig` 的**完整内容**（已是 base64(minisign 签名文本)）填入清单的 `signature` 字段；
//! 5. 运行互操作测试 `cargo test --lib service::update::verify::interop_with_tauri_signer` 验证（本机无 tauri CLI 时自动跳过并打印提示，此时可依上述步骤手动验证）。

use crate::common::constant::secrets::ROLLCALLER_UPDATE_PUBKEY;
use crate::common::entity::update::Artifact;
use crate::common::ext::encode_ext::Base64Ext;
use crate::common::ext::hash_ext::HashExt;
use anyhow::anyhow;
use base64::Engine;
use sha2::Digest;
use std::path::Path;

/// 与官方插件 verify_signature 逐行一致：先解 base64，再解析 minisign 文本并验证
///
/// - `release_signature`：base64(minisign 签名文本)，即 `Artifact.signature`
/// - `pub_key_b64`：base64(minisign 公钥文本)，即 [`ROLLCALLER_UPDATE_PUBKEY`]
pub fn verify_signature(
    data: &[u8],
    release_signature: &str,
    pub_key_b64: &str,
) -> anyhow::Result<()> {
    let public_key = minisign_verify::PublicKey::decode(
        &pub_key_b64
            .base64_decode()
            .map_err(|e| anyhow!("签名公钥 base64 解码失败：{e} / {pub_key_b64}"))?,
    )
        .map_err(|e| {
            anyhow!("minisign 公钥解析失败（外层解码后应为两行：untrusted comment 行 + 42 字节公钥 base64）：{e}")
        })?;
    let signature = minisign_verify::Signature::decode(
        &release_signature
            .base64_decode()
            .map_err(|e| anyhow!("签名 base64 解码失败：{e} / {release_signature}"))?,
    )
        .map_err(|e| {
            anyhow!("minisign 签名解析失败（外层解码后应为四行 minisign 签名文本，当前行数或载荷长度不符）：{e}")
        })?;
    // true = 兼容 legacy 预哈希签名
    public_key.verify(data, &signature, true)?;
    Ok(())
}

/// sha256 十六进制（小写），与 `manifest.sha256` 比较
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> anyhow::Result<()> {
    let actual = data.sha256();
    if actual != expected_hex.to_ascii_lowercase() {
        anyhow::bail!("sha256 不匹配: 期望 {expected_hex}, 实际 {actual}");
    }
    Ok(())
}

/// 主包产物签名必填前置（fail closed）
///
/// `signature` 为空即拒绝。旧语义"空签名 = 该产物无签名、只验 sha256"已废弃——清单自身未经
/// 认证，允许空签名等于把 `sha256` 的控制权交给任何能改写清单缓存的人；而签名没有私钥无法伪造，
/// 因此"签名必填"是本模块唯一的真实性保证。详见模块文档「签名必填（fail closed）」。
///
/// 确实无签名的调用方（Go updater 更新器）请**直接使用** [`verify_sha256`]，
/// 不要经由 [`verify_artifact`] / [`verify_artifact_path`]。
fn ensure_signed(artifact: &Artifact) -> anyhow::Result<()> {
    if artifact.signature.trim().is_empty() {
        anyhow::bail!("清单缺少产物签名（signature 为空），拒绝校验：更新产物必须带 minisign 签名")
    }
    Ok(())
}

/// 下载字节的统一校验入口（bytes 形态）：签名必填 → sha256（完整性）→ minisign 签名（真实性）
pub fn verify_artifact(data: &[u8], artifact: &Artifact) -> anyhow::Result<()> {
    ensure_signed(artifact)?;
    verify_sha256(data, &artifact.sha256)?;
    verify_signature(data, &artifact.signature, active_pubkey())?;
    Ok(())
}

/// 已落盘文件的统一校验入口（path 形态）：签名必填 → sha256 + minisign 签名
///
/// # 内存语义
/// minisign-verify 仅支持整段字节（`&[u8]`），签名验证必须一次性读入文件；
/// sha256因此也基于同一份字节整段计算（复用 [`verify_sha256`]，适配 `hash_ext`），
/// 不再对文件自维护流式哈希——两种方式的峰值内存相同（都被签名步骤的整读决定）。
///
/// # 签名必填
/// 空签名在 [`ensure_signed`] 处直接拒绝（fail closed）。需要"只验 sha256"的调用方
/// 使用 [`verify_sha256`]——那是一条独立原语，不属于本入口的语义。
///
/// # 双重 base64
/// [`Manifest.signature`] 与公钥文件内容均为 base64(minisign 文本)：
/// 先经 [`Base64Ext::base64_decode`] 解外层，
/// 再交 minisign-verify 解析内层文本
/// 与官方插件行为一致，详见 [`verify_signature`]。
pub fn verify_artifact_path(path: &Path, artifact: &Artifact) -> anyhow::Result<()> {
    ensure_signed(artifact)?;
    let data = std::fs::read(path)
        .map_err(|e| anyhow!("读取下载产物失败（{}）：{e}", path.display()))?;
    verify_sha256(&data, &artifact.sha256)?;
    verify_signature(&data, &artifact.signature, active_pubkey())?;
    Ok(())
}

/// 当前构建使用的验签公钥（base64(minisign 公钥文本)）
///
/// - **生产构建**：嵌入的正式公钥 [`ROLLCALLER_UPDATE_PUBKEY`]；
/// - **测试构建**：改用进程内临时生成的测试密钥对（见 [`test_keys`]）——正式公钥对应的私钥
///   不在代码库内，测试无法为它产出合法签名，若不替换，组合入口的成功路径将无法被单测覆盖。
#[cfg(not(test))]
fn active_pubkey() -> &'static str {
    ROLLCALLER_UPDATE_PUBKEY
}

/// 测试构建：使用进程内临时测试公钥（详见 [`test_keys`]）
#[cfg(test)]
fn active_pubkey() -> &'static str {
    test_keys::pubkey_b64()
}

/// 测试构建专用的进程内密钥对（不落盘、不写死私钥，规避私钥泄露进生产构建）
///
/// 生产构建不包含本模块（`#[cfg(test)]`）。作用：让 [`verify_artifact`] /
/// [`verify_artifact_path`] 在单测中既能覆盖成功路径（用本密钥对真实签名），
/// 又不依赖仓库外的正式私钥。
#[cfg(test)]
pub(crate) mod test_keys {
    use base64::Engine;
    use std::sync::LazyLock;

    struct Keys {
        /// 公钥外层 base64（= [`super::active_pubkey`] 的返回内容）
        pub_b64: String,
        /// 用于签名的测试密钥对
        pair: minisign::KeyPair,
    }

    static KEYS: LazyLock<Keys> = LazyLock::new(|| {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().expect("生成测试密钥对失败");
        // minisign 0.7：sign(pk: Option<&PublicKey>, sk: &SecretKey, reader: Read, trusted, untrusted)，
        // 恒定使用预哈希（对应 minisign-verify 的 verify(..., true)）
        let pub_text = pair.pk.to_box().expect("公钥转 PublicKeyBox 失败").into_string();
        Keys {
            pub_b64: base64::engine::general_purpose::STANDARD.encode(pub_text),
            pair,
        }
    });

    /// 测试公钥外层 base64（= 生产侧嵌入公钥的同格式）
    pub(crate) fn pubkey_b64() -> &'static str {
        &KEYS.pub_b64
    }

    /// 用测试密钥对 `data` 签名，返回外层 base64(minisign 签名文本)——即 `Artifact.signature` 期望格式
    pub(crate) fn sign(data: &[u8]) -> String {
        let signature = minisign::sign(None, &KEYS.pair.sk, data, None, None).expect("测试签名失败");
        base64::engine::general_purpose::STANDARD.encode(signature.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_paths;
    use std::path::PathBuf;

    /// 动态生成临时密钥对并对 data 签名，返回 (公钥外层 base64, 签名外层 base64)
    ///
    /// 不落盘、不写死任何私钥，规避私钥泄露进生产构建的风险。
    fn sign_fixture(data: &[u8]) -> (String, String) {
        let key_pair = minisign::KeyPair::generate_unencrypted_keypair().expect("生成密钥对失败");
        // minisign 0.7：sign(pk: Option<&PublicKey>, sk: &SecretKey, reader: Read, trusted, untrusted)，
        // 恒定使用预哈希（对应 minisign-verify 的 verify(..., true)）
        let signature = minisign::sign(None, &key_pair.sk, &data[..], None, None).expect("签名失败");
        let pub_text = key_pair
            .pk
            .to_box()
            .expect("公钥转 PublicKeyBox 失败")
            .into_string();
        let pub_b64 = base64::engine::general_purpose::STANDARD.encode(pub_text);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_string());
        (pub_b64, sig_b64)
    }

    /// 构造 Artifact：用动态密钥对 data 签名，sha256 为 data 的真实哈希
    fn make_artifact(data: &[u8]) -> Artifact {
        let (_, sig_b64) = sign_fixture(data);
        Artifact {
            url: "https://example.com/app.exe".parse().unwrap(),
            sha256: hex::encode(sha2::Sha256::digest(data)),
            signature: sig_b64,
            size: data.len() as u64,
        }
    }

    /// 构造 Artifact：用**当前构建的有效密钥对**（[`test_keys`]）签名，sha256 为 data 的真实哈希
    ///
    /// 组合入口（[`verify_artifact`] / [`verify_artifact_path`]）走 [`active_pubkey`]：
    /// 生产是正式公钥、测试是 [`test_keys`]，故只有用 `test_keys` 签名才能覆盖成功路径。
    fn make_signed_artifact(data: &[u8]) -> Artifact {
        Artifact {
            url: "https://example.com/app.exe".parse().unwrap(),
            sha256: hex::encode(sha2::Sha256::digest(data)),
            signature: super::test_keys::sign(data),
            size: data.len() as u64,
        }
    }

    #[test]
    fn verify_ok_with_valid_signature_and_sha256() {
        let data = b"hello update payload";
        let (pub_b64, sig_b64) = sign_fixture(data);
        verify_signature(data, &sig_b64, &pub_b64).expect("合法签名应通过");
        verify_sha256(data, &hex::encode(sha2::Sha256::digest(data))).expect("正确 sha256 应通过");
    }

    #[test]
    fn tampered_data_rejected() {
        let data = b"hello update payload";
        let (pub_b64, sig_b64) = sign_fixture(data);
        let mut tampered = data.to_vec();
        tampered[0] ^= 0xFF;
        // 篡改 1 字节 → 签名与 sha256 任意一个环节都应拒绝
        assert!(verify_signature(&tampered, &sig_b64, &pub_b64).is_err());
        assert!(
            verify_sha256(&tampered, &hex::encode(sha2::Sha256::digest(data))).is_err()
        );
    }

    #[test]
    fn wrong_public_key_rejected() {
        let data = b"hello update payload";
        let (_, sig_b64) = sign_fixture(data);
        let (other_pub_b64, _) = sign_fixture(data); // 另一对密钥的公钥
        assert!(verify_signature(data, &sig_b64, &other_pub_b64).is_err());
    }

    #[test]
    fn empty_data_rejected() {
        // 签名的对象是非空数据，传空字节验证 → 不匹配
        let data = b"non-empty payload";
        let (pub_b64, sig_b64) = sign_fixture(data);
        assert!(verify_signature(b"", &sig_b64, &pub_b64).is_err());
    }

    #[test]
    fn invalid_base64_rejected() {
        let data = b"payload";
        let (pub_b64, _) = sign_fixture(data);
        // 非法 base64 的签名 / 公钥 → 解码环节报错
        assert!(verify_signature(data, "!!!not-base64!!!", &pub_b64).is_err());
        assert!(verify_signature(data, "c2ln", "not valid base64!!!").is_err());
    }

    #[test]
    fn base64_roundtrip() {
        let s = "hello";
        let b64 = base64::engine::general_purpose::STANDARD.encode(s);
        assert_eq!(b64.base64_decode().unwrap(), s);
        // 非法 base64 与解码后非 UTF-8 都报错
        assert!("!!!".base64_decode().is_err());
        let non_utf8 = base64::engine::general_purpose::STANDARD.encode([0xFF, 0xFE]);
        assert!(non_utf8.base64_decode().is_err());
    }

    #[test]
    fn verify_artifact_rejects_wrong_sha256() {
        // 组合入口：先过签名必填，sha256 不匹配即在第一环节中止，不进入签名环节
        let data = b"artifact bytes";
        let artifact = make_signed_artifact(data);
        let artifact = Artifact {
            sha256: "00".repeat(32),
            ..artifact
        };
        let err = verify_artifact(data, &artifact).expect_err("sha256 错误应被拒绝");
        assert!(err.to_string().contains("sha256 不匹配"));
    }

    /// 组合入口成功路径：sha256 正确 + 由当前构建有效密钥对签名 → 通过
    #[test]
    fn verify_artifact_ok_with_active_key() {
        let data = b"artifact bytes";
        let artifact = make_signed_artifact(data);
        verify_artifact(data, &artifact).expect("正确 sha256 + 有效签名应通过");
    }

    /// 组合入口拒绝**无签名**产物（fail closed）
    ///
    /// 回归用例：清单缓存可被本地改写，若允许"空签名 = 只验 sha256"，改写者只要把 sha256
    /// 换成自己产物的哈希即可让任意 exe 通过校验并被安装。
    #[test]
    fn verify_artifact_rejects_unsigned() {
        let data = b"artifact bytes";
        let artifact = Artifact {
            signature: String::new(),
            ..make_signed_artifact(data)
        };
        let err = verify_artifact(data, &artifact).expect_err("空签名应被拒绝");
        assert!(
            err.to_string().contains("缺少产物签名"),
            "错误应指出签名缺失: {err}"
        );

        // 空白签名同样拒绝（避免 trim 前后不一致被绕过）
        let blank = Artifact {
            signature: "   \n".to_string(),
            ..make_signed_artifact(data)
        };
        assert!(verify_artifact(data, &blank).is_err());
    }

    /// 组合入口拒绝"签名不属于当前公钥"的产物（签名真实但密钥不对）
    #[test]
    fn verify_artifact_rejects_foreign_key() {
        let data = b"artifact bytes";
        let artifact = make_artifact(data); // sign_fixture 生成的临时密钥对，非 active_pubkey
        assert!(verify_artifact(data, &artifact).is_err());
    }

    #[test]
    fn embedded_pubkey_readable() {
        // 占位阶段内容是说明文本；作者替换后是 base64(minisign 公钥文本)。两者都必须是 UTF-8。
        let pk = ROLLCALLER_UPDATE_PUBKEY;
        assert!(!pk.is_empty());
    }

    /// 互操作测试：本机存在 tauri CLI 时，用 `tauri signer sign` 生成真实 .sig 并验证。
    ///
    /// 无 tauri CLI 的环境自动跳过（打印提示），手动复现步骤见本模块文档。
    #[test]
    fn interop_with_tauri_signer() {
        let tauri_cli_path = app_paths::resources_dir().join("tauri/cargo-tauri.exe");
        let tauri_ok = std::process::Command::new(&tauri_cli_path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !tauri_cli_path.exists() || !tauri_ok {
            eprintln!("[interop] 未检测到 tauri CLI，跳过互操作测试；手动步骤见 crate::service::update::verify 模块文档");
            return;
        }

        let dir = app_paths::temp_dir().join(format!("rollcaller-verify-interop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect(&format!("创建临时目录 {} 失败", dir.display()));

        // 1. 生成密钥对（固定测试密码，非交互）
        // 新版 tauri-cli（2.11）：generate 用 -p 密码 + -w 私钥输出文件 + --ci 免交互；
        // 产出 {file}.key 与 {file}.key.pub（find_file_with_ext 按扩展名兜底兼容命名）
        let key_file = dir.join("interopkey.key");
        let status = std::process::Command::new(&tauri_cli_path)
            .args(["signer", "generate", "--ci", "-p", "test", "-w"])
            .arg(&key_file)
            .status()
            .expect("执行 tauri signer generate 失败");
        assert!(status.success(), "tauri signer generate 退出码非 0");

        // 2. 对数据文件签名
        let data = b"interop payload for tauri signer";
        let data_file = dir.join("payload.bin");
        std::fs::write(&data_file, data).expect("写入 payload 失败");
        // 新版 sign：FILE 为位置参数，-f 为私钥文件路径，-p 为密码。
        // env 默认值与 -f 冲突——此处显式剥离，强制使用本测试自生成的密钥文件。
        let status = std::process::Command::new(tauri_cli_path)
            .env_remove("TAURI_SIGNING_PRIVATE_KEY")
            .env_remove("TAURI_SIGNING_PRIVATE_KEY_PASSWORD")
            .arg("signer")
            .arg("sign")
            .arg(&data_file)
            .arg("-f")
            .arg(&key_file)
            .args(["-p", "test"])
            .status()
            .expect("执行 tauri signer sign 失败");
        assert!(status.success(), "tauri signer sign 退出码非 0");

        // 3. 读取 .pub / .sig 并验证。
        // 注意：新版 tauri-cli 直接输出 base64(minisign 文本) 的文件（与发布清单
        // signature 字段同格式），此处文件内容即外层 base64，直接复用，不再二次编码。
        let pub_file = find_file_with_ext(&dir, "pub").expect("未找到 .pub 文件");
        let sig_file = find_file_with_ext(&dir, "sig").expect("未找到 .sig 文件");
        let pub_b64 = std::fs::read_to_string(&pub_file).map(|s| s.trim().to_string()).unwrap();
        let sig_b64 = std::fs::read_to_string(&sig_file).map(|s| s.trim().to_string()).unwrap();
        verify_signature(data, &sig_b64, &pub_b64).expect("tauri signer 产物应通过 minisign-verify 验证");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 在目录中按扩展名查找文件（tauri signer 输出文件名随版本/命名变化，用扩展名兜底）
    fn find_file_with_ext(dir: &Path, ext: &str) -> Option<PathBuf> {
        std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
            p.extension().is_some_and(|e| e == ext)
        })
    }
}
