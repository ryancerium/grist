use crate::cardinal::Cardinal;
use crate::hotkey_action::{HotkeyAction, VK};
use crate::safe_win32::*;
use crate::PRINT_STYLE;
use eyre::eyre;
use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTOPRIMARY;
use windows::Win32::System::Threading::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, SET_WINDOW_POS_FLAGS, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, WS_BORDER, WS_CAPTION, WS_CHILD, WS_CHILDWINDOW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
    WS_DISABLED, WS_DLGFRAME, WS_EX_ACCEPTFILES, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_COMPOSITED,
    WS_EX_CONTEXTHELP, WS_EX_CONTROLPARENT, WS_EX_DLGMODALFRAME, WS_EX_LAYERED, WS_EX_LAYOUTRTL, WS_EX_LEFT,
    WS_EX_LEFTSCROLLBAR, WS_EX_LTRREADING, WS_EX_MDICHILD, WS_EX_NOACTIVATE, WS_EX_NOINHERITLAYOUT,
    WS_EX_NOPARENTNOTIFY, WS_EX_NOREDIRECTIONBITMAP, WS_EX_RIGHT, WS_EX_RIGHTSCROLLBAR, WS_EX_RTLREADING,
    WS_EX_STATICEDGE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_EX_WINDOWEDGE, WS_GROUP, WS_HSCROLL,
    WS_ICONIC, WS_MAXIMIZE, WS_MAXIMIZEBOX, WS_MINIMIZE, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_POPUPWINDOW,
    WS_SIZEBOX, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_TILED, WS_VISIBLE, WS_VSCROLL,
};

type WorkAreaToWindowPosFn = dyn Fn(&RECT) -> RECT;

fn get_window_executable(hwnd: HWND) -> eyre::Result<String> {
    let tpid = get_window_thread_process_id(hwnd);
    let process_handle = open_process(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, tpid.process_id)?;
    let filename = get_module_file_name(process_handle);
    let _ = close_handle(process_handle);
    filename
}

pub fn get_foreground_window_not_zoom() -> eyre::Result<HWND> {
    let hwnd = get_foreground_window()?;
    let filename = get_window_executable(hwnd)?;
    if filename.contains("Zoom.exe") {
        return Err(eyre!("Don't position Zoom.exe"));
    }
    Ok(hwnd)
}

pub fn add_actions(actions: &mut Vec<HotkeyAction>) {
    actions.extend_from_slice(&[
        HotkeyAction::new("Top Left", top_left, &[VK::LeftWindows, VK::Numpad7]),
        HotkeyAction::new("Top Left", top_left, &[VK::LeftWindows, VK::N1]),
        HotkeyAction::new("Top Right", top_right, &[VK::LeftWindows, VK::Numpad9]),
        HotkeyAction::new("Top Right", top_right, &[VK::LeftWindows, VK::N2]),
        HotkeyAction::new("Bottom Left", bottom_left, &[VK::LeftWindows, VK::Numpad1]),
        HotkeyAction::new("Bottom Left", bottom_left, &[VK::LeftWindows, VK::N3]),
        HotkeyAction::new("Bottom Right", bottom_right, &[VK::LeftWindows, VK::Numpad3]),
        HotkeyAction::new("Bottom Right", bottom_right, &[VK::LeftWindows, VK::N4]),
        HotkeyAction::new("West", west, &[VK::LeftWindows, VK::Numpad4]),
        HotkeyAction::new("West", west, &[VK::LeftWindows, VK::N7]),
        HotkeyAction::new("East", east, &[VK::LeftWindows, VK::Numpad6]),
        HotkeyAction::new("East", east, &[VK::LeftWindows, VK::N8]),
        HotkeyAction::new("North", north, &[VK::LeftWindows, VK::Numpad8]),
        HotkeyAction::new("North", north, &[VK::LeftWindows, VK::N5]),
        HotkeyAction::new("South", south, &[VK::LeftWindows, VK::Numpad2]),
        HotkeyAction::new("South", south, &[VK::LeftWindows, VK::N6]),
        HotkeyAction::new("Maximize", maximize, &[VK::LeftWindows, VK::Up]),
        HotkeyAction::new("Minimize", minimize, &[VK::LeftWindows, VK::Down]),
        HotkeyAction::new("Clear Topmost Flag", clear_topmost, &[VK::LeftWindows, VK::LeftShift, VK::Z]),
        HotkeyAction::new("Print Flags", print_window_flags, &[VK::LeftWindows, VK::LeftShift, VK::F]),
    ]);
}

