use crate::cardinal::Cardinal;
use crate::hotkey_action::{HotkeyAction, VK};
use crate::monitor;
use crate::CHECK_BOOL;
use crate::CHECK_HRESULT;
use crate::CHECK_HWND;
use winapi::shared::minwindef::BOOL;
use winapi::shared::windef::{HWND, RECT};
use winapi::shared::winerror::S_OK;
use winapi::um::dwmapi::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use winapi::um::winuser::*;

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

type WorkAreaToWindowPosFn = dyn Fn(&RECT) -> RECT;

pub fn add_actions(actions: &mut Vec<HotkeyAction>) {
    actions.extend_from_slice(&[
        HotkeyAction::new(top_left, &[VK::LeftWindows, VK::Numpad7]),
        HotkeyAction::new(top_left, &[VK::LeftWindows, VK::N1]),
        HotkeyAction::new(top_right, &[VK::LeftWindows, VK::Numpad9]),
        HotkeyAction::new(top_right, &[VK::LeftWindows, VK::N2]),
        HotkeyAction::new(bottom_left, &[VK::LeftWindows, VK::Numpad1]),
        HotkeyAction::new(bottom_left, &[VK::LeftWindows, VK::N3]),
        HotkeyAction::new(bottom_right, &[VK::LeftWindows, VK::Numpad3]),
        HotkeyAction::new(bottom_right, &[VK::LeftWindows, VK::N4]),
        HotkeyAction::new(west, &[VK::LeftWindows, VK::Numpad4]),
        HotkeyAction::new(west, &[VK::LeftWindows, VK::N7]),
        HotkeyAction::new(east, &[VK::LeftWindows, VK::Numpad6]),
        HotkeyAction::new(east, &[VK::LeftWindows, VK::N8]),
        HotkeyAction::new(north, &[VK::LeftWindows, VK::Numpad8]),
        HotkeyAction::new(north, &[VK::LeftWindows, VK::N5]),
        HotkeyAction::new(south, &[VK::LeftWindows, VK::Numpad2]),
        HotkeyAction::new(south, &[VK::LeftWindows, VK::N6]),
        HotkeyAction::new(maximize, &[VK::LeftWindows, VK::Up]),
        HotkeyAction::new(minimize, &[VK::LeftWindows, VK::Down]),
        HotkeyAction::new(clear_topmost, &[VK::LeftWindows, VK::LeftShift, VK::Z]),
    ]);
}

pub fn set_window_rect(hwnd: HWND, position: &RECT, flags: u32) -> BOOL {
    unsafe {
        let text_length = GetWindowTextLengthW(hwnd) + 1;
        let mut chars = Vec::new();
        chars.resize(text_length as usize, 0);
        GetWindowTextW(hwnd, chars.as_mut_ptr(), chars.len() as i32);
        let title = OsString::from_wide(chars.as_slice());
        println!("Positioning '{}'", title.into_string().unwrap());
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

fn calculate_margin(hwnd: HWND) -> RECT {
    let mut window_rect = RECT::default();
    let mut extended_frame_bounds = RECT::default();

    unsafe {
        CHECK_BOOL!(GetWindowRect(hwnd, &mut window_rect));
        CHECK_HRESULT!(DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut extended_frame_bounds as *mut RECT as *mut winapi::ctypes::c_void,
            std::mem::size_of_val(&extended_frame_bounds) as u32,
        ));
    }

    RECT {
        left: window_rect.left - extended_frame_bounds.left,
        right: window_rect.right - extended_frame_bounds.right,
        top: window_rect.top - extended_frame_bounds.top,
        bottom: window_rect.bottom - extended_frame_bounds.bottom,
    }
}

fn set_window_pos_action(workarea_to_window_pos: &WorkAreaToWindowPosFn) {
    unsafe {
        let foreground_window = CHECK_HWND!(GetForegroundWindow());
        let mut monitor_info = monitor::init_monitor_info();
        CHECK_BOOL!(GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        ));
        let window_pos = workarea_to_window_pos(&monitor_info.rcWork);
        CHECK_BOOL!(set_window_rect(foreground_window, &window_pos, SWP_NOZORDER));
        CHECK_BOOL!(SetCursorPos(window_pos.center().x, window_pos.center().y));
    }
}

fn top_left() {
    set_window_pos_action(&|r| RECT::from_points(r.top_left(), r.center()));
}

fn top_right() {
    set_window_pos_action(&|r| RECT::from_points(r.top_right(), r.center()));
}

fn bottom_left() {
    set_window_pos_action(&|r| RECT::from_points(r.bottom_left(), r.center()));
}

fn bottom_right() {
    set_window_pos_action(&|r| RECT::from_points(r.bottom_right(), r.center()));
}

fn west() {
    set_window_pos_action(&|r| RECT::from_points(r.top_left(), r.south()));
}

fn east() {
    set_window_pos_action(&|r| RECT::from_points(r.top_right(), r.south()));
}

fn north() {
    set_window_pos_action(&|r| RECT::from_points(r.top_left(), r.east()));
}

fn south() {
    set_window_pos_action(&|r| RECT::from_points(r.bottom_left(), r.east()));
}

fn maximize() {
    unsafe {
        let foreground_window = CHECK_HWND!(GetForegroundWindow());
        ShowWindowAsync(foreground_window, SW_MAXIMIZE);
    }
}

fn minimize() {
    unsafe {
        let foreground_window = CHECK_HWND!(GetForegroundWindow());
        ShowWindowAsync(foreground_window, SW_MINIMIZE);
    }
}

pub fn clear_topmost() -> () {
    unsafe {
        let foreground_window = CHECK_HWND!(GetForegroundWindow());
        CHECK_BOOL!(SetWindowPos(
            foreground_window,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE
        ));
    }
}
