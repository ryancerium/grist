//use crate::hotkey_action;
use crate::timeout_action;
use crate::ACTIONS;
//use crate::DEBUG;
use bitarray::BitArray;
//use hotkey_action::VK;
use std::sync::Mutex;
use timeout_action::TimeoutAction;
use typenum::U256;
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::um::winuser::*;

lazy_static! {
    static ref PRESSED_KEYS: Mutex<BitArray<u32, U256>> =
        Mutex::new(BitArray::<u32, U256>::from_elem(false));
}

pub unsafe extern "system" fn low_level_keyboard_proc(
    n_code: c_int,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let _timeout_warning = TimeoutAction::new(std::time::Duration::from_millis(300), || {
        println!("Timer elapsed");
    });

    if n_code < 0 {
        println!("low_level_keyboard_proc(): ncode < 0");
        return CallNextHookEx(std::ptr::null_mut(), n_code, wparam, lparam);
    }

    let key_action = wparam as UINT;
    let kbdllhookstruct = &*(lparam as *const KBDLLHOOKSTRUCT);

    let mut pressed_keys = PRESSED_KEYS.lock().unwrap();

    let key_down = key_action == WM_KEYDOWN || key_action == WM_SYSKEYDOWN;
    pressed_keys.set(kbdllhookstruct.vkCode as usize, key_down);

    // {
    //     let debug = DEBUG.lock().unwrap();
    //     if *debug && key_down {
    //         let s = pressed_keys
    //             .iter()
    //             .enumerate()
    //             .filter(|(_, pressed)| *pressed)
    //             .map(|(i, _)| i)
    //             .fold(String::new(), |mut s, i| {
    //                 let key: VK = num::FromPrimitive::from_usize(i).unwrap();
    //                 match std::fmt::write(&mut s, format_args!("{:?} ", key)) {
    //                     Ok(()) => s,
    //                     Err(_) => s,
    //                 }
    //             });
    //         println!("{}", s);
    //     }
    // }

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
        None => CallNextHookEx(std::ptr::null_mut(), n_code, wparam, lparam),
    }
}
