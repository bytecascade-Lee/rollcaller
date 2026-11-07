use crate::config::app_paths::{cache_dir, logs_dir, temp_dir, AppMode};
use crate::updater::model::{Artifact, UpdateSource};
use anyhow::anyhow;
use semver::Version;
use std::path::PathBuf;

/// 指定更新源的版本清单缓存路径 `cache/update/github/1.2.3.json`
pub fn manifest(source: &UpdateSource, version: &Version) -> PathBuf {
    cache_dir().join(format!("update/{}/{version}.json", source.slug()))
}

/// 生成下载中的临时碎片文件路径 `temp_dir/downloads/{随机uuid}.part`
pub fn part() -> PathBuf {
    temp_dir().join(format!(
        "downloads/{}.part",
        uuid::Uuid::new_v4().to_string().replace("-", "")
    ))
}

/// 下载完成的安装包 / 压缩包的存放路径 `temp/update/packages/{file_name}`
pub fn package(file_name: &str) -> PathBuf {
    temp_dir().join(format!("update/packages/{file_name}"))
}

/// 凭据对应的正式产物落点（文件名取 url 最后一段）
pub fn package_of(artifact: &Artifact) -> anyhow::Result<PathBuf> {
    let file_name = artifact
        .file_name()
        .ok_or_else(|| anyhow!("下载地址缺少文件名：{}", artifact.url))?;
    Ok(package(&file_name))
}

/// 更新器缓存目录 `cache/update/bin/`
pub fn updater_bin_dir() -> PathBuf {
    cache_dir().join("update/bin")
}

/// 更新器可执行文件的存放路径 `cache/update/bin/{file_name}`
pub fn portable_updater_bin(file_name: &str) -> PathBuf {
    updater_bin_dir().join(file_name)
}

/// zip 包解压的暂存根目录 `temp/update/staging/{portable|develop}-source-{version}/`
pub fn zip_staging(mode: &AppMode, version: &Version) -> PathBuf {
    temp_dir()
        .join("update")
        .join("staging")
        .join(format!("{}-source-{version}", mode.slug()))
}

/// 便携版更新时的备份落点 `temp_dir/update/backup/{mode}-backup-{from}-to-{to}-{随机6位uuid}`
pub fn backup(mode: &AppMode, from: &Version, to: &Version) -> PathBuf {
    temp_dir().join("update").join("backup").join(format!(
        "{}-backup-{from}-to-{to}-{}",
        mode.slug(),
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..6]
    ))
}

/// 更新器的安装会话配置文件路径
pub fn updater_config(mode: &AppMode, from: &Version, to: &Version) -> PathBuf {
    temp_dir()
        .join("update")
        .join("config")
        .join(format!("{}-config-{from}-to-{to}.json", mode.slug()))
}

/// 一次安装会话的更新器日志 `logs/u/{mode}-update-{from}-to-{to}-{微秒}.log`
pub fn updater_log(mode: &AppMode, from: &Version, to: &Version) -> PathBuf {
    logs_dir().join("u").join(format!(
        "{}-update-{from}-to-{to}-{}.log",
        mode.slug(),
        jiff::Timestamp::now().as_microsecond()
    ))
}
