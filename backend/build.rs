use std::env::current_dir;
use std::path::Path;
use std::process::Command;

fn main() {
    ensure_build_info();
    generate_config_constant();
    tauri_build::build()
}

/// 保证 build-info.json 存在
///
/// 正常路径由 tauri 的 beforeDevCommand / beforeBuildCommand 生成（每次 dev/build 都刷新，
/// 不受 cargo 缓存机制摆布）；这里只兜住「不经 tauri 的 cargo 调用」—— 例如直接执行
/// `cargo build --release`，此时 app_info 的 release 分支要用 include_str! 在编译期嵌入它，
/// 文件不存在会直接编译失败。
///
/// 本函数的输出恒定，**刻意不输出任何 rustc-env**：即使 build script 重跑也不会连带触发
/// 下游 crate 重编译 —— 重编译的唯一驱动交给 include_str! 的文件依赖跟踪（仅 release 生效），
/// 而 debug 走运行时读取，完全不进入编译输入。
fn ensure_build_info() {
    // build script 的工作目录是包根（backend/），故路径均相对它
    let json = Path::new("build-info.json");
    println!("cargo:rerun-if-changed=build-info.json");
    if json.exists() {
        return;
    }
    let status = Command::new("uv")
        .args(["run", "scripts/generate_build_info.py"])
        .current_dir("..")
        .status()
        .expect("执行 uv run scripts/generate_build_info.py 失败");
    if !status.success() {
        panic!("生成 build-info.json 失败");
    }
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