pub fn set_window_rect(hwnd: HWND, position: &RECT, flags: SET_WINDOW_POS_FLAGS) -> eyre::Result<BOOL> {
    let _ = show_window(hwnd, SW_RESTORE)?;

    let margin = calculate_margin(hwnd)?;
    set_window_pos(
        hwnd,
        HWND::default(),
        position.left + margin.left,
        position.top + margin.top,
        position.width() + margin.right - margin.left,
        position.height() + margin.bottom - margin.top,
        flags,
    )
}

fn calculate_margin(hwnd: HWND) -> eyre::Result<RECT> {
    let window_rect = get_window_rect(hwnd)?;
    let extended_frame_bounds = dwm_get_window_attribute_extended_frame_bounds(hwnd)?;

    Ok(RECT {
        left: window_rect.left - extended_frame_bounds.left,
        right: window_rect.right - extended_frame_bounds.right,
        top: window_rect.top - extended_frame_bounds.top,
        bottom: window_rect.bottom - extended_frame_bounds.bottom,
    })
}

fn set_window_pos_action(workarea_to_window_pos: &WorkAreaToWindowPosFn) -> eyre::Result<()> {
    let foreground_window = get_foreground_window()?;
    let monitor_info = get_monitor_info(monitor_from_window(foreground_window, MONITOR_DEFAULTTOPRIMARY)?)?;
    let new_window_pos = workarea_to_window_pos(&monitor_info.rcWork);
    let _ = set_window_rect(foreground_window, &new_window_pos, SWP_NOZORDER)?;

    if point_in_rect(&new_window_pos, &get_cursor_pos()?) {
        Ok(())
    } else {
        set_cursor_pos(new_window_pos.center().x, new_window_pos.center().y).map(|_| ())
    }
}

fn top_left() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.top_left(), r.center()))
}

fn top_right() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.top_right(), r.center()))
}

fn bottom_left() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.bottom_left(), r.center()))
}

fn bottom_right() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.bottom_right(), r.center()))
}

fn west() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.top_left(), r.south()))
}

fn east() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.top_right(), r.south()))
}

fn north() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.top_left(), r.east()))
}

fn south() -> eyre::Result<()> {
    set_window_pos_action(&|r| RECT::from_points(r.bottom_left(), r.east()))
}

fn maximize() -> eyre::Result<()> {
    get_foreground_window()
        .and_then(|hwnd| show_window(hwnd, SW_MAXIMIZE))
        .map(|_| ())
}

fn minimize() -> eyre::Result<()> {
    get_foreground_window_not_zoom()
        .and_then(|hwnd| show_window(hwnd, SW_MINIMIZE))
        .map(|_| ())
}

pub fn clear_topmost() -> eyre::Result<()> {
    get_foreground_window()
        .and_then(|hwnd| set_window_pos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE))
        .map(|_| ())
}

