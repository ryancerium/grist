use crate::safe_win32::{
    call_next_hook, create_popup_menu, create_window, def_window_proc, destroy_icon, destroy_menu, get_module_handle,
    get_window_long_ptr, insert_menu, message_box, post_message, register_class, set_foreground_window,
    set_window_long_ptr, set_windows_hook, shell_notify_icon, track_popup_menu, unhook_windows_hook_ex,
    wts_register_session_notification, wts_unregister_session_notification,
};
use crate::{hotkey_action, msg, print_pressed_keys, ACTIONS, DEBUG, PRESSED_KEYS};
use num::FromPrimitive;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICONDATAW_0,
    NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    LoadImageW, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CW_USEDEFAULT, HCURSOR, HHOOK, HICON, HMENU, IMAGE_ICON,
    KBDLLHOOKSTRUCT, LR_DEFAULTSIZE, LR_LOADFROMFILE, MB_OK, MF_BYPOSITION, MF_CHECKED, MF_STRING, MF_UNCHECKED,
    TPM_BOTTOMALIGN, TPM_LEFTBUTTON, TPM_RIGHTALIGN, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WM_APP,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_ENTERIDLE, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK, WM_MOUSEMOVE, WM_NULL,
    WM_QUIT, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_WTSSESSION_CHANGE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    WTS_CONSOLE_CONNECT, WTS_CONSOLE_DISCONNECT, WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT, WTS_SESSION_CREATE,
    WTS_SESSION_LOCK, WTS_SESSION_LOGOFF, WTS_SESSION_LOGON, WTS_SESSION_REMOTE_CONTROL, WTS_SESSION_TERMINATE,
    WTS_SESSION_UNLOCK,
};

const NOTIFY_FOR_THIS_SESSION: u32 = 0x00000000;

// Notification icon messages
const WM_CLICK_NOTIFY_ICON: u32 = WM_APP + 1;
const MENU_EXIT: usize = 0x00;
const MENU_RELOAD: usize = 0x01;
const MENU_PRINT_KEYS: usize = 0x02;
const MENU_HELP: usize = 0x03;
const MENU_DEBUG: usize = 0x04;
const MENU_ACTIONS: usize = 0x05;
const GRIST_INDEX: WINDOW_LONG_PTR_INDEX = WINDOW_LONG_PTR_INDEX(0);

fn grist_app_from_hwnd(hwnd: &mut HWND) -> &mut GristApp {
    get_window_long_ptr(*hwnd, GRIST_INDEX)
        .map(|ptr| unsafe { &mut *(ptr as *mut GristApp) })
        .unwrap()
}

fn load_icon(name: &str) -> eyre::Result<HICON> {
    unsafe {
        LoadImageW(
            Some(HINSTANCE::default()),
            &HSTRING::from(name),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        )
        .map(|handle| HICON(handle.0))
        .map_err(eyre::Report::from)
    }
}

fn create_notification_icon(hwnd: HWND) -> eyre::Result<NOTIFYICONDATAW> {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 0x0,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_CLICK_NOTIFY_ICON,
        hIcon: load_icon("grist.ico")?,
        szTip: [0; 128],
        dwState: Default::default(),
        dwStateMask: Default::default(),
        szInfo: [0; 256],
        Anonymous: NOTIFYICONDATAW_0 {
            uVersion: NOTIFYICON_VERSION_4,
        },
        szInfoTitle: [0; 64],
        dwInfoFlags: Default::default(),
        guidItem: Default::default(),
        hBalloonIcon: Default::default(),
    };

    let mut tooltip: Vec<u16> = "Grist Window Manager".encode_utf16().collect();
    tooltip.resize(nid.szTip.len(), 0);
    nid.szTip.copy_from_slice(tooltip.as_slice());

    shell_notify_icon(NIM_ADD, &mut nid)?;
    shell_notify_icon(NIM_SETVERSION, &mut nid)?;

    Ok(nid)
}

