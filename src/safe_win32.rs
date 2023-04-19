use std::ffi::c_void;

use eyre::eyre;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, BOOL, HANDLE, HMODULE, HWND, LPARAM, LRESULT, MAX_PATH, NO_ERROR, POINT,
    RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, PtInRect, HDC, HMONITOR, MONITORINFO, MONITOR_FROM_FLAGS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, WTSUnRegisterSessionNotification};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ACCESS_RIGHTS};
use windows::Win32::UI::Shell::{Shell_NotifyIconW, NOTIFYICONDATAW, NOTIFY_ICON_MESSAGE};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetForegroundWindow, GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, InsertMenuW, MessageBoxW, PostMessageW, RegisterClassW, SetCursorPos,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, SetWindowsHookExW, ShowWindow, ShowWindowAsync,
    TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, HHOOK, HICON, HMENU, HOOKPROC, MENU_ITEM_FLAGS,
    MESSAGEBOX_RESULT, MESSAGEBOX_STYLE, MSG, SET_WINDOW_POS_FLAGS, SHOW_WINDOW_CMD, TRACK_POPUP_MENU_FLAGS,
    WINDOWS_HOOK_ID, WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WINDOW_STYLE, WNDCLASSW,
};

pub trait Win32Handle
where
    Self: std::marker::Sized,
{
    fn ok(self) -> eyre::Result<Self>;
}

impl Win32Handle for HWND {
    fn ok(self) -> eyre::Result<HWND> {
        match self {
            HWND(0) => Err(eyre::Report::from(windows::core::Error::from_win32())),
            HWND(h) => Ok(HWND(h)),
        }
    }
}

impl Win32Handle for HMODULE {
    fn ok(self) -> eyre::Result<HMODULE> {
        match self {
            HMODULE(0) => Err(eyre::Report::from(windows::core::Error::from_win32())),
            HMODULE(h) => Ok(HMODULE(h)),
        }
    }
}

impl Win32Handle for HMONITOR {
    fn ok(self) -> eyre::Result<HMONITOR> {
        if self.is_invalid() {
            Err(eyre::Report::from(windows::core::Error::from_win32()))
        } else {
            Ok(self)
        }
    }
}

pub fn call_next_hook(hhk: HHOOK, ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { CallNextHookEx(hhk, ncode, wparam, lparam) }
}

pub fn close_handle(handle: HANDLE) -> eyre::Result<()> {
    unsafe { CloseHandle(handle).ok().map_err(eyre::Report::from) }
}

pub fn create_popup_menu() -> eyre::Result<HMENU> {
    unsafe { CreatePopupMenu().map_err(eyre::Report::from) }
}

#[allow(clippy::too_many_arguments)]
pub fn create_window(
    dwexstyle: WINDOW_EX_STYLE,
    lpclassname: PCWSTR,
    lpwindowname: PCWSTR,
    dwstyle: WINDOW_STYLE,
    x: i32,
    y: i32,
    nwidth: i32,
    nheight: i32,
    hwndparent: HWND,
    hmenu: HMENU,
    hinstance: HMODULE,
    lpparam: *mut c_void,
) -> eyre::Result<HWND> {
    unsafe {
        CreateWindowExW(
            dwexstyle,
            lpclassname,
            lpwindowname,
            dwstyle,
            x,
            y,
            nwidth,
            nheight,
            hwndparent,
            hmenu,
            hinstance,
            Some(lpparam),
        )
        .ok()
    }
}

pub fn def_window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn destroy_icon(hicon: HICON) -> eyre::Result<()> {
    unsafe { DestroyIcon(hicon).ok().map_err(eyre::Report::from) }
}

pub fn destroy_menu(hmenu: HMENU) -> eyre::Result<()> {
    unsafe { DestroyMenu(hmenu).ok().map_err(eyre::Report::from) }
}

pub fn dispatch_message(msg: &MSG) -> LRESULT {
    unsafe { DispatchMessageW(msg) }
}

pub fn dwm_get_window_attribute_extended_frame_bounds(hwnd: HWND) -> eyre::Result<RECT> {
    unsafe {
        let mut extended_frame_bounds = RECT::default();
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut extended_frame_bounds as *mut RECT as *mut core::ffi::c_void,
            std::mem::size_of::<RECT>() as u32,
        )
        .map(|_| extended_frame_bounds)
        .map_err(|e| eyre!(e.message()))
    }
}

