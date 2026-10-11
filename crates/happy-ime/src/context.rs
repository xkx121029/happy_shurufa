//! 宿主进程识别。TSF IME 运行在文本应用进程内，直接查当前进程即可。

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW, GetCurrentProcessId,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

/// 当前前台窗口所属进程的 exe 文件名（小写，如 "code.exe"）。
/// 运行在宿主进程内，无跨进程权限问题；所有失败静默降级返回 None。
pub fn foreground_process() -> Option<String> {
    unsafe {
        // IME 的输入焦点就在本进程；前台窗口用于兜底（如 IMEUI 宿主）
        let hwnd = GetForegroundWindow();
        let mut pid = 0u32;
        if hwnd.is_invalid() {
            pid = GetCurrentProcessId();
        } else {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                pid = GetCurrentProcessId();
            }
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        if result.is_err() || len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        let name = full.rsplit(['\\', '/']).next().unwrap_or(&full);
        Some(name.to_lowercase())
    }
}
