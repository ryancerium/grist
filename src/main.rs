// #![windows_subsystem = "windows"]
// Uncomment the above line to make a windowed app instead of a console app

pub mod cardinal;
pub mod hotkey_action;
pub mod keyboard;
pub mod timeout_action;

mod monitor;
mod ui;
mod window_actions;

#[macro_use]
pub mod macros;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate num_derive;
use hotkey_action::{HotkeyAction, VK};
use std::sync::Mutex;
use winapi::um::winuser::{DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_COMMAND};

lazy_static! {
    static ref ACTIONS: Mutex<Vec<HotkeyAction>> = Mutex::default();
}

lazy_static! {
    pub static ref DEBUG: Mutex<bool> = Mutex::new(false);
}

fn create_actions() -> Vec<HotkeyAction> {
    let mut actions = Vec::new();

    monitor::add_actions(&mut actions);
    window_actions::add_actions(&mut actions);

    actions.extend_from_slice(&[HotkeyAction::new(
        || {
            let mut debug = DEBUG.lock().unwrap();
            *debug = !*debug;
            println!("Setting debug to {}", *debug);
        },
        &[VK::LeftWindows, VK::LeftControl, VK::K],
    )]);
    actions
}

fn main() {
    {
        *ACTIONS.lock().unwrap() = create_actions();
    }

    let mut app = ui::create();
    let mut msg = MSG::default();
    println!("Press any hotkey...");

    unsafe {
        while GetMessageW(&mut msg, app.hwnd, 0, 0) > 0 {
            // Manually check for a keyboard hook reload request because... thread safety
            if msg.hwnd == app.hwnd
                && msg.message == WM_COMMAND
                && msg.wParam == ui::MENU_RELOAD
                && msg.lParam == 0
            {
                app.rehook_keyboard();
            }

            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
