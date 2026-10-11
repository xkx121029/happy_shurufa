//! 候选/联想悬浮窗。
//!
//! 设计规范：纯色背景、圆角（DWM）、细边框、无衬线字体、
//! 从下到上 120ms 进入动画（ease-out）+ 整体淡入，不抢焦点不进任务栏。

use std::mem::size_of;
use std::time::Instant;

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, CreateSolidBrush, EndPaint, FillRect, GetDC,
    GetTextExtentPoint32W, LOGFONTW, OUT_TT_PRECIS, PAINTSTRUCT, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DROPSHADOW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    HWND_TOPMOST, KillTimer, GetWindowLongPtrW, RegisterClassExW, SetLayeredWindowAttributes, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, WNDCLASSEXW, WINDOW_LONG_PTR_INDEX, WM_PAINT,
    WM_TIMER, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, Result, w};

use crate::global::{dll_module, to_widez};

const WINDOW_CLASS: PCWSTR = w!("HAPPY_CANDIDATE");

// 布局常量（逻辑像素）
const PAD_X: i32 = 10;
const PAD_Y: i32 = 6;
const ROW_GAP: i32 = 2;
const BORDER: i32 = 1;
const ANIM_MS: u128 = 120;
const ANIM_OFFSET: i32 = 12;
const ANIM_TIMER_ID: usize = 0x4850; // "HP"

// 配色（纯色，无玻璃质感）
const BG: COLORREF = COLORREF(0xFBFBFB); // #FBFBFB
const BORDER_COLOR: COLORREF = COLORREF(0xD6D6D6); // #D6D6D6
const HIGHLIGHT: COLORREF = COLORREF(0xE9F1FB); // #E9F1FB
const TEXT: COLORREF = COLORREF(0x1F1F1F);
const DESC: COLORREF = COLORREF(0x8A8A8A);
const INDEX: COLORREF = COLORREF(0x7A7A7A);

/// 窗口要显示的一条条目。
#[derive(Clone)]
pub struct DisplayItem {
    pub label: String,
    pub desc: String,
}

/// 注册窗口类（DllMain 时调用一次）。
pub fn setup() -> bool {
    unsafe {
        let hmodule = dll_module();
        let hmodule: HMODULE = if hmodule.is_invalid() {
            GetModuleHandleW(PCWSTR::null()).unwrap_or_default()
        } else {
            HMODULE(hmodule.0)
        };
        let wcex = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(hmodule.0),
            hCursor: Default::default(),
            hbrBackground: CreateSolidBrush(BG),
            lpszClassName: WINDOW_CLASS,
            ..Default::default()
        };
        RegisterClassExW(&wcex) != 0
    }
}

#[cfg(target_pointer_width = "64")]
type LongPtr = isize;
#[cfg(target_pointer_width = "32")]
type LongPtr = i32;

/// 传给 WM_PAINT 的绘制参数。
struct PaintArg {
    width: i32,
    height: i32,
    row_height: i32,
    rows: Vec<Row>, // (序号, 主体, 说明) 的 UTF-16
    main_font: isize,
    small_font: isize,
}

struct Row {
    index: Vec<u16>,
    text: Vec<u16>,
    desc: Vec<u16>,
}

impl PaintArg {
    fn attach(self, hwnd: HWND) {
        unsafe {
            SetWindowLongPtrW(
                hwnd,
                WINDOW_LONG_PTR_INDEX(0),
                Box::into_raw(Box::new(self)) as LongPtr,
            );
        }
    }
}

/// 动画状态（挂在 GWLP_USERDATA）。
struct AnimState {
    target_x: i32,
    target_y: i32,
    start: Instant,
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => paint(hwnd),
            WM_TIMER => {
                if wparam.0 == ANIM_TIMER_ID {
                    animate_step(hwnd);
                    return LRESULT(0);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn paint(hwnd: HWND) -> LRESULT {
    unsafe {
        let arg = {
            let ptr = SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), 0);
            if ptr == 0 {
                return LRESULT(0);
            }
            // 借用后立即放回，避免重入丢失
            let arg: &PaintArg = &*(ptr as *const PaintArg);
            let _ = SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), ptr);
            arg
        };
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        if dc.is_invalid() {
            return LRESULT(0);
        }