#[inline]
#[allow(non_snake_case)]
fn LOWORD(dword: u32) -> u16 {
    dword as u16
}

#[inline]
#[allow(non_snake_case)]
fn HIWORD(dword: u32) -> u16 {
    (dword >> 16) as u16
}

#[inline]
#[allow(non_snake_case)]
fn GET_X_LPARAM(dword: u32) -> i32 {
    LOWORD(dword as u32) as i16 as i32
}

#[inline]
#[allow(non_snake_case)]
fn GET_Y_LPARAM(dword: u32) -> i32 {
    HIWORD(dword as u32) as i16 as i32
}

fn on_notification_icon(hwnd: &mut HWND, wparam: WPARAM, lparam: LPARAM) -> eyre::Result<()> {
    match LOWORD(lparam.0 as u32) as u32 {
        WM_LBUTTONDBLCLK => {
            print_pressed_keys();
            grist_app_from_hwnd(hwnd).rehook_keyboard();
            Ok(())
        }
        WM_RBUTTONUP => {
            let x = GET_X_LPARAM(wparam.0 as u32);
            let y = GET_Y_LPARAM(wparam.0 as u32);
            let hmenu = create_popup_menu()?;
            let uflags = MF_BYPOSITION | MF_STRING;
            let debug_checked = if crate::DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            };

            let items = [
                (MENU_PRINT_KEYS, "Pressed Keys", uflags),
                (MENU_RELOAD, "Reload", uflags),
                (MENU_HELP, "Help", uflags),
                (MENU_DEBUG, "Debug", uflags | debug_checked),
                (MENU_ACTIONS, "Actions", uflags | debug_checked),
                (MENU_EXIT, "Exit", uflags),
            ];
            for (uposition, (uidnewitem, lpnewitem, uflags)) in items.iter().enumerate() {
                let _ = insert_menu(hmenu, uposition as u32, *uflags, *uidnewitem, lpnewitem);
            }

            set_foreground_window(*hwnd)?;
            let _ = track_popup_menu(
                hmenu,
                TPM_BOTTOMALIGN | TPM_RIGHTALIGN | TPM_LEFTBUTTON,
                x,
                y,
                Some(0), //nReserved, must be 0
                *hwnd,
            );
            destroy_menu(hmenu)?;
            let _ = post_message(Some(*hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            Ok(())
        }
        _ => Ok(()),
    }
}

fn on_wm_command(wparam: WPARAM, hwnd: &mut HWND) {
    match wparam {
        WPARAM(MENU_EXIT) => {
            if let Ok(ptr) = get_window_long_ptr(*hwnd, GRIST_INDEX) {
                let _grist_app = unsafe { Box::from_raw(ptr as *mut GristApp) };
            }
            let _ = post_message(Some(*hwnd), WM_QUIT, WPARAM(0), LPARAM(0));
        }
        WPARAM(MENU_RELOAD) => {
            grist_app_from_hwnd(hwnd).rehook_keyboard();
        }
        WPARAM(MENU_PRINT_KEYS) => {
            print_pressed_keys();
        }
        WPARAM(MENU_DEBUG) => {
            let debug = !crate::DEBUG.load(core::sync::atomic::Ordering::Relaxed);
            println!("Setting debug to {}", debug);
            crate::DEBUG.store(debug, core::sync::atomic::Ordering::Relaxed);
        }
        WPARAM(MENU_ACTIONS) => {
            let actions = ACTIONS
                .read()
                .unwrap()
                .iter()
                .map(|action| format!("{:?}", action))
                .collect::<Vec<String>>()
                .join("\n");
            message_box(Some(*hwnd), actions.as_str(), "Grist Help", MB_OK);
        }
        WPARAM(MENU_HELP) => {
            let text = r"Actions:
    ClearTop,
    Maximize,
    Minimize,
    MonitorBottom,
    MonitorBottomLeft,
    MonitorBottomRight,
    MonitorLeft,
    MonitorRight,
    MonitorTop,
    MonitorTopLeft,
    MonitorTopRight,
    MoveNextMonitor,
    MovePrevMonitor,
    OnDesktop { x: i32, y: i32, w: i32, h: i32 },
    OnMonitor { x: i32, y: i32, w: i32, h: i32 },
            ";
            message_box(Some(*hwnd), text, "Grist Help", MB_OK);
        }
        _ => (),
    }
}

fn on_wtssession_change(hwnd: &mut HWND, _msg: u32, wparam: WPARAM, _lparam: LPARAM) {
    let print_wts = |wts| println!("          WM_WTSSESSION_CHANGE {}", wts);

    match wparam.0 as u32 {
        WTS_CONSOLE_CONNECT => print_wts("WTS_CONSOLE_CONNECT"),
        WTS_CONSOLE_DISCONNECT => print_wts("WTS_CONSOLE_DISCONNECT"),
        WTS_REMOTE_CONNECT => print_wts("WTS_REMOTE_CONNECT"),
        WTS_REMOTE_DISCONNECT => print_wts("WTS_REMOTE_DISCONNECT"),
        WTS_SESSION_LOGON => {
            grist_app_from_hwnd(hwnd).hook_keyboard();
            // print_wts("WTS_SESSION_LOGON")
        }
        WTS_SESSION_LOGOFF => {
            grist_app_from_hwnd(hwnd).unhook_keyboard();
            // print_wts("WTS_SESSION_LOGOFF")
        }
        WTS_SESSION_LOCK => {
            grist_app_from_hwnd(hwnd).unhook_keyboard();
            // print_wts("WTS_SESSION_LOCK")
        }
        WTS_SESSION_UNLOCK => {
            grist_app_from_hwnd(hwnd).hook_keyboard();
            // print_wts("WTS_SESSION_UNLOCK")
        }
        WTS_SESSION_REMOTE_CONTROL => {
            // print_wts("WTS_SESSION_REMOTE_CONTROL")
            ()
        }
        WTS_SESSION_CREATE => {
            // print_wts("WTS_SESSION_CREATE")
            ()
        }
        WTS_SESSION_TERMINATE => {
            //print_wts("WTS_SESSION_TERMINATE")
            ()
        }
        _ => {
            // print_wts("WTS Unknown wParam")
            ()
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let mut hwnd = hwnd;
    match msg {
        WM_CREATE => {
            if let Err(error) = wts_register_session_notification(hwnd, NOTIFY_FOR_THIS_SESSION) {
                println!("{:?}", error)
            }

            let nid = match create_notification_icon(hwnd) {
                Ok(nid) => nid,
                Err(error) => {
                    println!("{:?}", error);
                    return def_window_proc(hwnd, msg, wparam, lparam);
                }
            };
            let mut grist_app = Box::new(GristApp { nid, hook: HHOOK::default() });
            grist_app.hook_keyboard();
            let _ = set_window_long_ptr(hwnd, GRIST_INDEX, Box::into_raw(grist_app) as isize);
        }
        WM_DESTROY => {
            let _ = wts_unregister_session_notification(hwnd);
            if let Ok(ptr) = get_window_long_ptr(hwnd, GRIST_INDEX) {
                drop(unsafe { Box::from_raw(ptr as *mut GristApp) });
            }
        }
        WM_CLICK_NOTIFY_ICON => on_notification_icon(&mut hwnd, wparam, lparam).unwrap_or(()),
        WM_COMMAND => on_wm_command(wparam, &mut hwnd),
        WM_WTSSESSION_CHANGE => on_wtssession_change(&mut hwnd, msg, wparam, lparam),
        _ => {
            if DEBUG.load(std::sync::atomic::Ordering::Relaxed)
                && msg != WM_ENTERIDLE
                && lparam != LPARAM(WM_MOUSEMOVE as isize)
            {
                println!("{:>30} w: 0x{:X} l: 0x{:X}", msg::msg_to_string(msg), wparam.0, lparam.0);
            }
        }
    };

    def_window_proc(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if n_code < 0 {
        println!("low_level_keyboard_proc(): ncode < 0");
        return call_next_hook(Some(HHOOK::default()), n_code, wparam, lparam);
    }

    let msg = wparam.0 as u32;
    let vk_code = (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode;

    if let Some(vk_code) = hotkey_action::VK::from_u32(vk_code) {
        match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => PRESSED_KEYS.write().unwrap().insert(vk_code),
            WM_KEYUP | WM_SYSKEYUP => PRESSED_KEYS.write().unwrap().remove(&vk_code),
            _ => true,
        }
    } else {
        // How did we get an invalid VK_CODE?
        return call_next_hook(Some(HHOOK::default()), n_code, wparam, lparam);
    };

    // Print the current keys if debug is enabled
    if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
        print_pressed_keys();
    }

    // Trigger the matching actions
    if let Some(action) = ACTIONS
        .read()
        .unwrap()
        .iter()
        .find(|hotkey_action| hotkey_action.trigger == *PRESSED_KEYS.read().unwrap())
    {
        if let Err(error) = action.action.apply() {
            println!("{:?}", error);
        }
        LRESULT(1)
    } else {
        call_next_hook(Some(HHOOK::default()), n_code, wparam, lparam)
    }
}

pub fn create() -> eyre::Result<HWND> {
    let mut name: Vec<u16> = "Grist".encode_utf16().collect();

    let hmodule = get_module_handle(PCWSTR(std::ptr::null_mut()))?;
    let hinstance: HINSTANCE = HINSTANCE(hmodule.0);

    let wnd_class = WNDCLASSW {
        style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance,
        lpszClassName: PCWSTR(name.as_mut_ptr()),
        cbClsExtra: 0,
        cbWndExtra: std::mem::size_of::<*mut GristApp>() as i32,
        hIcon: HICON::default(),
        hCursor: HCURSOR::default(),
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR(std::ptr::null_mut()),
    };

    register_class(&wnd_class)?;

    create_window(
        WINDOW_EX_STYLE::default(),
        PCWSTR(name.as_mut_ptr()),
        PCWSTR(name.as_mut_ptr()),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        Some(HWND::default()),
        Some(HMENU::default()),
        Some(hinstance),
        std::ptr::null_mut(),
    )
}

struct GristApp {
    nid: NOTIFYICONDATAW,
    hook: HHOOK,
}

impl GristApp {
    pub fn unhook_keyboard(&mut self) {
        if self.hook.is_invalid() {
            println!("Keyboard wasn't hooked!");
            return;
        }

        let _ = unhook_windows_hook_ex(self.hook);
        self.hook = HHOOK::default();
        // println!("Unhooked keyboard events");
    }

    pub fn hook_keyboard(&mut self) {
        if !self.hook.is_invalid() {
            println!("Keyboard was already hooked!");
            return;
        }

        let hmodule = match get_module_handle(PCWSTR::null()) {
            Ok(hmodule) => hmodule,
            Err(_) => {
                println!("Failed to get module handle");
                return;
            }
        };
        let hinstance: HINSTANCE = HINSTANCE(hmodule.0);

        if let Ok(hook) = set_windows_hook(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), Some(hinstance), 0) {
            self.hook = hook;
            // println!("Hooked keyboard events");
            return;
        }

        println!("Failed to hook keyboard events");
        self.hook = HHOOK::default();
    }

    pub fn rehook_keyboard(&mut self) {
        self.unhook_keyboard();
        PRESSED_KEYS.write().unwrap().clear();
        self.hook_keyboard();
    }
}

impl Drop for GristApp {
    fn drop(&mut self) {
        let _ = shell_notify_icon(NIM_DELETE, &mut self.nid);
        if !self.nid.hIcon.is_invalid() {
            let _ = destroy_icon(self.nid.hIcon);
            self.nid.hIcon = HICON::default();
        }
        self.unhook_keyboard();
    }
}
