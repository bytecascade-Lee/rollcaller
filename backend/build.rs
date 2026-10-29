fn main() {
    println!("cargo:rustc-env=ROLLCALLER_TARGET={}", std::env::var("TARGET").expect("cargo 会在 build script 中注入 TARGET"));
    tauri_build::build();
}