        // 背景 + 边框
        let full = RECT { left: 0, top: 0, right: arg.width, bottom: arg.height };
        let bg_brush = CreateSolidBrush(BG);
        FillRect(dc, &full, bg_brush);
        let top = RECT { left: 0, top: 0, right: arg.width, bottom: BORDER };
        let bottom = RECT { left: 0, top: arg.height - BORDER, right: arg.width, bottom: arg.height };
        let left = RECT { left: 0, top: 0, right: BORDER, bottom: arg.height };
        let right = RECT { left: arg.width - BORDER, top: 0, right: arg.width, bottom: arg.height };
        let border_brush = CreateSolidBrush(BORDER_COLOR);
        FillRect(dc, &top, border_brush);
        FillRect(dc, &bottom, border_brush);
        FillRect(dc, &left, border_brush);
        FillRect(dc, &right, border_brush);

        // 行：第一行高亮
        SetBkMode(dc, TRANSPARENT);
        let mut y = BORDER + PAD_Y;
        for (i, row) in arg.rows.iter().enumerate() {
            if i == 0 {
                let hl = RECT {
                    left: BORDER,
                    top: y - 1,
                    right: arg.width - BORDER,
                    bottom: y + arg.row_height - ROW_GAP + 1,
                };
                FillRect(dc, &hl, CreateSolidBrush(HIGHLIGHT));
            }
            let font = if i == 0 { arg.main_font } else { arg.small_font };
            let index_font = arg.small_font;
            draw_text(dc, &row.index, BORDER + PAD_X, y, INDEX, index_font as *mut _);
            let text_x = BORDER + PAD_X + row_index_width(dc, row, index_font as *mut _) + 4;
            draw_text(dc, &row.text, text_x, y, TEXT, font as *mut _);
            let desc_x = text_x + row_text_width(dc, row, font as *mut _) + 10;
            draw_text(dc, &row.desc, desc_x, y, DESC, arg.small_font as *mut _);
            y += arg.row_height + ROW_GAP;
        }

        ReleaseDC(Some(hwnd), dc);
        let _ = EndPaint(hwnd, &ps);
        LRESULT(0)
    }
}

unsafe fn draw_text(dc: windows::Win32::Graphics::Gdi::HDC, text: &[u16], x: i32, y: i32, color: COLORREF, font: *mut core::ffi::c_void) {
    unsafe {
        SelectObject(dc, windows::Win32::Graphics::Gdi::HGDIOBJ(font));
        SetTextColor(dc, color);
        let _ = TextOutW(dc, x, y, text);
    }
}

unsafe fn row_index_width(dc: windows::Win32::Graphics::Gdi::HDC, row: &Row, font: *mut core::ffi::c_void) -> i32 {
    unsafe { text_width(dc, &row.index, font) }
}

unsafe fn row_text_width(dc: windows::Win32::Graphics::Gdi::HDC, row: &Row, font: *mut core::ffi::c_void) -> i32 {
    unsafe { text_width(dc, &row.text, font) }
}

unsafe fn text_width(dc: windows::Win32::Graphics::Gdi::HDC, text: &[u16], font: *mut core::ffi::c_void) -> i32 {
    unsafe {
        let old = SelectObject(dc, windows::Win32::Graphics::Gdi::HGDIOBJ(font));
        let mut size = windows::Win32::Foundation::SIZE::default();
        let _ = GetTextExtentPoint32W(dc, text, &mut size);
        SelectObject(dc, old);
        size.cx
    }
}

unsafe fn animate_step(hwnd: HWND) {
    unsafe {
        let state = GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA) as *mut AnimState;
        let Some(state) = state.as_ref() else { return };
        let elapsed = state.start.elapsed().as_millis();
        let t = ((elapsed as f64) / (ANIM_MS as f64)).min(1.0);
        // ease-out cubic
        let eased = 1.0 - (1.0 - t).powi(3);
        let y = state.target_y + ((1.0 - eased) * ANIM_OFFSET as f64).round() as i32;
        let alpha = (255.0 * eased) as u8;
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            state.target_x,
            y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE,
        );
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA);
        if t >= 1.0 {
            let _ = KillTimer(Some(hwnd), ANIM_TIMER_ID);
        }
    }
}

/// 候选窗。
pub struct CandidateWindow {
    hwnd: HWND,
    main_font: isize,
    small_font: isize,
}

