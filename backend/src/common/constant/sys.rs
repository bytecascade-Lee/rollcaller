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

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const OS_ARCH_COMPATIBLE_WITH_HISTORY: &str = "windows-x86_64";
#[cfg(all(target_os = "windows", target_arch = "arm64"))]
pub const OS_ARCH_COMPATIBLE_WITH_HISTORY: &str = "windows-aarch64";
#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "aarch64")
)))]
compile_error!("Rollcaller only supports windows-x86_64 and windows-aarch64 combinations!");
