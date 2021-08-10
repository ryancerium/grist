use crate::cardinal::Cardinal;
use crate::hotkey_action::{HotkeyAction, VK};
use crate::window_actions::set_window_rect;
use bindings::Windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use bindings::Windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use bindings::Windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetCursorPos, SET_WINDOW_POS_FLAGS};

enum Direction {
    Left,
    Right,
}

impl Direction {
    fn apply(&self, i: usize, len: usize) -> usize {
        if len < 2 {
            0
        } else {
            match self {
                Direction::Left => return if i == 0 { len - 1 } else { i - 1 },
                Direction::Right => return if i == (len - 1) { 0 } else { i + 1 },
            }
        }
    }
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
    move_to_adjacent_monitor(Direction::Right);
}

fn move_to_prev_monitor() {
    move_to_adjacent_monitor(Direction::Left);
}

unsafe extern "system" fn enum_display_monitors_callback(
    h_monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    _dw_data: LPARAM,
) -> BOOL {
    let mut monitor_info = init_monitor_info();

    let success = GetMonitorInfoW(h_monitor, &mut monitor_info);
    if !success.as_bool() {
        return false.into();
    }

    let monitors = &mut *(_dw_data.0 as *mut Vec<MONITORINFO>);
    monitors.push(monitor_info);
    true.into()
}

fn move_to_adjacent_monitor(direction: Direction) {
    let mut monitors: Vec<MONITORINFO> = Vec::new();
    unsafe {
        let success = EnumDisplayMonitors(
            HDC::NULL,
            std::ptr::null_mut(),
            Some(enum_display_monitors_callback),
            LPARAM(&mut monitors as *mut Vec<MONITORINFO> as isize),
        );
        if !success.as_bool() {
            return;
        }
    }

    monitors.sort_by(|lhs, rhs| match lhs.rcWork.left == rhs.rcWork.left {
        true => lhs.rcWork.top.cmp(&rhs.rcWork.top),
        false => lhs.rcWork.left.cmp(&rhs.rcWork.left),
    });

    unsafe {
        let foreground_window = GetForegroundWindow();
        if foreground_window.is_null() {
            return;
        }
        let mut monitor_info = init_monitor_info();
        let success = GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        );
        if !success.as_bool() {
            return;
        }

        let i = direction.apply(
            monitors
                .iter()
                .position(|&m| {
                    m.rcWork.left == monitor_info.rcWork.left
                        && m.rcWork.right == monitor_info.rcWork.right
                        && m.rcWork.top == monitor_info.rcWork.top
                        && m.rcWork.bottom == monitor_info.rcWork.bottom
                })
                .unwrap(),
            monitors.len(),
        );

        let work_area = monitors[i].rcWork;
        let window_pos = RECT::from_points(work_area.top_left(), work_area.center());
        set_window_rect(foreground_window, &window_pos, SET_WINDOW_POS_FLAGS(0));
        SetCursorPos(window_pos.center().x, window_pos.center().y);
    }
}
