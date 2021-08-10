use crate::{hotkey_action, msg, ACTIONS, CHECK_BOOL, DEBUG};
use bindings::Windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, WPARAM};
use bindings::Windows::Win32::Graphics::Gdi::HBRUSH;
use bindings::Windows::Win32::System::LibraryLoader::GetModuleHandleW;
use bindings::Windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use bindings::Windows::Win32::UI::Controls::{LR_DEFAULTSIZE, LR_LOADFROMFILE};
use bindings::Windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NOTIFYICONDATAW,
    NOTIFYICONDATAW_0, NOTIFYICON_VERSION_4,
};
use bindings::Windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowTextW, InsertMenuW, LoadImageW, PostMessageW, RegisterClassW, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowsHookExW, TrackPopupMenu, UnhookWindowsHookEx, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CW_USEDEFAULT, HCURSOR,
    HHOOK, HICON, HMENU, IMAGE_ICON, KBDLLHOOKSTRUCT, MF_BYPOSITION, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTBUTTON,
    TPM_RIGHTALIGN, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WM_APP, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_ENTERIDLE, WM_KEYDOWN, WM_KEYUP, WM_MOUSEMOVE, WM_NULL, WM_QUIT, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_WTSSESSION_CHANGE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WTS_CONSOLE_CONNECT, WTS_CONSOLE_DISCONNECT,
    WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT, WTS_SESSION_CREATE, WTS_SESSION_LOCK, WTS_SESSION_LOGOFF,
    WTS_SESSION_LOGON, WTS_SESSION_REMOTE_CONTROL, WTS_SESSION_TERMINATE, WTS_SESSION_UNLOCK,
};
use num::FromPrimitive;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::iter::once;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::sync::Mutex;

const NOTIFY_FOR_THIS_SESSION: u32 = 0x00000000;
#[allow(dead_code)]
const NOTIFY_FOR_ALL_SESSIONS: u32 = 0x00000001;

// Notification icon messages
const WM_CLICK_NOTIFY_ICON: u32 = WM_APP + 1;
const MENU_EXIT: usize = 0x00;
const MENU_RELOAD: usize = 0x01;

lazy_static! {
    static ref PRESSED_KEYS: Mutex<BTreeSet<hotkey_action::VK>> = Mutex::new(BTreeSet::<hotkey_action::VK>::new());
}

fn utf16(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

fn grist_app_from_hwnd<'window>(hwnd: &'window HWND) -> &'window mut GristApp {
    unsafe {
        let grist_app_ptr = GetWindowLongPtrW(*hwnd, WINDOW_LONG_PTR_INDEX(0)) as *mut GristApp;
        let grist_app = &mut *grist_app_ptr;
        grist_app
    }
}

fn load_icon() -> HICON {
    unsafe {
        let handle = LoadImageW(
            HINSTANCE::NULL,
            "grist.ico",
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        );
        if handle.is_invalid() {
            println!("Could not load icon");
        }
        HICON(handle.0)
    }
}

fn create_notification_icon(hwnd: HWND) -> NOTIFYICONDATAW {
    let tooltip: Vec<u16> = utf16("Grist Window Manager");

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 0x0,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_CLICK_NOTIFY_ICON,
        hIcon: load_icon(),
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

    let nid_sz_tip = std::ptr::addr_of_mut!(nid.szTip);

    unsafe {
        let len = std::cmp::min((*nid_sz_tip).len(), tooltip.len());
        let nid_sz_tip_ptr = nid_sz_tip as *mut u16;
        nid_sz_tip_ptr.copy_from_nonoverlapping(tooltip.as_ptr(), len);

        CHECK_BOOL!(Shell_NotifyIconW(NIM_ADD, &mut nid));
        CHECK_BOOL!(Shell_NotifyIconW(NIM_SETVERSION, &mut nid));
    };

    nid
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

fn on_notification_icon(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> () {
    match LOWORD(lparam.0 as u32) as u32 {
        WM_RBUTTONUP => unsafe {
            let x = GET_X_LPARAM(wparam.0 as u32);
            let y = GET_Y_LPARAM(wparam.0 as u32);
            let hmenu = CreatePopupMenu();
            InsertMenuW(hmenu, 0, MF_BYPOSITION | MF_STRING, MENU_RELOAD as usize, "Reload");
            InsertMenuW(hmenu, 1, MF_BYPOSITION | MF_STRING, MENU_EXIT as usize, "Exit");
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
            PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        },
        _ => (),
    };
}

fn on_wm_command(wparam: WPARAM, hwnd: HWND) {
    match wparam {
        WPARAM(MENU_EXIT) => unsafe {
            let _grist_app = Box::from_raw(GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0)) as *mut GristApp);
            PostMessageW(hwnd, WM_QUIT, WPARAM(0), LPARAM(0));
            ()
        }
        WPARAM(MENU_RELOAD) => {
            grist_app_from_hwnd(&hwnd).rehook_keyboard();
            ()
        }
        _ => (),
    }
}