pub fn enum_display_monitors() -> eyre::Result<Vec<MONITORINFO>> {
    // Callback function for the Win32 EnumDisplayMonitors function
    unsafe extern "system" fn enum_display_monitors_callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        monitors: LPARAM,
    ) -> BOOL {
        if let Ok(monitor_info) = get_monitor_info(hmonitor) {
            let monitors = &mut *(monitors.0 as *mut Vec<MONITORINFO>);
            monitors.push(monitor_info);
            BOOL::from(true)
        } else {
            BOOL::from(false)
        }
    }

    let mut monitors = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_display_monitors_callback),
            LPARAM(&mut monitors as *mut Vec<MONITORINFO> as isize),
        )
    }
    .ok()
    .map(|_| monitors)
    .map_err(|_| eyre!("EnumDisplayMonitors() failed"))
}

pub fn get_cursor_pos() -> eyre::Result<POINT> {
    let mut point = Default::default();
    unsafe { GetCursorPos(&mut point).ok().map(|_| point).map_err(eyre::Report::from) }
}

pub fn get_foreground_window() -> eyre::Result<HWND> {
    unsafe { GetForegroundWindow().ok() }
}

pub fn get_message(msg: &mut MSG, hwnd: HWND, wmsgfiltermin: u32, wmsgfiltermax: u32) -> BOOL {
    unsafe { GetMessageW(msg, hwnd, wmsgfiltermin, wmsgfiltermax) }
}

pub fn get_module_handle(lpmodulename: PCWSTR) -> eyre::Result<HMODULE> {
    unsafe { GetModuleHandleW(lpmodulename).map_err(|e| eyre!(e.message())) }
}

pub fn get_module_file_name(hprocess: HANDLE) -> eyre::Result<String> {
    let mut filename: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
    match unsafe { GetModuleFileNameExW(hprocess, HMODULE::default(), &mut filename) } {
        0 => Err(std::io::Error::last_os_error().into()),
        _ => String::from_utf16(&filename).map_err(eyre::Report::from),
    }
}

pub fn get_monitor_info(hmonitor: HMONITOR) -> eyre::Result<MONITORINFO> {
    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        GetMonitorInfoW(hmonitor, &mut monitor_info)
            .ok()
            .map(|_| monitor_info)
            .map_err(eyre::Report::from)
    }
}

pub fn get_window_long_ptr(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX) -> eyre::Result<isize> {
    match unsafe { GetWindowLongPtrW(hwnd, nindex) } {
        0 => Err(std::io::Error::last_os_error().into()),
        longptr => Ok(longptr),
    }
}

pub fn get_window_rect(hwnd: HWND) -> eyre::Result<RECT> {
    let mut rect: RECT = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)
            .ok()
            .map(|_| rect)
            .map_err(eyre::Report::from)
    }
}

#[allow(dead_code)]
pub fn get_window_text_length(hwnd: HWND) -> eyre::Result<i32> {
    unsafe {
        SetLastError(NO_ERROR);

        let text_length: i32 = GetWindowTextLengthW(hwnd);
        if text_length > 0 {
            return Ok(text_length);
        }

        if GetLastError() == NO_ERROR {
            return Ok(0);
        }

        Err(std::io::Error::last_os_error().into())
    }
}

#[allow(dead_code)]
pub fn get_window_text(hwnd: HWND) -> eyre::Result<String> {
    let text_length = get_window_text_length(hwnd)? + 1;
    let mut chars = vec![0; text_length as usize];
    unsafe { GetWindowTextW(hwnd, &mut chars) };
    String::from_utf16(chars.as_slice()).map_err(eyre::Report::from)
}

pub struct ThreadProcessId {
    pub thread_id: u32,
    pub process_id: u32,
}

pub fn get_window_thread_process_id(hwnd: HWND) -> ThreadProcessId {
    let mut process_id = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
    ThreadProcessId { thread_id, process_id }
}

pub fn insert_menu(
    hmenu: HMENU,
    uposition: u32,
    uflags: MENU_ITEM_FLAGS,
    uidnewitem: usize,
    lpnewitem: &str,
) -> eyre::Result<()> {
    unsafe {
        InsertMenuW(hmenu, uposition, uflags, uidnewitem, &HSTRING::from(lpnewitem))
            .ok()
            .map_err(eyre::Report::from)
    }
}

pub fn message_box(hwnd: HWND, text: &str, caption: &str, utype: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe { MessageBoxW(hwnd, &HSTRING::from(text), &HSTRING::from(caption), utype) }
}