impl CandidateWindow {
    pub fn create() -> Result<Self> {
        unsafe {
            let hinstance = dll_module();
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                WINDOW_CLASS,
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                hinstance.into(),
                None,
            )?;
            // DWM 圆角（Win11 生效，旧系统静默忽略）
            let pref = DWMWCP_ROUND.0 as u32;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const u32 as *const core::ffi::c_void,
                4,
            );
            // DPI 缩放字号
            let dpi = GetDpiForWindow(hwnd).max(96);
            let scale = dpi as f32 / 96.0;
            let font_name = to_widez("Microsoft YaHei UI");
            let make_font = |pt: i32| -> isize {
                let height = -(pt as f32 * scale * 96.0 / 72.0).round() as i32;
                let mut lf = LOGFONTW {
                    lfHeight: height,
                    lfOutPrecision: OUT_TT_PRECIS,
                    ..Default::default()
                };
                lf.lfFaceName[..font_name.len().min(32)]
                    .copy_from_slice(&font_name[..font_name.len().min(32)]);
                CreateFontIndirectW(&lf).0 as isize
            };
            let main_font = make_font(12);
            let small_font = make_font(9);
            Ok(Self { hwnd, main_font, small_font })
        }
    }

    /// 显示条目，定位到光标右下方 (x, y)（屏幕物理坐标）。
    pub fn show(&self, items: &[DisplayItem], x: i32, y: i32) -> Result<()> {
        unsafe {
            if items.is_empty() {
                return Ok(());
            }
            let dc = GetDC(Some(self.hwnd));
            let rows: Vec<Row> = items
                .iter()
                .enumerate()
                .map(|(i, item)| Row {
                    index: to_widez(&format!("{}.", (i + 1) % 10)),
                    text: to_widez(&item.label),
                    desc: to_widez(&item.desc),
                })
                .collect();
            // 测量
            let mut main_h = 0;
            let mut small_h = 0;
            let mut index_w = 0;
            let mut text_w = 0;
            let mut desc_w = 0;
            let mut size = windows::Win32::Foundation::SIZE::default();
            for row in &rows {
                SelectObject(dc, windows::Win32::Graphics::Gdi::HGDIOBJ(self.small_font as *mut _));
                let _ = GetTextExtentPoint32W(dc, &row.index, &mut size);
                index_w = index_w.max(size.cx);
                small_h = small_h.max(size.cy);
                SelectObject(dc, windows::Win32::Graphics::Gdi::HGDIOBJ(self.main_font as *mut _));
                let _ = GetTextExtentPoint32W(dc, &row.text, &mut size);
                text_w = text_w.max(size.cx);
                main_h = main_h.max(size.cy);
                let _ = GetTextExtentPoint32W(dc, &row.desc, &mut size);
                desc_w = desc_w.max(size.cx);
            }
            ReleaseDC(Some(self.hwnd), dc);
            let row_height = main_h.max(small_h);
            let width = BORDER * 2 + PAD_X * 2 + index_w + 4 + text_w + if desc_w > 0 { 10 + desc_w } else { 0 };
            let height = BORDER * 2 + PAD_Y * 2 + rows.len() as i32 * row_height + (rows.len() as i32 - 1) * ROW_GAP;

            let arg = PaintArg {
                width,
                height,
                row_height,
                rows,
                main_font: self.main_font,
                small_font: self.small_font,
            };
            // 覆盖旧绘制参数
            let old = SetWindowLongPtrW(self.hwnd, WINDOW_LONG_PTR_INDEX(0), 0);
            if old != 0 {
                let _ = Box::from_raw(old as *mut PaintArg);
            }
            arg.attach(self.hwnd);

            // 屏幕边缘防溢出
            let (x, y) = clamp_to_screen(x, y, width, height);

            // 动画状态
            let state = Box::new(AnimState { target_x: x, target_y: y, start: Instant::now() });
            SetWindowLongPtrW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                Box::into_raw(state) as LongPtr,
            );
            SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y + ANIM_OFFSET, width, height, SWP_NOACTIVATE)?;
            let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), 0, windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA);
            let _ = SetTimer(Some(self.hwnd), ANIM_TIMER_ID, 16, None);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            Ok(())
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
            let _ = KillTimer(Some(self.hwnd), ANIM_TIMER_ID);
        }
    }

    pub fn destroy(&self) -> Result<()> {
        unsafe { DestroyWindow(self.hwnd) }
    }
}

fn clamp_to_screen(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    unsafe {
        let dpi = windows::Win32::UI::HiDpi::GetDpiForSystem() as i32;
        let _ = dpi;
        let mut rect = RECT::default();
        if windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_ACTION(0x0048), // SPI_GETWORKAREA
            0,
            Some(&mut rect as *mut RECT as *mut core::ffi::c_void),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_err()
        {
            return (x, y);
        }
        let x = if x + w > rect.right { (rect.right - w).max(0) } else { x };
        let y = if y + h > rect.bottom { y - h - 24 } else { y };
        (x, y)
    }
}
