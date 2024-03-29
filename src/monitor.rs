use crate::cardinal::Cardinal;
use crate::hotkey_action::{Action, HotkeyAction, VK};
use crate::safe_win32::{
    enum_display_monitors, get_foreground_window, get_monitor_info, monitor_from_window, set_cursor_pos,
};
use crate::window_actions::set_window_rect;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTOPRIMARY;
use windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;

enum Direction {
    Left,
    Right,
}

impl Direction {
    fn apply(&self, i: usize, len: usize) -> usize {
        if len < 2 {
            return 0;
        }

        match (self, i) {
            (Direction::Left, 0) => len - 1,
            (Direction::Left, _) => i - 1,
            (Direction::Right, _) => (i + 1) % len,
        }
    }
}

pub fn add_actions(actions: &mut Vec<HotkeyAction>) {
    actions.extend_from_slice(&[
        HotkeyAction::new("Move Next", Action::MoveNextMonitor, &[VK::LeftWindows, VK::Numpad5]),
        HotkeyAction::new("Move Next", Action::MoveNextMonitor, &[VK::LeftWindows, VK::Right]),
        HotkeyAction::new("Move Prev", Action::MovePrevMonitor, &[VK::LeftWindows, VK::Clear]),
        HotkeyAction::new("Move Prev", Action::MovePrevMonitor, &[VK::LeftWindows, VK::Left]),
    ]);
}

pub fn move_to_next_monitor() -> eyre::Result<()> {
    move_to_adjacent_monitor(Direction::Right)
}

pub fn move_to_prev_monitor() -> eyre::Result<()> {
    move_to_adjacent_monitor(Direction::Left)
}

fn move_to_adjacent_monitor(direction: Direction) -> eyre::Result<()> {
    let mut monitors = enum_display_monitors()?;

    monitors.sort_by(|lhs, rhs| match lhs.rcWork.left == rhs.rcWork.left {
        true => lhs.rcWork.top.cmp(&rhs.rcWork.top),
        false => lhs.rcWork.left.cmp(&rhs.rcWork.left),
    });

    let foreground_window = get_foreground_window()?;
    let monitor_info = get_monitor_info(monitor_from_window(foreground_window, MONITOR_DEFAULTTOPRIMARY)?)?;

    let i = direction.apply(
        monitors.iter().position(|&m| m.rcWork == monitor_info.rcWork).unwrap(),
        monitors.len(),
    );

    let work_area = monitors[i].rcWork;
    let window_pos = RECT::from_points(work_area.top_left(), work_area.center());
    let _ = set_window_rect(foreground_window, &window_pos, SET_WINDOW_POS_FLAGS::default());
    let _ = set_cursor_pos(window_pos.center().x, window_pos.center().y);
    Ok(())
}
