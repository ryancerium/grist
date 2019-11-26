use crate::cardinal::Cardinal;
use crate::hotkey_action::{HotkeyAction, VK};
use crate::window_actions::set_window_rect;
use crate::CHECK_BOOL;
use winapi::shared::minwindef::{BOOL, LPARAM};
use winapi::shared::windef::{HDC, HMONITOR, LPRECT, RECT};
use winapi::um::winuser::*;

enum Increment {
    Left = -1,
    Right = 1,
}

pub fn add_actions(actions: &mut Vec<HotkeyAction>) {
    actions.extend_from_slice(&[
        HotkeyAction::new(move_to_next_monitor, &[VK::LeftWindows, VK::Numpad5]),
        HotkeyAction::new(move_to_next_monitor, &[VK::LeftWindows, VK::Right]),
        HotkeyAction::new(move_to_prev_monitor, &[VK::LeftWindows, VK::Clear]),
        HotkeyAction::new(move_to_prev_monitor, &[VK::LeftWindows, VK::Left]),
    ]);
}

pub fn init_monitor_info() -> MONITORINFO {
    let mut monitor_info = MONITORINFO::default();
    monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    monitor_info
}

fn move_to_next_monitor() {
    move_to_adjacent_monitor(Increment::Right);
}

fn move_to_prev_monitor() {
    move_to_adjacent_monitor(Increment::Left);
}

unsafe extern "system" fn enum_display_monitors_callback(
    h_monitor: HMONITOR,
    _hdc: HDC,
    _rect: LPRECT,
    _dw_data: LPARAM,
) -> BOOL {
    let mut monitor_info = init_monitor_info();
    CHECK_BOOL!(GetMonitorInfoW(h_monitor, &mut monitor_info));
    let monitors = &mut *(_dw_data as *mut Vec<MONITORINFO>);
    monitors.push(monitor_info);
    1
}

fn move_to_adjacent_monitor(increment: Increment) {
    let mut monitors: Vec<MONITORINFO> = Vec::new();
    unsafe {
        CHECK_BOOL!(EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_display_monitors_callback),
            &mut monitors as *mut Vec<MONITORINFO> as isize,
        ));
    }

    monitors.sort_by(|lhs, rhs| match lhs.rcWork.left == rhs.rcWork.left {
        true => lhs.rcWork.top.cmp(&rhs.rcWork.top),
        false => lhs.rcWork.left.cmp(&rhs.rcWork.left),
    });

    unsafe {
        let foreground_window = GetForegroundWindow();
        let mut monitor_info = init_monitor_info();
        CHECK_BOOL!(GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        ));

        let mut i = monitors
            .iter()
            .position(|&m| {
                m.rcWork.left == monitor_info.rcWork.left
                    && m.rcWork.right == monitor_info.rcWork.right
                    && m.rcWork.top == monitor_info.rcWork.top
                    && m.rcWork.bottom == monitor_info.rcWork.bottom
            })
            .unwrap() as i32
            + increment as i32;

        i = if i < 0 {
            monitors.len() as i32 - 1
        } else if i >= monitors.len() as i32 {
            0
        } else {
            i
        };

        let work_area = monitors[i as usize].rcWork;
        let window_pos = RECT::from_points(work_area.top_left(), work_area.center());
        set_window_rect(foreground_window, &window_pos, 0);
        CHECK_BOOL!(SetCursorPos(window_pos.center().x, window_pos.center().y));
    }
}
