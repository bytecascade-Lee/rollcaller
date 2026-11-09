//! 产物与清单的签名验证（minisign / Ed25519）与 sha256 完整性校验。
//!
//! # 双层 base64
//!
//! 清单 `signature` 与公钥内容都是 `base64(minisign 文本)`，需要先通过 base64 解码外层，
//! 再交给 `minisign-verify` 解析。tauri-cli >= 2.11 的 `.pub` / `.sig` 本身就是该格式。

use crate::common::ext::encode_ext::Base64Ext;
use crate::common::ext::hash_ext::HashExt;
use crate::updater::constant::ROLLCALLER_UPDATE_PUBKEY;
use crate::updater::model::{Artifact, ManifestDigest};
use anyhow::anyhow;
use std::path::Path;

/// 验签，先解外层 base64，再解析 minisign 文本
pub fn verify_signature(data: &[u8], release_signature: &str, pub_key_b64: &str) -> anyhow::Result<()> {
    let public_key = minisign_verify::PublicKey::decode(
        &pub_key_b64
            .trim()
            .base64_decode()
            .map_err(|e| anyhow!("签名公钥 base64 解码失败：{e} / {pub_key_b64}"))?,
    )
    .map_err(|e| anyhow!("minisign 公钥解析失败（外层解码后应为两行：untrusted comment 行 + 42 字节公钥 base64）：{e}"))?;
    let signature = minisign_verify::Signature::decode(
        &release_signature
            .trim()
            .base64_decode()
            .map_err(|e| anyhow!("签名 base64 解码失败：{e} / {release_signature}"))?,
    )
    .map_err(|e| anyhow!("minisign 签名解析失败（外层解码后应为四行 minisign 签名文本，当前行数或载荷长度不符）：{e}"))?;
    // true = 兼容 legacy 预哈希签名
    public_key.verify(data, &signature, true)?;
    Ok(())
}

/// sha256 十六进制，与 `manifest.sha256` 比较
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> anyhow::Result<()> {
    let actual = data.sha256();
    if actual != expected_hex.to_ascii_lowercase() {
        anyhow::bail!("sha256 不匹配: 期望 {expected_hex}, 实际 {actual}");
    }
    Ok(())
}

/// 签名必填前置：空签名即拒绝（产物与清单同一档 fail closed）
fn ensure_signed(signature: &str, what: &str) -> anyhow::Result<()> {
    if signature.trim().is_empty() {
        anyhow::bail!("{what}缺少 minisign 签名（signature 为空），拒绝校验")
    }
    Ok(())
}

/// 产物校验：签名必填 -> sha256 -> minisign
pub fn verify_artifact(data: &[u8], artifact: &Artifact) -> anyhow::Result<()> {
    ensure_signed(&artifact.signature, "更新产物")?;
    verify_sha256(data, &artifact.sha256)?;
    verify_signature(data, &artifact.signature, active_pubkey())?;
    Ok(())
}

/// 清单验签：签名必填（与产物同一档）
///
/// sha256 不在这里比——调用方对"摘要不符"的处理是宽松的（索引抄错不该阻断更新），
/// 而签名验不过说明清单不是发布方给的，必须直接失败。
pub fn verify_manifest(data: &[u8], digest: &ManifestDigest) -> anyhow::Result<()> {
    let signature = digest.signature.as_deref().unwrap_or_default();
    ensure_signed(signature, "清单")?;
    verify_signature(data, signature, active_pubkey())
}

/// 产物校验：签名必填 -> sha256 -> minisign
pub fn verify_artifact_path(path: &Path, artifact: &Artifact) -> anyhow::Result<()> {
    ensure_signed(&artifact.signature, "更新产物")?;
    let data = std::fs::read(path).map_err(|e| anyhow!("读取下载产物失败（{}）：{e}", path.display()))?;
    verify_sha256(&data, &artifact.sha256)?;
    verify_signature(&data, &artifact.signature, active_pubkey())?;
    Ok(())
}

/// 当前构建的验签公钥
#[cfg(not(test))]
fn active_pubkey() -> &'static str {
    ROLLCALLER_UPDATE_PUBKEY
}

/// 测试构建使用进程内临时测试公钥
#[cfg(test)]
fn active_pubkey() -> &'static str {
    test_keys::pubkey_b64()
}