pub fn monitor_from_window(hwnd: HWND, dwflags: MONITOR_FROM_FLAGS) -> eyre::Result<HMONITOR> {
    unsafe { MonitorFromWindow(hwnd, dwflags).ok() }
}

pub fn open_process(
    dwdesiredaccess: PROCESS_ACCESS_RIGHTS,
    binherithandle: bool,
    dwprocessid: u32,
) -> eyre::Result<HANDLE> {
    unsafe { OpenProcess(dwdesiredaccess, binherithandle, dwprocessid).map_err(eyre::Report::from) }
}

pub fn point_in_rect(rect: &RECT, point: &POINT) -> bool {
    unsafe { PtInRect(rect, *point).as_bool() }
}

pub fn post_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL {
    unsafe { PostMessageW(hwnd, msg, wparam, lparam) }
}

pub fn register_class(wndclass: &WNDCLASSW) -> eyre::Result<()> {
    unsafe {
        match RegisterClassW(wndclass) {
            0 => Err(std::io::Error::last_os_error().into()),
            _ => Ok(()),
        }
    }
}

pub fn set_cursor_pos(x: i32, y: i32) -> eyre::Result<()> {
    unsafe { SetCursorPos(x, y).ok().map_err(eyre::Report::from) }
}

pub fn set_foreground_window(hwnd: HWND) -> eyre::Result<()> {
    unsafe { SetForegroundWindow(hwnd).ok().map_err(eyre::Report::from) }
}

pub fn set_window_long_ptr(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> eyre::Result<isize> {
    unsafe {
        SetLastError(NO_ERROR);

        let previous = SetWindowLongPtrW(hwnd, nindex, dwnewlong);
        if previous != 0 || GetLastError() == NO_ERROR {
            return Ok(previous);
        }

        Err(std::io::Error::last_os_error().into())
    }
}

pub fn set_window_pos(
    hwnd: HWND,
    hwndinsertafter: HWND,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    uflags: SET_WINDOW_POS_FLAGS,
) -> eyre::Result<()> {
    unsafe {
        SetWindowPos(hwnd, hwndinsertafter, x, y, cx, cy, uflags)
            .ok()
            .map_err(eyre::Report::from)
    }
}

pub fn set_windows_hook(
    idhook: WINDOWS_HOOK_ID,
    lpfn: HOOKPROC,
    hmod: HMODULE,
    dwthreadid: u32,
) -> eyre::Result<HHOOK> {
    unsafe { SetWindowsHookExW(idhook, lpfn, hmod, dwthreadid).map_err(eyre::Report::from) }
}

pub fn shell_notify_icon(dwmessage: NOTIFY_ICON_MESSAGE, lpdata: &mut NOTIFYICONDATAW) -> eyre::Result<()> {
    unsafe { Shell_NotifyIconW(dwmessage, lpdata).ok().map_err(eyre::Report::from) }
}

pub fn show_window(hwnd: HWND, ncmdshow: SHOW_WINDOW_CMD) -> eyre::Result<()> {
    unsafe { ShowWindow(hwnd, ncmdshow).ok().map_err(eyre::Report::from) }
}

#[allow(dead_code)]
pub fn show_window_async(hwnd: HWND, ncmdshow: SHOW_WINDOW_CMD) -> eyre::Result<()> {
    unsafe { ShowWindowAsync(hwnd, ncmdshow).ok().map_err(eyre::Report::from) }
}

pub fn track_popup_menu(
    hmenu: HMENU,
    uflags: TRACK_POPUP_MENU_FLAGS,
    x: i32,
    y: i32,
    nreserved: i32,
    hwnd: HWND,
) -> BOOL {
    unsafe { TrackPopupMenu(hmenu, uflags, x, y, nreserved, hwnd, None) }
}

pub fn translate_message(msg: &MSG) -> BOOL {
    unsafe { TranslateMessage(msg) }
}

pub fn unhook_windows_hook_ex(hhk: HHOOK) -> eyre::Result<()> {
    unsafe { UnhookWindowsHookEx(hhk).ok().map_err(eyre::Report::from) }
}

pub fn wts_register_session_notification(hwnd: HWND, dwflags: u32) -> eyre::Result<()> {
    unsafe {
        WTSRegisterSessionNotification(hwnd, dwflags)
            .ok()
            .map_err(eyre::Report::from)
    }
}

pub fn wts_unregister_session_notification(hwnd: HWND) -> eyre::Result<()> {
    unsafe { WTSUnRegisterSessionNotification(hwnd).ok().map_err(eyre::Report::from) }
}
