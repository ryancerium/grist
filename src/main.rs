mod cardinal;
mod hotkey_action;

#[macro_use]
extern crate lazy_static;

use bitarray::BitArray;
use cardinal::{default_rect, make_rect, Cardinal};
use hotkey_action::{HotkeyAction, VK};
use std::sync::Mutex;
use typenum::U256;
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{BOOL, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HDC, HMONITOR, HWND, LPRECT, RECT};
use winapi::um::dwmapi::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    CallNextHookEx, DispatchMessageW, EnumDisplayMonitors, GetForegroundWindow, GetMessageW,
    GetMonitorInfoW, GetWindowRect, MonitorFromWindow, SetCursorPos, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TranslateMessage, KBDLLHOOKSTRUCT, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, MSG, SWP_NOZORDER, SW_RESTORE, WM_KEYDOWN, WM_SYSKEYDOWN,
};

lazy_static! {
    static ref ACTIONS: Mutex<Vec<HotkeyAction>> = Mutex::default();
}

lazy_static! {
    static ref PRESSED_KEYS: Mutex<BitArray<u32, U256>> =
        Mutex::new(BitArray::<u32, U256>::from_elem(false));
}

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: c_int,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
    }

    let key_action = w_param as UINT;
    let kbdllhookstruct = *(l_param as *const KBDLLHOOKSTRUCT);

    let mut pressed_keys = PRESSED_KEYS.lock().unwrap();
    let key_down = key_action == WM_KEYDOWN || key_action == WM_SYSKEYDOWN;
    pressed_keys.set(kbdllhookstruct.vkCode as usize, key_down);

    match ACTIONS
        .lock()
        .unwrap()
        .iter()
        .find(|hotkey_action| hotkey_action.matches(&pressed_keys))
    {
        Some(action) => {
            (action.action)();
            1
        }
        None => CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param),
    }
}

fn init_monitor_info() -> MONITORINFO {
    unsafe {
        let mut monitor_info: MONITORINFO = std::mem::zeroed();
        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        monitor_info
    }
}

unsafe extern "system" fn enum_display_monitors_callback(
    h_monitor: HMONITOR,
    _hdc: HDC,
    _rect: LPRECT,
    _dw_data: LPARAM,
) -> BOOL {
    let mut monitor_info = init_monitor_info();
    GetMonitorInfoW(h_monitor, &mut monitor_info);
    let monitors = &mut *(_dw_data as *mut Vec<MONITORINFO>);
    monitors.push(monitor_info);
    return 1;
}

fn move_to_adjacent_monitor(increment: i32) {
    let mut monitors: Vec<MONITORINFO> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_display_monitors_callback),
            &mut monitors as *mut Vec<MONITORINFO> as isize,
        );
    }

    monitors.sort_by(|lhs, rhs| match lhs.rcWork.left == rhs.rcWork.left {
        true => lhs.rcWork.top.cmp(&rhs.rcWork.top),
        false => lhs.rcWork.left.cmp(&rhs.rcWork.left),
    });

    unsafe {
        let foreground_window = GetForegroundWindow();
        let mut monitor_info = init_monitor_info();
        GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        );

        let mut i = monitors
            .iter()
            .position(|&m| {
                m.rcWork.left == monitor_info.rcWork.left
                    && m.rcWork.right == monitor_info.rcWork.right
                    && m.rcWork.top == monitor_info.rcWork.top
                    && m.rcWork.bottom == monitor_info.rcWork.bottom
            })
            .unwrap() as i32
            + increment;

        if i == -1 {
            i = monitors.len() as i32 - 1;
        } else if i == monitors.len() as i32 {
            i = 0;
        }

        let work_area = monitors[i as usize].rcWork;
        let window_pos = make_rect(&work_area.top_left(), &work_area.center());
        set_window_rect(foreground_window, &window_pos, 0);
        SetCursorPos(window_pos.center().x, window_pos.center().y);
    }
}

fn move_to_next_monitor() {
    move_to_adjacent_monitor(1);
}

fn move_to_prev_monitor() {
    move_to_adjacent_monitor(-1);
}

fn calculate_margin(hwnd: HWND) -> RECT {
    let mut window_rect: RECT = default_rect();
    let mut extended_frame_bounds = default_rect();

    unsafe {
        GetWindowRect(hwnd, &mut window_rect);
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut extended_frame_bounds as *mut RECT as *mut winapi::ctypes::c_void,
            std::mem::size_of_val(&extended_frame_bounds) as u32,
        );
    }

    RECT {
        left: window_rect.left - extended_frame_bounds.left,
        right: window_rect.right - extended_frame_bounds.right,
        top: window_rect.top - extended_frame_bounds.top,
        bottom: window_rect.bottom - extended_frame_bounds.bottom,
    }
}

fn set_window_rect(hwnd: HWND, position: &RECT, flags: u32) -> BOOL {
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        let margin = calculate_margin(hwnd);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            position.left + margin.left,
            position.top + margin.top,
            position.width() + margin.right - margin.left,
            position.height() + margin.bottom - margin.top,
            flags,
        )
    }
}

type WorkAreaToWindowPosFn = Fn(&RECT) -> RECT;

fn set_window_pos_action(workarea_to_window_pos: &WorkAreaToWindowPosFn) {
    unsafe {
        let foreground_window = GetForegroundWindow();
        let mut monitor_info = init_monitor_info();
        GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        );
        let window_pos = workarea_to_window_pos(&monitor_info.rcWork);
        set_window_rect(foreground_window, &window_pos, SWP_NOZORDER);
        SetCursorPos(window_pos.center().x, window_pos.center().y);
    }
}

fn top_left() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_left(), &r.center()));
}

fn top_right() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_right(), &r.center()));
}

fn bottom_left() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.bottom_left(), &r.center()));
}

fn bottom_right() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.bottom_right(), &r.center()));
}

fn west() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_left(), &r.south()));
}

fn east() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_right(), &r.south()));
}

fn north() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_left(), &r.east()));
}

fn south() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.bottom_left(), &r.east()));
}

fn set_actions() {
    let mut actions = ACTIONS.lock().unwrap();

    actions.extend_from_slice(&[
        HotkeyAction::new(top_left, &[VK::LeftWindows, VK::Numpad7]),
        HotkeyAction::new(top_left, &[VK::LeftWindows, VK::Numpad7]),
        HotkeyAction::new(top_right, &[VK::LeftWindows, VK::Numpad9]),
        HotkeyAction::new(bottom_left, &[VK::LeftWindows, VK::Numpad1]),
        HotkeyAction::new(bottom_right, &[VK::LeftWindows, VK::Numpad3]),
        HotkeyAction::new(west, &[VK::LeftWindows, VK::Numpad4]),
        HotkeyAction::new(east, &[VK::LeftWindows, VK::Numpad6]),
        HotkeyAction::new(north, &[VK::LeftWindows, VK::Numpad8]),
        HotkeyAction::new(south, &[VK::LeftWindows, VK::Numpad2]),
        HotkeyAction::new(move_to_next_monitor, &[VK::LeftWindows, VK::Numpad5]),
        HotkeyAction::new(move_to_prev_monitor, &[VK::LeftWindows, VK::Clear]),
    ]);
}

fn main() {
    set_actions();

    unsafe {
        SetWindowsHookExW(
            13,
            Option::Some(low_level_keyboard_proc),
            GetModuleHandleW(std::ptr::null()),
            0,
        );

        let mut msg: MSG = std::mem::zeroed();

        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
