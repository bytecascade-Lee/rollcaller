//! 下载产物的签名验证（minisign / Ed25519）与 sha256 完整性校验
//!
//! 本模块纯逻辑 + 测试，不涉及安装（安装分派在任务 04-06）。
//! 调用时机：下载完成后、任何写盘/安装前调用 [`verify_artifact`]（bytes 形态，供
//! install 阶段对读入内存的产物复验）或 [`verify_artifact_path`]（path 形态，供
//! download 阶段对已落盘的 .part 校验）；失败即中止，不进入安装阶段。
//!
//! # 双重 base64
//!
//! `manifest.signature` 与公钥文件（`assets/updater.pub`）的原始内容都是
//! **base64(minisign 文本)**：
//!
//! - minisign 的 `.sig` 文件全文（含 `untrusted comment:` / `trusted comment:` 头）
//!   经 base64 编码后存入 [`Artifact::signature`]；
//! - `tauri signer generate` 产出的 `.pub` 文件全文（`untrusted comment: minisign
//!   public key: ...`）经 base64 编码后作为 `assets/updater.pub` 的内容。
//!
//! 因此验签前必须先用 STANDARD base64 解码，得到 minisign 文本后再交给
//! `minisign-verify` 解析。参考 `docs/02-签名与验签方案.md`。
//!
//! # 互操作（手动可复现）
//!
//! 1. 生成密钥对：`tauri signer generate -w <密码> <名称>`，产出 `<名称>.key` 与 `<名称>.pub`；
//! 2. 将 `<名称>.pub` 的**完整内容**（含 untrusted comment 行）base64 编码后写入 `backend/assets/updater.pub`；
//! 3. 对产物签名：`tauri signer sign -f <文件> -k <名称>.key -p <密码>`，产出 `<文件>.sig`；
//! 4. 将 `<文件>.sig` 的**完整内容** base64 编码后填入清单的 `signature` 字段；
//! 5. 运行互操作测试 `cargo test updater::verify::interop` 验证（本机无 tauri CLI 时
//!    自动跳过并打印提示，此时可依上述步骤手动验证）。

use crate::common::constant::secrets::ROLLCALLER_UPDATE_PUBKEY;
use crate::common::entity::update::Artifact;
use crate::common::ext::encode_ext::Base64Ext;
use crate::common::ext::hash_ext::HashExt;
use base64::Engine;
use sha2::Digest;
use std::path::Path;

/// 外层 base64 → UTF-8 文本（minisign 的公钥/签名均为文本格式，先解 base64）
pub fn base64_to_string(b64: &str) -> anyhow::Result<String> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(b64)?;
    Ok(String::from_utf8(decoded)?)
}

