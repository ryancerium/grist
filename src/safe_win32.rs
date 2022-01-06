use std::ffi::c_void;

use eyre::eyre;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, BOOL, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, MAX_PATH, PWSTR, RECT,
    WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_FROM_FLAGS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, WTSUnRegisterSessionNotification};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ACCESS_RIGHTS};
use windows::Win32::UI::Shell::{Shell_NotifyIconW, NOTIFYICONDATAW, NOTIFY_ICON_MESSAGE};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow,
    GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    InsertMenuW, PostMessageW, RegisterClassW, SetCursorPos, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    SetWindowsHookExW, ShowWindow, ShowWindowAsync, TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, HHOOK,
    HMENU, HOOKPROC, MENU_ITEM_FLAGS, MSG, SET_WINDOW_POS_FLAGS, SHOW_WINDOW_CMD, TRACK_POPUP_MENU_FLAGS,
    WINDOWS_HOOK_ID, WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WINDOW_STYLE, WNDCLASSW,
};

pub trait Win32ReturnIntoResult
where
    Self: Sized,
{
    fn into_result(self) -> eyre::Result<Self>;
}

impl Win32ReturnIntoResult for BOOL {
    fn into_result(self) -> eyre::Result<BOOL> {
        match self.as_bool() {
            false => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for HANDLE {
    fn into_result(self) -> eyre::Result<HANDLE> {
        match self {
            HANDLE(-1) => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for HHOOK {
    fn into_result(self) -> eyre::Result<HHOOK> {
        match self {
            HHOOK(0) => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for HINSTANCE {
    fn into_result(self) -> eyre::Result<Self> {
        match self {
            HINSTANCE(0) => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for HMENU {
    fn into_result(self) -> eyre::Result<Self> {
        match self {
            HMENU(0) => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for HMONITOR {
    fn into_result(self) -> eyre::Result<Self> {
        match self {
            HMONITOR(0) => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for HWND {
    fn into_result(self) -> eyre::Result<Self> {
        match self.0 {
            0 => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

impl Win32ReturnIntoResult for u16 {
    fn into_result(self) -> eyre::Result<Self> {
        match self {
            0 => Err(std::io::Error::last_os_error().into()),
            _ => Ok(self),
        }
    }
}

pub fn call_next_hook(hhk: HHOOK, ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { CallNextHookEx(hhk, ncode, wparam, lparam) }
}

pub fn close_handle(handle: HANDLE) -> eyre::Result<BOOL> {
    unsafe { CloseHandle(handle).into_result() }
}

pub fn create_popup_menu() -> eyre::Result<HMENU> {
    unsafe { CreatePopupMenu().into_result() }
}

#[allow(clippy::too_many_arguments)]
pub fn create_window(
    dwexstyle: WINDOW_EX_STYLE,
    lpclassname: PWSTR,
    lpwindowname: PWSTR,
    dwstyle: WINDOW_STYLE,
    x: i32,
    y: i32,
    nwidth: i32,
    nheight: i32,
    hwndparent: HWND,
    hmenu: HMENU,
    hinstance: HINSTANCE,
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
            lpparam,
        )
        .into_result()
    }
}

pub fn def_window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn dispatch_message(msg: &MSG) -> LRESULT {
    unsafe { DispatchMessageW(msg) }
}

pub fn dwm_get_window_attribute_extended_frame_bounds(hwnd: HWND) -> eyre::Result<RECT> {
    let mut extended_frame_bounds = RECT::default();
    unsafe {
        let success = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut extended_frame_bounds as *mut RECT as *mut core::ffi::c_void,
            std::mem::size_of_val(&extended_frame_bounds) as u32,
        );
        match success {
            Ok(_) => Ok(extended_frame_bounds),
            Err(e) => Err(eyre!(e.message())),
        }
    }
}

pub fn enum_display_monitors() -> eyre::Result<Vec<MONITORINFO>> {
    unsafe extern "system" fn enum_display_monitors_callback(
        h_monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        _dw_data: LPARAM,
    ) -> BOOL {
        let monitor_info = match get_monitor_info(h_monitor) {
            Ok(monitor_info) => monitor_info,
            Err(_) => return false.into(),
        };
        let monitors = &mut *(_dw_data.0 as *mut Vec<MONITORINFO>);
        monitors.push(monitor_info);
        true.into()
    }

    let mut monitors = Vec::new();
    unsafe {
        let success = EnumDisplayMonitors(
            HDC(0),
            std::ptr::null_mut(),
            Some(enum_display_monitors_callback),
            LPARAM(&mut monitors as *mut Vec<MONITORINFO> as isize),
        );
        if !success.as_bool() {
            return Err(eyre!("EnumDisplayMonitors() failed"));
        }
    }
    Ok(monitors)
}

pub fn get_foreground_window() -> eyre::Result<HWND> {
    unsafe { GetForegroundWindow().into_result() }
}

pub fn get_message(msg: &mut MSG, hwnd: HWND, wmsgfiltermin: u32, wmsgfiltermax: u32) -> BOOL {
    unsafe { GetMessageW(msg, hwnd, wmsgfiltermin, wmsgfiltermax) }
}

pub fn get_module_handle(lpmodulename: PWSTR) -> eyre::Result<HINSTANCE> {
    unsafe { GetModuleHandleW(lpmodulename).into_result() }
}

pub fn get_module_file_name(hprocess: HANDLE) -> eyre::Result<String> {
    let mut filename: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
    match unsafe { K32GetModuleFileNameExW(hprocess, HINSTANCE(0), PWSTR(filename.as_mut_ptr()), MAX_PATH) } {
        0 => Err(std::io::Error::last_os_error().into()),
        _ => match String::from_utf16(&filename) {
            Ok(s) => Ok(s),
            Err(e) => Err(e.into()),
        },
    }
}

pub fn get_monitor_info(hmonitor: HMONITOR) -> eyre::Result<MONITORINFO> {
    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        GetMonitorInfoW(hmonitor, &mut monitor_info)
            .into_result()
            .map(|_| monitor_info)
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
    unsafe { GetWindowRect(hwnd, &mut rect).into_result().map(|_| rect) }
}

pub fn get_window_text_length(hwnd: HWND) -> eyre::Result<i32> {
    unsafe {
        SetLastError(WIN32_ERROR(0));

        let text_length: i32 = GetWindowTextLengthW(hwnd);
        if text_length > 0 {
            return Ok(text_length);
        }

        if GetLastError().0 == 0 {
            return Ok(0);
        }

        Err(std::io::Error::last_os_error().into())
    }
}

pub fn get_window_text(hwnd: HWND) -> eyre::Result<String> {
    let text_length = get_window_text_length(hwnd)? + 1;
    let mut chars = vec![0; text_length as usize];
    unsafe { GetWindowTextW(hwnd, PWSTR(chars.as_mut_ptr()), chars.len() as i32) };
    match String::from_utf16(chars.as_slice()) {
        Ok(t) => Ok(t),
        Err(e) => Err(e.into()),
    }
}

pub struct ThreadProcessId {
    pub thread_id: u32,
    pub process_id: u32,
}

pub fn get_window_thread_process_id(hwnd: HWND) -> ThreadProcessId {
    let mut process_id = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
    ThreadProcessId { thread_id, process_id }
}

pub fn insert_menu(
    hmenu: HMENU,
    uposition: u32,
    uflags: MENU_ITEM_FLAGS,
    uidnewitem: usize,
    lpnewitem: &str,
) -> eyre::Result<BOOL> {
    unsafe { InsertMenuW(hmenu, uposition, uflags, uidnewitem, lpnewitem).into_result() }
}

pub fn monitor_from_window(hwnd: HWND, dwflags: MONITOR_FROM_FLAGS) -> eyre::Result<HMONITOR> {
    unsafe { MonitorFromWindow(hwnd, dwflags).into_result() }
}

pub fn open_process(
    dwdesiredaccess: PROCESS_ACCESS_RIGHTS,
    binherithandle: bool,
    dwprocessid: u32,
) -> eyre::Result<HANDLE> {
    unsafe {
        let handle = OpenProcess(dwdesiredaccess, binherithandle, dwprocessid);
        handle.into_result()
    }
}

pub fn post_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL {
    unsafe { PostMessageW(hwnd, msg, wparam, lparam) }
}

pub fn register_class(wndclass: &WNDCLASSW) -> eyre::Result<u16> {
    unsafe { RegisterClassW(wndclass).into_result() }
}

pub fn set_cursor_pos(x: i32, y: i32) -> eyre::Result<BOOL> {
    unsafe { SetCursorPos(x, y).into_result() }
}

pub fn set_foreground_window(hwnd: HWND) -> eyre::Result<BOOL> {
    unsafe { SetForegroundWindow(hwnd).into_result() }
}

pub fn set_window_long_ptr(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX, dwnewlong: isize) -> eyre::Result<isize> {
    unsafe {
        SetLastError(WIN32_ERROR(0));

        let previous = SetWindowLongPtrW(hwnd, nindex, dwnewlong);
        if previous != 0 {
            return Ok(previous);
        }

        if GetLastError().0 == 0 {
            return Ok(0);
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
) -> eyre::Result<BOOL> {
    unsafe { SetWindowPos(hwnd, hwndinsertafter, x, y, cx, cy, uflags).into_result() }
}

pub fn set_windows_hook(
    idhook: WINDOWS_HOOK_ID,
    lpfn: Option<HOOKPROC>,
    hmod: HINSTANCE,
    dwthreadid: u32,
) -> eyre::Result<HHOOK> {
    unsafe { SetWindowsHookExW(idhook, lpfn, hmod, dwthreadid).into_result() }
}

pub fn shell_notify_icon(dwmessage: NOTIFY_ICON_MESSAGE, lpdata: &mut NOTIFYICONDATAW) -> eyre::Result<BOOL> {
    unsafe { Shell_NotifyIconW(dwmessage, lpdata).into_result() }
}

pub fn show_window(hwnd: HWND, ncmdshow: SHOW_WINDOW_CMD) -> eyre::Result<BOOL> {
    unsafe { ShowWindow(hwnd, ncmdshow).into_result() }
}

#[allow(dead_code)]
pub fn show_window_async(hwnd: HWND, ncmdshow: SHOW_WINDOW_CMD) -> eyre::Result<BOOL> {
    unsafe { ShowWindowAsync(hwnd, ncmdshow).into_result() }
}

pub fn track_popup_menu(
    hmenu: HMENU,
    uflags: TRACK_POPUP_MENU_FLAGS,
    x: i32,
    y: i32,
    nreserved: i32,
    hwnd: HWND,
    prcrect: Option<&RECT>,
) -> BOOL {
    unsafe {
        TrackPopupMenu(
            hmenu,
            uflags,
            x,
            y,
            nreserved,
            hwnd,
            match prcrect {
                Some(prcrect) => prcrect,
                None => std::ptr::null(),
            },
        )
    }
}

pub fn translate_message(msg: &MSG) -> BOOL {
    unsafe { TranslateMessage(msg) }
}

pub fn unhook_windows_hook_ex(hhk: HHOOK) -> eyre::Result<BOOL> {
    unsafe { UnhookWindowsHookEx(hhk).into_result() }
}

pub fn wts_register_session_notification(hwnd: HWND, dwflags: u32) -> eyre::Result<BOOL> {
    unsafe { WTSRegisterSessionNotification(hwnd, dwflags).into_result() }
}

pub fn wts_unregister_session_notification(hwnd: HWND) -> eyre::Result<BOOL> {
    unsafe { WTSUnRegisterSessionNotification(hwnd).into_result() }
}
