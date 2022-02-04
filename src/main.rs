// #![windows_subsystem = "windows"]
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
use once_cell::sync::Lazy;

// Import crate members
use crate::safe_win32::{dispatch_message, get_message, translate_message};
use hotkey_action::{HotkeyAction, VK};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock,
    },
};
use windows::Win32::UI::WindowsAndMessaging::MSG;

static ACTIONS: Lazy<RwLock<Vec<HotkeyAction>>> = Lazy::new(RwLock::default);
static DEBUG: Lazy<AtomicBool> = Lazy::new(AtomicBool::default);
static PRESSED_KEYS: Lazy<RwLock<BTreeSet<hotkey_action::VK>>> = Lazy::new(RwLock::default);

fn print_pressed_keys() {
    let pressed_keys = PRESSED_KEYS.read().unwrap();
    let s = pressed_keys.iter().fold(String::new(), |mut s, i| {
        let _ = std::fmt::write(&mut s, format_args!("{:?} ", *i));
        s
    });
    println!("Pressed keys: [{}]", s);
}

fn create_actions() -> Vec<HotkeyAction> {
    let mut actions = Vec::new();

    monitor::add_actions(&mut actions);
    window_actions::add_actions(&mut actions);

    actions.extend_from_slice(&[
        HotkeyAction::new(
            "Toggle Debug",
            || {
                let debug = !DEBUG.load(Ordering::Relaxed);
                println!("Setting debug to {}", debug);
                DEBUG.store(debug, Ordering::Relaxed);
                Ok(())
            },
            &[VK::LeftWindows, VK::LeftShift, VK::D],
        ),
        HotkeyAction::new(
            "Print Actions",
            || {
                for action in ACTIONS.read().unwrap().iter() {
                    println!("{:?}", action);
                }
                Ok(())
            },
            &[VK::LeftWindows, VK::LeftShift, VK::OEM2], // Win+LeftShift+?
        ),
    ]);
    actions
}

fn main() -> eyre::Result<()> {
    {
        *ACTIONS.write().unwrap() = create_actions();
    }

    let hwnd = match ui::create() {
        Ok(hwnd) => hwnd,
        Err(e) => return Err(e),
    };

    println!("Win + LeftShift + D to toggle debug");
    println!("Win + LeftShift + ? to view actions");

    let mut msg = MSG::default();
    loop {
        match get_message(&mut msg, hwnd, 0, 0).0 {
            -1 => return Err(eyre!("GetMessageW() failed")),
            0 => return Ok(()),
            _ => {
                translate_message(&msg);
                dispatch_message(&msg);
            }
        }
    }
}
