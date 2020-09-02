use crate::timeout_action;
use crate::ACTIONS;
use crate::{hotkey_action, msg, CHECK_BOOL, DEBUG};
use num::FromPrimitive;
use std::ffi::OsStr;
use std::iter::once;
use std::{collections::HashSet, os::windows::ffi::OsStrExt, sync::Mutex};
use timeout_action::TimeoutAction;
use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::minwindef::{BOOL, DWORD, LOWORD, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::shared::windef::HICON;
use winapi::shared::windef::HWND;
use winapi::shared::windowsx::{GET_X_LPARAM, GET_Y_LPARAM};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::*;
use winapi::{ctypes::c_int, um::winuser::*};

#[link(name = "wtsapi32")]
extern "system" {
    pub fn WTSRegisterSessionNotification(hwnd: HWND, dwFlags: DWORD) -> BOOL;
    pub fn WTSUnRegisterSessionNotification(hwnd: HWND) -> BOOL;
}

pub const NOTIFY_FOR_THIS_SESSION: DWORD = 0x00000000;
#[allow(dead_code)]
pub const NOTIFY_FOR_ALL_SESSIONS: DWORD = 0x00000001;

// Notification icon messages
pub const WM_CLICK_NOTIFY_ICON: UINT = winapi::um::winuser::WM_APP + 1;
pub const MENU_EXIT: UINT_PTR = 0x00;
pub const MENU_RELOAD: UINT_PTR = 0x01;

lazy_static! {
    static ref PRESSED_KEYS: Mutex<HashSet<hotkey_action::VK>> = Mutex::new(HashSet::<hotkey_action::VK>::new());
}

fn utf16(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

fn grist_app_from_hwnd<'window>(hwnd: &'window HWND) -> &'window mut GristApp {
    unsafe {
        let grist_app_ptr = GetWindowLongPtrW(*hwnd, 0) as *mut GristApp;
        let grist_app = &mut *grist_app_ptr;
        grist_app
    }
}

fn load_icon() -> HICON {
    unsafe {
        LoadImageW(
            std::ptr::null_mut(),
            utf16("grist.ico").as_ptr(),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        ) as HICON
    }
}

fn create_notification_icon(window: HWND) -> winapi::um::shellapi::NOTIFYICONDATAW {
    let tooltip: Vec<u16> = utf16("Grist Window Manager");

    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = window;
    nid.uID = 0x0;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_CLICK_NOTIFY_ICON;
    nid.hIcon = load_icon();
    let tooltip_len = std::cmp::min(nid.szTip.len(), tooltip.len());
    nid.szTip[..tooltip_len].clone_from_slice(&tooltip[..tooltip_len]);

    unsafe {
        *nid.u.uVersion_mut() = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_ADD, &mut nid);
        Shell_NotifyIconW(NIM_SETVERSION, &mut nid);
    };

    nid
}

