use jiff::tz::TimeZone;
use jiff::Timestamp;
use std::env;
use std::env::current_dir;
use std::process::Command;

fn main() {
    generate_build_info();
    generate_config_constant();
    tauri_build::build()
}

fn generate_build_info() {
    let branch = env::var("BRANCH_NAME").unwrap_or_else(|_| get_git_output(&["rev-parse", "--abbrev-ref", "HEAD"]));
    let commit_count = get_git_output(&["rev-list", "--count", "HEAD"]);
    let short_hash = get_git_output(&["rev-parse", "--short", "HEAD"]);
    let commit_time = get_git_output(&["log", "-1", "--format=%cd", "--date=iso-strict"]);
    let version = env::var("VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
        .trim_start_matches(|c: char| c == 'v' || c == 'V')
        .to_string();
    let time_string = format!("{:.0}", Timestamp::now().to_zoned(TimeZone::system()));
    let build_time = time_string.split('[').next().unwrap_or(time_string.as_str());

    println!("cargo:rustc-env=GIT_BRANCH={}", branch);
    println!("cargo:rustc-env=GIT_COMMIT_COUNT={}", commit_count);
    println!("cargo:rustc-env=GIT_SHORT_HASH={}", short_hash);
    println!("cargo:rustc-env=GIT_COMMIT_TIME={}", commit_time);
    println!("cargo:rustc-env=VERSION={}", version);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
}

fn get_git_output(args: &[&str]) -> String {
    let output = Command::new("git").args(args).output().expect("failed to execute git");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn generate_config_constant() {
    let status = Command::new("uv")
        .arg("run")
        .arg("scripts/generate_config_constants.py")
        .current_dir(current_dir().unwrap().parent().unwrap())
        .spawn()
        .expect("Failed to spawn process")
        .wait()
        .expect("Failed to execute uv run");
    if !status.success() {
        panic!("Configuration generation failed");
    }
    println!("cargo:rerun-if-changed=../resources/develop/config.key");
    println!("cargo:rerun-if-changed=../scripts/generate_config_constants.py");
}
