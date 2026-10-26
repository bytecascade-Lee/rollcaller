use crate::common::enums::sys;

#[cfg(target_os = "windows")]
pub const OS: sys::OS = sys::OS::Windows;
#[cfg(not(target_os = "windows"))]
compile_error!("Rollcaller now only supports windows operating systems!");

#[cfg(target_arch = "x86_64")]
pub const ARCH: sys::Arch = sys::Arch::X86_64;
#[cfg(target_arch = "aarch64")]
pub const ARCH: sys::Arch = sys::Arch::Arm64;
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
compile_error!("Rollcaller now only supports x86_64 and aarch64 architectures!");

pub const TARGET: &str = env!("ROLLCALLER_TARGET");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_agrees_with_os_and_arch() {
        let expected = match (OS, ARCH) {
            (sys::OS::Windows, sys::Arch::X86_64) => "x86_64-pc-windows-msvc",
            (sys::OS::Windows, sys::Arch::Arm64) => "aarch64-pc-windows-msvc",
            _ => unreachable!("compile_error 已限制支持的 OS 与架构组合"),
        };
        assert_eq!(TARGET, expected);
    }
}
