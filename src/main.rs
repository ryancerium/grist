// #![windows_subsystem = "windows"]
// Uncomment the above line to make a windowed app instead of a console app

// Declare the application's modules
mod cardinal;
mod hotkey_action;
mod monitor;
mod msg;
mod timeout_action;
mod ui;
mod window_actions;

// Declare the application's macros
#[macro_use]
mod macros;

// Import external crate macros
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate num_derive;

// Import crate members
use hotkey_action::{HotkeyAction, VK};
use std::sync::Mutex;
use winapi::um::winuser::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};

lazy_static! {
    static ref ACTIONS: Mutex<Vec<HotkeyAction>> = Mutex::default();
}

lazy_static! {
    pub static ref DEBUG: Mutex<bool> = Mutex::new(true);
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

    let hwnd = unsafe { CHECK_HWND!(ui::create()) };

    let mut msg = MSG::default();
    println!("Win + LeftCtrl + K to toggle debug");

    unsafe {
        while GetMessageW(&mut msg, hwnd, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