/// 测试构建专用的进程内密钥对（不落盘、不写死私钥；`#[cfg(test)]` 不进生产构建）
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

    /// 用测试密钥对 `data` 签名，返回外层 base64(minisign 签名文本)
    pub(crate) fn sign(data: &[u8]) -> String {
        let signature = minisign::sign(None, &KEYS.pair.sk, data, None, None).expect("测试签名失败");
        base64::engine::general_purpose::STANDARD.encode(signature.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_paths;
    use base64::Engine;
    use sha2::Digest;
    use std::path::PathBuf;

    /// 临时密钥对签名，返回 (公钥外层 base64, 签名外层 base64)
    fn sign_fixture(data: &[u8]) -> (String, String) {
        let key_pair = minisign::KeyPair::generate_unencrypted_keypair().expect("生成密钥对失败");
        let signature = minisign::sign(None, &key_pair.sk, data, None, None).expect("签名失败");
        let pub_text = key_pair.pk.to_box().expect("公钥转 PublicKeyBox 失败").into_string();
        let pub_b64 = base64::engine::general_purpose::STANDARD.encode(pub_text);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_string());
        (pub_b64, sig_b64)
    }

    /// 用临时密钥对（非当前构建的有效密钥）签名的产物
    fn make_artifact(data: &[u8]) -> Artifact {
        let (_, sig_b64) = sign_fixture(data);
        Artifact {
            url: "https://example.com/app.exe".parse().unwrap(),
            sha256: hex::encode(sha2::Sha256::digest(data)),
            signature: sig_b64,
            size: data.len() as u64,
        }
    }

    /// 用当前构建的有效密钥对（[`test_keys`]）签名的产物——组合入口的成功路径只能靠它覆盖
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
        assert!(verify_sha256(&tampered, &hex::encode(sha2::Sha256::digest(data))).is_err());
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

    /// 清单签名必填：缺失或纯空白一律拒绝（与产物同一档 fail closed）
    #[test]
    fn verify_manifest_rejects_unsigned() {
        let data = br#"{"version":"0.8.6","platforms":{}}"#;
        let unsigned = ManifestDigest {
            sha256: hex::encode(sha2::Sha256::digest(data)),
            signature: None,
        };
        assert!(verify_manifest(data, &unsigned).is_err(), "缺签名应被拒绝");

        // 空签名 / 纯空白等同于"无签名"，不能因为字段存在就当成"有签名"
        let blank = ManifestDigest {
            signature: Some("  \n".to_string()),
            ..unsigned
        };
        assert!(verify_manifest(data, &blank).is_err(), "空白签名应被拒绝");
    }

    /// 清单签名：必须由当前构建的密钥对签出，别人的签名一律拒绝
    #[test]
    fn verify_manifest_requires_active_key_signature() {
        let data = br#"{"version":"0.8.6","platforms":{}}"#;
        let digest = ManifestDigest {
            sha256: hex::encode(sha2::Sha256::digest(data)),
            signature: Some(super::test_keys::sign(data)),
        };
        verify_manifest(data, &digest).expect("当前密钥的签名应通过");

        let (_, foreign_sig) = sign_fixture(data);
        let foreign = ManifestDigest {
            signature: Some(foreign_sig),
            ..digest
        };
        assert!(verify_manifest(data, &foreign).is_err(), "异己签名应被拒绝");
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

    /// 组合入口拒绝无签名产物
    #[test]
    fn verify_artifact_rejects_unsigned() {
        let data = b"artifact bytes";
        let artifact = Artifact {
            signature: String::new(),
            ..make_signed_artifact(data)
        };
        let err = verify_artifact(data, &artifact).expect_err("空签名应被拒绝");
        assert!(err.to_string().contains("缺少 minisign 签名"), "错误应指出签名缺失: {err}");

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
        let tauri_cli_path = app_paths::resources_dir().join(format!("tauri/cargo-tauri{}", std::env::consts::EXE_SUFFIX));
        let tauri_ok = std::process::Command::new(&tauri_cli_path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !tauri_cli_path.exists() || !tauri_ok {
            eprintln!("[interop] 未检测到 tauri CLI，跳过互操作测试；手动步骤见 crate::updater::verify 模块文档");
            return;
        }

        let dir = app_paths::temp_dir().join(format!("rollcaller-verify-interop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("创建临时目录 {} 失败: {error}", dir.display()));

        // 1. 生成密钥对
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

        // 3. 读取 .pub / .sig 并验证
        let pub_file = find_file_with_ext(&dir, "pub").expect("未找到 .pub 文件");
        let sig_file = find_file_with_ext(&dir, "sig").expect("未找到 .sig 文件");
        let pub_b64 = std::fs::read_to_string(&pub_file).map(|s| s.trim().to_string()).unwrap();
        let sig_b64 = std::fs::read_to_string(&sig_file).map(|s| s.trim().to_string()).unwrap();
        verify_signature(data, &sig_b64, &pub_b64).expect("tauri signer 产物应通过 minisign-verify 验证");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 在目录中按扩展名查找文件（tauri signer 输出文件名随版本/命名变化，用扩展名兜底）
    fn find_file_with_ext(dir: &Path, ext: &str) -> Option<PathBuf> {
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == ext))
    }
}
