use crate::cardinal::Cardinal;
use crate::hotkey_action::{HotkeyAction, VK};
use crate::{PRINT_STYLE, monitor};
use bindings::Windows::Win32::Foundation::{BOOL, HWND, RECT};
use bindings::Windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use bindings::Windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY,
};
use bindings::Windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, SetCursorPos, SetWindowPos, ShowWindow,
    ShowWindowAsync, GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, SET_WINDOW_POS_FLAGS, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WINDOW_EX_STYLE, WINDOW_STYLE,
    WS_BORDER, WS_CAPTION, WS_CHILD, WS_CHILDWINDOW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_DISABLED,
    WS_DLGFRAME, WS_EX_ACCEPTFILES, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_COMPOSITED,
    WS_EX_CONTEXTHELP, WS_EX_CONTROLPARENT, WS_EX_DLGMODALFRAME, WS_EX_LAYERED, WS_EX_LAYOUTRTL,
    WS_EX_LEFT, WS_EX_LEFTSCROLLBAR, WS_EX_LTRREADING, WS_EX_MDICHILD, WS_EX_NOACTIVATE,
    WS_EX_NOINHERITLAYOUT, WS_EX_NOPARENTNOTIFY, WS_EX_NOREDIRECTIONBITMAP, WS_EX_RIGHT,
    WS_EX_RIGHTSCROLLBAR, WS_EX_RTLREADING, WS_EX_STATICEDGE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_EX_WINDOWEDGE, WS_GROUP, WS_HSCROLL, WS_ICONIC, WS_MAXIMIZE,
    WS_MAXIMIZEBOX, WS_MINIMIZE, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_POPUPWINDOW,
    WS_SIZEBOX, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_TILED, WS_VISIBLE, WS_VSCROLL,
};

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
        HotkeyAction::new(print_window_flags, &[VK::LeftWindows, VK::LeftShift, VK::F]),
    ]);
}

pub fn set_window_rect(hwnd: HWND, position: &RECT, flags: SET_WINDOW_POS_FLAGS) -> BOOL {
    unsafe {
        println!("Positioning '{}'", crate::ui::get_window_text(hwnd));
        ShowWindow(hwnd, SW_RESTORE);

        let margin = calculate_margin(hwnd);
        SetWindowPos(
            hwnd,
            HWND::NULL,
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
        GetWindowRect(hwnd, &mut window_rect);
        let success = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS.0 as u32,
            &mut extended_frame_bounds as *mut RECT as *mut core::ffi::c_void,
            std::mem::size_of_val(&extended_frame_bounds) as u32,
        );
        match success {
            Ok(_) => (),
            Err(e) => println!("{}", e.message()),
        }
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
        let foreground_window = GetForegroundWindow();
        if foreground_window.is_null() {
            return;
        }
        let mut monitor_info = monitor::init_monitor_info();

        let success = GetMonitorInfoW(
            MonitorFromWindow(foreground_window, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor_info,
        );
        if !success.as_bool() {
            return;
        }

        let window_pos = workarea_to_window_pos(&monitor_info.rcWork);

        let success = set_window_rect(foreground_window, &window_pos, SWP_NOZORDER);
        if !success.as_bool() {
            return;
        }
        SetCursorPos(window_pos.center().x, window_pos.center().y);
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
        let foreground_window = GetForegroundWindow();
        if !foreground_window.is_null() {
            ShowWindowAsync(foreground_window, SW_MAXIMIZE.0 as i32);
        }
    }
}

fn minimize() {
    unsafe {
        let foreground_window = GetForegroundWindow();
        if !foreground_window.is_null() {
            ShowWindowAsync(foreground_window, SW_MINIMIZE.0 as i32);
        }
    }
}

pub fn clear_topmost() {
    unsafe {
        let foreground_window = GetForegroundWindow();
        if foreground_window.is_null() {
            return;
        }
        SetWindowPos(
            foreground_window,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        );
    }
}

pub fn print_window_flags() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return;
        }
        println!("Styles for '{}'", crate::ui::get_window_text(hwnd));
        let styles = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
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

        let styles = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
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
    }
}
