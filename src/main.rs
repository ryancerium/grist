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
use winapi::shared::windef::{HWND, RECT};
use winapi::um::dwmapi::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetMonitorInfoW,
    GetWindowRect, MonitorFromWindow, SetCursorPos, SetWindowPos, SetWindowsHookExW, ShowWindow,
    TranslateMessage, KBDLLHOOKSTRUCT, MONITORINFO, MONITOR_DEFAULTTOPRIMARY, MSG, SWP_NOZORDER,
    SW_RESTORE, WM_KEYDOWN, WM_SYSKEYDOWN,
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
        let mut monitor_info: MONITORINFO = std::mem::zeroed();
        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        );
        //println!("work_area: {}, {}", monitor_info.rcWork.left);
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

fn left() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_left(), &r.south()));
}

fn right() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_right(), &r.south()));
}

fn north() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.top_left(), &r.east()));
}

fn south() {
    set_window_pos_action(&|r: &RECT| make_rect(&r.bottom_left(), &r.east()));
}

fn set_actions() {
    let mut a = ACTIONS.lock().unwrap();

    a.push(HotkeyAction::new(top_left, &[VK::LeftWindows, VK::Numpad7]));
    a.push(HotkeyAction::new(
        top_right,
        &[VK::LeftWindows, VK::Numpad9],
    ));
    a.push(HotkeyAction::new(
        bottom_left,
        &[VK::LeftWindows, VK::Numpad1],
    ));
    a.push(HotkeyAction::new(
        bottom_right,
        &[VK::LeftWindows, VK::Numpad3],
    ));
    a.push(HotkeyAction::new(left, &[VK::LeftWindows, VK::Numpad4]));
    a.push(HotkeyAction::new(right, &[VK::LeftWindows, VK::Numpad6]));
    a.push(HotkeyAction::new(north, &[VK::LeftWindows, VK::Numpad8]));
    a.push(HotkeyAction::new(south, &[VK::LeftWindows, VK::Numpad2]));
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