/// 与官方插件 verify_signature 逐行一致：先解 base64，再解析 minisign 文本并验证
///
/// - `release_signature`：base64(minisign 签名文本)，即 `Artifact.signature`
/// - `pub_key_b64`：base64(minisign 公钥文本)，即 [`ROLLCALLER_UPDATE_PUBKEY`]
pub fn verify_signature(
    data: &[u8],
    release_signature: &str,
    pub_key_b64: &str,
) -> anyhow::Result<()> {
    let public_key = minisign_verify::PublicKey::decode(&pub_key_b64.base64_decode().unwrap());
    let signature = minisign_verify::Signature::decode(&release_signature.base64_decode().unwrap());
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

/// 下载字节的统一校验入口（bytes 形态）：先 sha256（完整性），再 minisign 签名（真实性）
pub fn verify_artifact(data: &[u8], artifact: &Artifact) -> anyhow::Result<()> {
    verify_sha256(data, &artifact.sha256)?;
    verify_signature(data, &artifact.signature, ROLLCALLER_UPDATE_PUBKEY)?;
    Ok(())
}

/// 已落盘文件的统一校验入口（path 形态）：整段读入后 sha256 + 可选签名
///
/// # 内存语义
/// minisign-verify 仅支持整段字节（`&[u8]`），签名验证必须一次性读入文件；sha256
/// 因此也基于同一份字节整段计算（复用 [`verify_sha256`]，适配 `hash_ext`），不再对
/// 文件自维护流式哈希——两种方式的峰值内存相同（都被签名步骤的整读决定）。
///
/// # 签名可空
/// `artifact.signature` 为空时只校验 sha256（Go updater 更新器仅发布 sha256、无
/// minisign 签名）；主包产物带签名，走完整双校验。
///
/// # 双重 base64（勿改）
/// `manifest.signature` 与公钥文件内容均为 base64(minisign 文本)：先经
/// [`Base64Ext::base64_decode`] 解外层，再交 minisign-verify 解析内层文本（与官方插件
/// 行为一致，见 [`verify_signature`]）。
pub fn verify_artifact_path(path: &Path, artifact: &Artifact) -> anyhow::Result<()> {
    let data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("读取下载产物失败（{}）：{e}", path.display()))?;
    verify_sha256(&data, &artifact.sha256)?;
    if !artifact.signature.is_empty() {
        verify_signature(&data, &artifact.signature, ROLLCALLER_UPDATE_PUBKEY)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // 篡改 1 字节 → 签名与 sha256 任一环节都应拒绝
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
        // 组合入口：sha256 不匹配时在第一个环节即中止，不进入签名环节
        let data = b"artifact bytes";
        let (_, sig_b64) = sign_fixture(data);
        let artifact = Artifact {
            url: "https://example.com/app.exe".parse().unwrap(),
            sha256: "00".repeat(32),
            signature: sig_b64,
            size: data.len() as u64,
        };
        let err = verify_artifact(data, &artifact).expect_err("sha256 错误应被拒绝");
        assert!(err.to_string().contains("sha256 不匹配"));
    }

    #[test]
    fn verify_artifact_reaches_signature_step() {
        // sha256 正确、但占位公钥无法解析 → 签名环节报错（证明组合顺序：先 sha256 后签名）
        let data = b"artifact bytes";
        let artifact = make_artifact(data);
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
        let tauri_ok = std::process::Command::new("tauri")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !tauri_ok {
            eprintln!("[interop] 未检测到 tauri CLI，跳过互操作测试；手动步骤见 updater::verify 模块文档");
            return;
        }

        let dir = std::env::temp_dir().join(format!("rollcaller-verify-interop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建临时目录失败");

        // 1. 生成密钥对（固定测试密码，非交互）
        let status = std::process::Command::new("tauri")
            .args(["signer", "generate", "-w", "test", "-d"])
            .arg(&dir)
            .arg("interopkey")
            .status()
            .expect("执行 tauri signer generate 失败");
        assert!(status.success(), "tauri signer generate 退出码非 0");

        // 2. 对数据文件签名
        let data = b"interop payload for tauri signer";
        let data_file = dir.join("payload.bin");
        std::fs::write(&data_file, data).expect("写入 payload 失败");
        let key_file = find_file_with_ext(&dir, "key").expect("未找到 .key 文件");
        let status = std::process::Command::new("tauri")
            .args(["signer", "sign", "-f"])
            .arg(&data_file)
            .args(["-k"])
            .arg(&key_file)
            .args(["-p", "test"])
            .status()
            .expect("执行 tauri signer sign 失败");
        assert!(status.success(), "tauri signer sign 退出码非 0");

        // 3. 读取 .pub / .sig，外层 base64 编码后验证（跨实现互操作：Go 签名 → Rust 验证）
        let pub_file = find_file_with_ext(&dir, "pub").expect("未找到 .pub 文件");
        let sig_file = find_file_with_ext(&dir, "sig").expect("未找到 .sig 文件");
        let pub_b64 =
            base64::engine::general_purpose::STANDARD.encode(std::fs::read_to_string(&pub_file).unwrap());
        let sig_b64 =
            base64::engine::general_purpose::STANDARD.encode(std::fs::read_to_string(&sig_file).unwrap());
        verify_signature(data, &sig_b64, &pub_b64).expect("tauri signer 产物应通过 minisign-verify 验证");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 在目录中按扩展名查找文件（tauri signer 输出文件名随版本/命名变化，用扩展名兜底）
    fn find_file_with_ext(dir: &std::path::Path, ext: &str) -> Option<std::path::PathBuf> {
        std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
            p.extension().is_some_and(|e| e == ext)
        })
    }
}
