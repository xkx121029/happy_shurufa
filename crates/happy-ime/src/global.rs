//! 全局常量与 DLL 模块句柄。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;

use windows::Win32::Foundation::{HINSTANCE, HMODULE};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::core::GUID;

pub const IME_NAME: &str = "Happy 输入法";
pub const IME_NAME_ASCII: &str = "HappyIME";

/// 本输入法的 COM CLSID 与各项注册 GUID（独立生成，勿与他人重复）。
pub const IME_ID: GUID = GUID::from_u128(0x1F2E8C4A_6B3D_4E7F_9A51_C8D2B0E4F6A3);
pub const LANG_PROFILE_ID: GUID = GUID::from_u128(0x2A9D7B5E_8C40_4F1B_B6E2_D3F5A7C9E1B4);
pub const DISPLAY_ATTR_ID: GUID = GUID::from_u128(0x3B8E6C4F_9D51_4A2C_A7F3_E4B6D8A0F2C5);
pub const SHIFT_TOGGLE_KEY: GUID = GUID::from_u128(0x4C7F5D6A_8E62_4B3D_B8A4_F5C7E9D1A3B6);

/// 简体中文（中国）langid
pub const LANGID_ZH_CN: u16 = 0x0804;
/// 中文（简体，中国）- 美式键盘 的 HKL（0.62 中 HKL 包装 *mut c_void）
pub const HKL_ZH_CN_US: isize = 0x08040804;

/// HKL_ZH_CN_US → windows 句柄类型
pub fn zh_cn_us_hkl() -> windows::Win32::UI::Input::KeyboardAndMouse::HKL {
    windows::Win32::UI::Input::KeyboardAndMouse::HKL(HKL_ZH_CN_US as usize as *mut _)
}

/// HINSTANCE 不是 Send/Sync，以 isize 存储。
static DLL_MODULE: OnceLock<isize> = OnceLock::new();

pub fn setup(module: HINSTANCE) {
    let _ = DLL_MODULE.set(module.0 as isize);
}

pub fn dll_module() -> HINSTANCE {
    HINSTANCE(DLL_MODULE.get().copied().unwrap_or_default() as *mut _)
}

/// DLL 自身完整路径（注册 InProcServer32 与托盘图标用）。
pub fn dll_path() -> Option<String> {
    let mut buf = [0u16; 512];
    let len = unsafe { GetModuleFileNameW(Some(HMODULE(dll_module().0)), &mut buf) } as usize;
    if len == 0 || len as usize > buf.len() {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Rust 字符串 → 以 NUL 结尾的 UTF-16。
pub fn to_widez(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}
