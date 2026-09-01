pub fn arch(arch: &str) -> &str {
    match arch {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    }
}
