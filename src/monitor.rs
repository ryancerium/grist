use crate::cardinal::Cardinal;
use crate::hotkey_action::{HotkeyAction, VK};
use crate::safe_win32::{enum_display_monitors, get_monitor_info, monitor_from_window, set_cursor_pos};
use crate::window_actions::{get_foreground_window_not_zoom, set_window_rect};
use bindings::Windows::Win32::Foundation::RECT;
use bindings::Windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTOPRIMARY;
use bindings::Windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;

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
        HotkeyAction::new("Move Next", move_to_next_monitor, &[VK::LeftWindows, VK::Numpad5]),
        HotkeyAction::new("Move Next", move_to_next_monitor, &[VK::LeftWindows, VK::Right]),
        HotkeyAction::new("Move Prev", move_to_prev_monitor, &[VK::LeftWindows, VK::Clear]),
        HotkeyAction::new("Move Prev", move_to_prev_monitor, &[VK::LeftWindows, VK::Left]),
    ]);
}

fn move_to_next_monitor() -> eyre::Result<()> {
    move_to_adjacent_monitor(Direction::Right)
}

fn move_to_prev_monitor() -> eyre::Result<()> {
    move_to_adjacent_monitor(Direction::Left)
}

fn move_to_adjacent_monitor(direction: Direction) -> eyre::Result<()> {
    let mut monitors = enum_display_monitors()?;

    monitors.sort_by(|lhs, rhs| match lhs.rcWork.left == rhs.rcWork.left {
        true => lhs.rcWork.top.cmp(&rhs.rcWork.top),
        false => lhs.rcWork.left.cmp(&rhs.rcWork.left),
    });

    let foreground_window = get_foreground_window_not_zoom()?;
    let monitor_info = get_monitor_info(monitor_from_window(foreground_window, MONITOR_DEFAULTTOPRIMARY)?)?;

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
    let _ = set_window_rect(foreground_window, &window_pos, SET_WINDOW_POS_FLAGS(0));
    let _ = set_cursor_pos(window_pos.center().x, window_pos.center().y);
    Ok(())
}
