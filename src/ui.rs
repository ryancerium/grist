use crate::keyboard;
use crate::DEBUG;
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::minwindef::{LOWORD, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::shared::windef::HICON;
use winapi::shared::windef::HWND;
use winapi::shared::windowsx::{GET_X_LPARAM, GET_Y_LPARAM};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::*;
use winapi::um::winuser::*;

pub const WM_CLICK_NOTIFY_ICON: UINT = winapi::um::winuser::WM_APP + 1;
pub const MENU_EXIT: UINT_PTR = 0x00;
pub const MENU_RELOAD: UINT_PTR = 0x01;

fn utf16(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
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

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg != WM_ENTERIDLE && lparam != WM_MOUSEMOVE as isize && *DEBUG.lock().unwrap() == true {
        println!("MSG: 0x{:X} w: 0x{:X} l: 0x{:X}", msg, wparam, lparam);
    }

    match msg {
        WM_CREATE => {
            let nid = create_notification_icon(hwnd);
            let hook = keyboard::set_keyboard_hook();
            let grist_app = Box::new(GristApp { nid, hook });
            SetWindowLongPtrW(hwnd, 0, Box::into_raw(grist_app) as isize);
            ()
        }
        WM_DESTROY => {
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
                let grist_app_ptr = GetWindowLongPtrW(hwnd, 0) as *mut GristApp;
                let grist_app = &mut *grist_app_ptr;
                grist_app.rehook_keyboard();
                ()
            }
            _ => (),
        },
        _ => (),
    };

    DefWindowProcW(hwnd, msg, wparam, lparam)
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
    pub fn rehook_keyboard(&mut self) {
        if self.hook != std::ptr::null_mut() {
            println!("Unhooked keyboard events...");

            keyboard::unset_keyboard_hook(self.hook);
            self.hook = std::ptr::null_mut();
        } else {
            println!("Keyboard wasn't hooked!");
        }

        self.hook = keyboard::set_keyboard_hook();
    }
}

impl Drop for GristApp {
    fn drop(&mut self) {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &mut self.nid);

            if self.hook != std::ptr::null_mut() {
                keyboard::unset_keyboard_hook(self.hook);
                self.hook = std::ptr::null_mut();
            }
        }
    }
}