fn on_wtssession_change(hwnd: HWND, _msg: u32, wparam: WPARAM, _lparam: LPARAM) {
    let print_wts = |wts| println!("          WM_WTSSESSION_CHANGE {}", wts);

    match wparam.0 as u32 {
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

pub fn get_window_text(hwnd: HWND) -> String {
    let text_length = unsafe { GetWindowTextLengthW(hwnd) + 1 };
    let mut chars = Vec::with_capacity(text_length as usize);
    chars.resize(text_length as usize, 0);
    unsafe { GetWindowTextW(hwnd, PWSTR(chars.as_mut_ptr()), chars.len() as i32) };
    OsString::from_wide(chars.as_slice()).into_string().unwrap()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            CHECK_BOOL!(WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION));
            let mut grist_app = Box::new(GristApp {
                nid: create_notification_icon(hwnd),
                hook: HHOOK::NULL,
            });
            grist_app.hook_keyboard();
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), Box::into_raw(grist_app) as isize);
            ()
        }
        WM_DESTROY => {
            let _success = WTSUnRegisterSessionNotification(hwnd);
            let _grist_app = Box::from_raw(GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0)) as *mut GristApp);
        }
        WM_CLICK_NOTIFY_ICON => on_notification_icon(hwnd, wparam, lparam),
        WM_COMMAND => on_wm_command(wparam, hwnd),
        WM_WTSSESSION_CHANGE => on_wtssession_change(hwnd, msg, wparam, lparam),
        _ => {
            if msg != WM_ENTERIDLE && lparam != LPARAM(WM_MOUSEMOVE as isize) {
                println!(
                    "{:>30} w: 0x{:X} l: 0x{:X}",
                    msg::msg_to_string(msg),
                    wparam.0,
                    lparam.0
                );
            }
        }
    };

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if n_code < 0 {
        println!("low_level_keyboard_proc(): ncode < 0");
        return CallNextHookEx(HHOOK::NULL, n_code, wparam, lparam);
    }

    let vk_code = (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode;
    let mut pressed_keys = PRESSED_KEYS.lock().unwrap();

    match hotkey_action::VK::from_u32(vk_code) {
        Some(vk_code) => match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => pressed_keys.insert(vk_code),
            WM_KEYUP | WM_SYSKEYUP => pressed_keys.remove(&vk_code),
            _ => true,
        },
        _ => true,
    };

    {
        let debug = *DEBUG.lock().unwrap();
        let msg = wparam.0 as u32;
        if debug && msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let s = pressed_keys.iter().fold(String::new(), |mut s, i| {
                match std::fmt::write(&mut s, format_args!("{:?} ", *i)) {
                    _ => s,
                }
            });
            println!("{}", s);
        }
    }

    match ACTIONS
        .read()
        .unwrap()
        .iter()
        .find(|hotkey_action| hotkey_action.trigger == *pressed_keys)
    {
        Some(action) => {
            (action.action)();
            LRESULT(1)
        }
        None => CallNextHookEx(HHOOK::NULL, n_code, wparam, lparam),
    }
}

pub fn create() -> HWND {
    let mut name = utf16("Grist");

    unsafe {
        let hinstance = GetModuleHandleW(PWSTR::NULL);
        let wnd_class = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: PWSTR(name.as_mut_ptr()),
            cbClsExtra: 0,
            cbWndExtra: std::mem::size_of::<*mut GristApp>() as i32,
            hIcon: HICON::NULL,
            hCursor: HCURSOR::NULL,
            hbrBackground: HBRUSH::NULL,
            lpszMenuName: PWSTR::NULL,
        };

        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            "Grist",
            "Grist",
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND::NULL,
            HMENU::NULL,
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
        if !self.hook.is_null() {
            unsafe {
                UnhookWindowsHookEx(self.hook);
            }
            self.hook = HHOOK::NULL;
            println!("Unhooked keyboard events");
        } else {
            println!("Keyboard wasn't hooked!");
        }
    }

    pub fn hook_keyboard(&mut self) {
        if self.hook.is_null() {
            self.hook = unsafe {
                SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    GetModuleHandleW(PWSTR::NULL),
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