pub fn print_window_flags() -> eyre::Result<()> {
    get_foreground_window()
        .and_then(|hwnd| {
            println!("Styles for '{}'", get_window_text(hwnd).unwrap_or_else(|_| "".to_owned()));
            let styles = get_window_long_ptr(hwnd, GWL_STYLE)? as u32;
            PRINT_STYLE!(styles, WS_BORDER);
            PRINT_STYLE!(styles, WS_CAPTION);
            PRINT_STYLE!(styles, WS_CHILD);
            PRINT_STYLE!(styles, WS_CHILDWINDOW);
            PRINT_STYLE!(styles, WS_CLIPCHILDREN);
            PRINT_STYLE!(styles, WS_CLIPSIBLINGS);
            PRINT_STYLE!(styles, WS_DISABLED);
            PRINT_STYLE!(styles, WS_DLGFRAME);
            PRINT_STYLE!(styles, WS_GROUP);
            PRINT_STYLE!(styles, WS_HSCROLL);
            PRINT_STYLE!(styles, WS_ICONIC);
            PRINT_STYLE!(styles, WS_MAXIMIZE);
            PRINT_STYLE!(styles, WS_MAXIMIZEBOX);
            PRINT_STYLE!(styles, WS_MINIMIZE);
            PRINT_STYLE!(styles, WS_MINIMIZEBOX);
            PRINT_STYLE!(styles, WS_OVERLAPPED);
            //PRINT_STYLE!(styles, WS_OVERLAPPEDWINDOW);
            PRINT_STYLE!(styles, WS_POPUP);
            PRINT_STYLE!(styles, WS_POPUPWINDOW);
            PRINT_STYLE!(styles, WS_SIZEBOX);
            PRINT_STYLE!(styles, WS_SYSMENU);
            PRINT_STYLE!(styles, WS_TABSTOP);
            PRINT_STYLE!(styles, WS_THICKFRAME);
            PRINT_STYLE!(styles, WS_TILED);
            //PRINT_STYLE!(styles, WS_TILEDWINDOW);
            PRINT_STYLE!(styles, WS_VISIBLE);
            PRINT_STYLE!(styles, WS_VSCROLL);

            let styles = get_window_long_ptr(hwnd, GWL_EXSTYLE)? as u32;
            PRINT_STYLE!(styles, WS_EX_ACCEPTFILES);
            PRINT_STYLE!(styles, WS_EX_APPWINDOW);
            PRINT_STYLE!(styles, WS_EX_CLIENTEDGE);
            PRINT_STYLE!(styles, WS_EX_COMPOSITED);
            PRINT_STYLE!(styles, WS_EX_CONTEXTHELP);
            PRINT_STYLE!(styles, WS_EX_CONTROLPARENT);
            PRINT_STYLE!(styles, WS_EX_DLGMODALFRAME);
            PRINT_STYLE!(styles, WS_EX_LAYERED);
            PRINT_STYLE!(styles, WS_EX_LAYOUTRTL);
            PRINT_STYLE!(styles, WS_EX_LEFT);
            PRINT_STYLE!(styles, WS_EX_LEFTSCROLLBAR);
            PRINT_STYLE!(styles, WS_EX_LTRREADING);
            PRINT_STYLE!(styles, WS_EX_MDICHILD);
            PRINT_STYLE!(styles, WS_EX_NOACTIVATE);
            PRINT_STYLE!(styles, WS_EX_NOINHERITLAYOUT);
            PRINT_STYLE!(styles, WS_EX_NOPARENTNOTIFY);
            PRINT_STYLE!(styles, WS_EX_NOREDIRECTIONBITMAP);
            //PRINT_STYLE!(styles, WS_EX_OVERLAPPEDWINDOW);
            //PRINT_STYLE!(styles, WS_EX_PALETTEWINDOW);
            PRINT_STYLE!(styles, WS_EX_RIGHT);
            PRINT_STYLE!(styles, WS_EX_RIGHTSCROLLBAR);
            PRINT_STYLE!(styles, WS_EX_RTLREADING);
            PRINT_STYLE!(styles, WS_EX_STATICEDGE);
            PRINT_STYLE!(styles, WS_EX_TOOLWINDOW);
            PRINT_STYLE!(styles, WS_EX_TOPMOST);
            PRINT_STYLE!(styles, WS_EX_TRANSPARENT);
            PRINT_STYLE!(styles, WS_EX_WINDOWEDGE);
            Ok(())
        })
        .map(|_| ())
}
