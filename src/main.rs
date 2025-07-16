#![windows_subsystem = "windows"]
// Uncomment the above line to make a windowed app instead of a console app

// Declare the application's modules
mod cardinal;
mod hotkey_action;
mod monitor;
mod msg;
mod safe_win32;
mod ui;
mod window_actions;

// Declare the application's macros
#[macro_use]
mod macros;

// Import external crate macros
#[macro_use]
extern crate num_derive;

use eyre::eyre;
use std::sync::LazyLock;

// Import crate members
use crate::safe_win32::{dispatch_message, get_message, message_box, translate_message};
use hotkey_action::HotkeyAction;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::RwLock;
use windows::{
    core::BOOL,
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{MB_OK, MSG},
    },
};

static ACTIONS: RwLock<Vec<HotkeyAction>> = RwLock::new(Vec::new());
static DEBUG: AtomicBool = AtomicBool::new(false);
static PRESSED_KEYS: LazyLock<RwLock<BTreeSet<hotkey_action::VK>>> = LazyLock::new(RwLock::default);
// https://github.com/rust-lang/rust/issues/71835
// static PRESSED_KEYS: RwLock<BTreeSet<hotkey_action::VK>> = RwLock::new(BTreeSet::new());

fn print_pressed_keys() {
    let mut s = PRESSED_KEYS.read().unwrap().iter().fold(String::new(), |mut s, i| {
        let _ = std::fmt::write(&mut s, format_args!("{:?} ", *i));
        s
    });
    if s.is_empty() {
        s = String::from("No keys currently pressed");
    }
    message_box(Some(HWND::default()), s.as_str(), "Pressed Keys", MB_OK);
}

fn create_actions() -> Vec<HotkeyAction> {
    let mut actions = Vec::new();

    monitor::add_actions(&mut actions);
    window_actions::add_actions(&mut actions);
    actions
}

fn main() -> eyre::Result<()> {
    {
        *ACTIONS.write().unwrap() = create_actions();
    }

    let hwnd = ui::create()?;
    let mut msg = MSG::default();
    loop {
        match get_message(&mut msg, Some(hwnd), 0, 0) {
            BOOL(-1) => return Err(eyre!("GetMessageW() failed")),
            BOOL(0) => return Ok(()),
            _ => {
                let _ = translate_message(&msg);
                dispatch_message(&msg);
            }
        }
    }
}