fn on_notification_icon(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> () {
    match LOWORD(lparam as u32) as u32 {
        WM_RBUTTONUP => unsafe {
            let x = GET_X_LPARAM(wparam as isize);
            let y = GET_Y_LPARAM(wparam as isize);
            let hmenu = CreatePopupMenu();
            InsertMenuW(
                hmenu,
                0,
                MF_BYPOSITION | MF_STRING,
                MENU_RELOAD as usize,
                utf16("Reload").as_ptr(),
            );
            InsertMenuW(
                hmenu,
                1,
                MF_BYPOSITION | MF_STRING,
                MENU_EXIT as usize,
                utf16("Exit").as_ptr(),
            );
            SetForegroundWindow(hwnd);
            TrackPopupMenu(
                hmenu,
                TPM_BOTTOMALIGN | TPM_RIGHTALIGN | TPM_LEFTBUTTON,
                x,
                y,
                0, //nReserved, must be 0
                hwnd,
                std::ptr::null(),
            );
            PostMessageW(hwnd, WM_NULL, 0, 0);
        },
        _ => (),
    };
}

fn on_wtssession_change(hwnd: HWND, _msg: UINT, wparam: WPARAM, _lparam: LPARAM) {
    let print_wts = |wts| println!("          WM_WTSSESSION_CHANGE {}", wts);

    match wparam {
        WTS_CONSOLE_CONNECT => print_wts("WTS_CONSOLE_CONNECT"),
        WTS_CONSOLE_DISCONNECT => print_wts("WTS_CONSOLE_DISCONNECT"),
        WTS_REMOTE_CONNECT => print_wts("WTS_REMOTE_CONNECT"),
        WTS_REMOTE_DISCONNECT => print_wts("WTS_REMOTE_DISCONNECT"),
        WTS_SESSION_LOGON => {
            grist_app_from_hwnd(&hwnd).hook_keyboard();
            print_wts("WTS_SESSION_LOGON")
        }
        WTS_SESSION_LOGOFF => {
            grist_app_from_hwnd(&hwnd).unhook_keyboard();
            print_wts("WTS_SESSION_LOGOFF")
        }
        WTS_SESSION_LOCK => {
            grist_app_from_hwnd(&hwnd).unhook_keyboard();
            print_wts("WTS_SESSION_LOCK")
        }
        WTS_SESSION_UNLOCK => {
            grist_app_from_hwnd(&hwnd).hook_keyboard();
            print_wts("WTS_SESSION_UNLOCK")
        }
        WTS_SESSION_REMOTE_CONTROL => print_wts("WTS_SESSION_REMOTE_CONTROL"),
        WTS_SESSION_CREATE => print_wts("WTS_SESSION_CREATE"),
        WTS_SESSION_TERMINATE => print_wts("WTS_SESSION_TERMINATE"),
        _ => print_wts("WTS Unknown wParam"),
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            CHECK_BOOL!(WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION));
            let mut grist_app = Box::new(GristApp {
                nid: create_notification_icon(hwnd),
                hook: std::ptr::null_mut(),
            });
            grist_app.hook_keyboard();
            SetWindowLongPtrW(hwnd, 0, Box::into_raw(grist_app) as isize);
            ()
        }
        WM_DESTROY => {
            CHECK_BOOL!(WTSUnRegisterSessionNotification(hwnd));
            let _grist_app = Box::from_raw(GetWindowLongPtrW(hwnd, 0) as *mut GristApp);
        }
        WM_CLICK_NOTIFY_ICON => on_notification_icon(hwnd, wparam, lparam),
        WM_COMMAND => match wparam {
            MENU_EXIT => {
                let _grist_app = Box::from_raw(GetWindowLongPtrW(hwnd, 0) as *mut GristApp);
                PostMessageW(hwnd, WM_QUIT, 0, 0);
                ()
            }
            MENU_RELOAD => {
                grist_app_from_hwnd(&hwnd).rehook_keyboard();
                ()
            }
            _ => (),
        },
        WM_WTSSESSION_CHANGE => on_wtssession_change(hwnd, msg, wparam, lparam),
        _ => {
            if msg != WM_ENTERIDLE && lparam != WM_MOUSEMOVE as isize && *DEBUG.lock().unwrap() == true {
                println!("{:>30} w: 0x{:X} l: 0x{:X}", msg::msg_to_string(msg), wparam, lparam);
            }
        }
    };

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub unsafe extern "system" fn low_level_keyboard_proc(n_code: c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let _timeout_warning = TimeoutAction::new(std::time::Duration::from_millis(300), || {
        println!("Timer elapsed");
    });

    if n_code < 0 {
        println!("low_level_keyboard_proc(): ncode < 0");
        return CallNextHookEx(std::ptr::null_mut(), n_code, wparam, lparam);
    }

    let vk_code = (*(lparam as *const KBDLLHOOKSTRUCT)).vkCode;
    let mut pressed_keys = PRESSED_KEYS.lock().unwrap();

    match hotkey_action::VK::from_u32(vk_code) {
        Some(vk_code) => match wparam as UINT {
            WM_KEYDOWN | WM_SYSKEYDOWN => pressed_keys.insert(vk_code),
            WM_KEYUP | WM_SYSKEYUP => pressed_keys.remove(&vk_code),
            _ => true,
        },
        _ => true,
    };

    // {
    //     let debug = *DEBUG.lock().unwrap();
    //     let msg = wparam as UINT;
    //     if debug && msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
    //         let s = pressed_keys.iter().fold(String::new(), |mut s, i| {
    //             match std::fmt::write(&mut s, format_args!("{:?} ", *i)) {
    //                 Ok(()) => s,
    //                 Err(_) => s,
    //             }
    //         });
    //         println!("{}", s);
    //     }
    // }

    match ACTIONS
        .lock()
        .unwrap()
        .iter()
        .find(|hotkey_action| hotkey_action.trigger == *pressed_keys)
    {
        Some(action) => {
            (action.action)();
            1
        }
        None => CallNextHookEx(std::ptr::null_mut(), n_code, wparam, lparam),
    }
}

pub fn create() -> HWND {
    let name = utf16("Grist");
    let title = utf16("Grist");

    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let wnd_class = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: name.as_ptr(),
            cbClsExtra: 0,
            cbWndExtra: std::mem::size_of::<*mut GristApp>() as i32,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null_mut(),
        };

        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            0,
            name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        hwnd
    }
}

struct GristApp {
    nid: NOTIFYICONDATAW,
    hook: HHOOK,
}

impl GristApp {
    pub fn unhook_keyboard(&mut self) {
        if self.hook != std::ptr::null_mut() {
            unsafe {
                UnhookWindowsHookEx(self.hook);
            }
            self.hook = std::ptr::null_mut();
            println!("Unhooked keyboard events");
        } else {
            println!("Keyboard wasn't hooked!");
        }
    }

    pub fn hook_keyboard(&mut self) {
        if self.hook == std::ptr::null_mut() {
            self.hook = unsafe {
                SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    GetModuleHandleW(std::ptr::null()),
                    0,
                )
            };
            println!("Hooked keyboard events");
        } else {
            println!("Keyboard was already hooked!");
        }
    }

    pub fn rehook_keyboard(&mut self) {
        self.unhook_keyboard();
        self.hook_keyboard();
    }
}

impl Drop for GristApp {
    fn drop(&mut self) {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
            self.unhook_keyboard();
        }
    }
}
