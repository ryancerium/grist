// #![windows_subsystem = "windows"]
// Uncomment the above line to make a windowed app instead of a console app

mod cardinal;
mod hotkey_action;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate num_derive;

use bitarray::BitArray;
use cardinal::Cardinal;
use hotkey_action::{HotkeyAction, VK};
//use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use typenum::U256;
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{BOOL, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HDC, HMONITOR, HWND, LPRECT, RECT};
use winapi::um::dwmapi::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW,
};
use winapi::um::wincon::GetConsoleWindow;
use winapi::um::winuser::{
    CallNextHookEx, DispatchMessageW, EnumDisplayMonitors, GetForegroundWindow, GetMessageW,
    GetMonitorInfoW, GetWindowRect, LoadIconW, MonitorFromWindow, SetCursorPos, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TranslateMessage, IDI_APPLICATION, KBDLLHOOKSTRUCT, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, MSG, SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WM_KEYDOWN,
    WM_SYSKEYDOWN,
};

lazy_static! {
    static ref ACTIONS: Mutex<Vec<HotkeyAction>> = Mutex::default();
}

lazy_static! {
    static ref PRESSED_KEYS: Mutex<BitArray<u32, U256>> =
        Mutex::new(BitArray::<u32, U256>::from_elem(false));
}

// static mut DEBUG: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: c_int,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        println!("low_level_keyboard_proc(): ncode < 0");
        return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
    }

    let key_action = w_param as UINT;
    let kbdllhookstruct = &*(l_param as *const KBDLLHOOKSTRUCT);

    let mut pressed_keys = PRESSED_KEYS.lock().unwrap();

    let key_down = key_action == WM_KEYDOWN || key_action == WM_SYSKEYDOWN;
    pressed_keys.set(kbdllhookstruct.vkCode as usize, key_down);

    /*if key_down {
        let s = pressed_keys
            .iter()
            .enumerate()
            .filter(|(_, pressed)| *pressed)
            .map(|(i, _)| i)
            .fold(String::new(), |mut s, i| {
                let key: VK = num::FromPrimitive::from_usize(i).unwrap();
                match std::fmt::write(&mut s, format_args!("{:?} ", key)) {
                    Ok(()) => s,
                    Err(_) => s,
                }
            });

        println!("{}", s);
    }*/

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
    let mut monitor_info: MONITORINFO = Default::default();
    monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    monitor_info
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
    1
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

        i = if i == -1 {
            monitors.len() as i32 - 1
        } else if i == monitors.len() as i32 {
            0
        } else {
            i
        };

        let work_area = monitors[i as usize].rcWork;
        let window_pos = RECT::from_points(work_area.top_left(), work_area.center());
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
    let mut window_rect = RECT::default();
    let mut extended_frame_bounds = RECT::default();

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

type WorkAreaToWindowPosFn = dyn Fn(&RECT) -> RECT;

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

fn minimize() {
    unsafe {
        let foreground_window = GetForegroundWindow();
        ShowWindow(foreground_window, SW_MINIMIZE);
    }
}

fn maximize() {
    unsafe {
        let foreground_window = GetForegroundWindow();
        ShowWindow(foreground_window, SW_MAXIMIZE);
    }
}

fn set_actions() {
    let mut actions = ACTIONS.lock().unwrap();

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
        HotkeyAction::new(move_to_next_monitor, &[VK::LeftWindows, VK::Numpad5]),
        HotkeyAction::new(move_to_next_monitor, &[VK::LeftWindows, VK::Right]),
        HotkeyAction::new(move_to_prev_monitor, &[VK::LeftWindows, VK::Clear]),
        HotkeyAction::new(move_to_prev_monitor, &[VK::LeftWindows, VK::Left]),
        HotkeyAction::new(maximize, &[VK::LeftWindows, VK::Up]),
        HotkeyAction::new(minimize, &[VK::LeftWindows, VK::Down]),
        HotkeyAction::new(|| {}, &[VK::LeftWindows, VK::K]),
    ]);
}

fn set_notify_icon() {
    // to navigate calling with the winapi "crate" use the search function at link
    // https://docs.rs/winapi/*/x86_64-pc-windows-msvc/winapi/um/wincon/fn.GetConsoleWindow.html

    let wm_mymessage = winapi::um::winuser::WM_APP + 100; //prep WM_MYMESSAGE
    let tooltip: Vec<u16> = "Tool tip words here\0".to_string().encode_utf16().collect();

    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = unsafe { GetConsoleWindow() };
    nid.uID = 1001;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = wm_mymessage;
    nid.hIcon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    let tooltip_len = std::cmp::min(nid.szTip.len(), tooltip.len());
    nid.szTip[..tooltip_len].clone_from_slice(&tooltip[..tooltip_len]);

    unsafe { Shell_NotifyIconW(NIM_ADD, &mut nid) };
}

fn main() {
    set_actions();
    //set_notify_icon();

    let mut msg = MSG::default();

    unsafe {
        SetWindowsHookExW(
            13,
            Some(low_level_keyboard_proc),
            GetModuleHandleW(std::ptr::null()),
            0,
        );

        println!("Press any hotkey...");

        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    println!("Good bye!");
}
