//! Game controllers for `_DEVICES`/`_DEVICEINPUT`/`STRIG` (replaces the
//! vendored libstem gamepad library in `internal/c/parts/input/game_controller`).
//!
//! Backend: gilrs. Buttons and axes use gilrs' standard layout with fixed
//! indices ([`button_index`], [`axis_index`]); legacy reported the raw driver
//! order, which differed per OS and device (documented deviation).
//! Y axes are negated so that down/right stay positive as with the legacy
//! Windows multimedia joystick API.
//!
//! All calls happen on the thread that runs `QBNex_GAMEPAD_POLL` (MAIN_LOOP);
//! the gilrs context is created lazily on that thread.

use std::cell::RefCell;
use std::ffi::{c_char, CString};

use gilrs::{Axis, Button, EventType, Gilrs};

/// Number of buttons / axes reported for every controller.
pub const BUTTONS: i32 = 19;
pub const AXES: i32 = 8;

/// Fixed 0-based button index for a gilrs button.
pub fn button_index(b: Button) -> Option<i32> {
    Some(match b {
        Button::South => 0,
        Button::East => 1,
        Button::North => 2,
        Button::West => 3,
        Button::LeftTrigger => 4,
        Button::RightTrigger => 5,
        Button::LeftTrigger2 => 6,
        Button::RightTrigger2 => 7,
        Button::Select => 8,
        Button::Start => 9,
        Button::Mode => 10,
        Button::LeftThumb => 11,
        Button::RightThumb => 12,
        Button::DPadUp => 13,
        Button::DPadDown => 14,
        Button::DPadLeft => 15,
        Button::DPadRight => 16,
        Button::C => 17,
        Button::Z => 18,
        _ => return None,
    })
}

/// Fixed 0-based axis index and sign for a gilrs axis.
pub fn axis_index(a: Axis) -> Option<(i32, f32)> {
    Some(match a {
        Axis::LeftStickX => (0, 1.0),
        Axis::LeftStickY => (1, -1.0),
        Axis::RightStickX => (2, 1.0),
        Axis::RightStickY => (3, -1.0),
        Axis::LeftZ => (4, 1.0),
        Axis::RightZ => (5, 1.0),
        Axis::DPadX => (6, 1.0),
        Axis::DPadY => (7, -1.0),
        _ => return None,
    })
}

/// C callbacks into libqb.cpp (the old onDeviceAttached/onButtonDown/... glue).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Callbacks {
    pub attached: Option<extern "C" fn(u32, *const c_char, i32, i32, i32, i32)>,
    pub removed: Option<extern "C" fn(u32)>,
    pub button: Option<extern "C" fn(u32, i32, i32)>,
    pub axis: Option<extern "C" fn(u32, i32, f32)>,
}

thread_local! {
    static GILRS: RefCell<Option<Gilrs>> = const { RefCell::new(None) };
    static STARTED: RefCell<bool> = const { RefCell::new(false) };
}

fn attach(cb: &Callbacks, g: &Gilrs, id: gilrs::GamepadId) {
    let pad = g.gamepad(id);
    let name = CString::new(pad.name().replace('\0', "")).unwrap_or_default();
    let (vendor, product) = (
        pad.vendor_id().unwrap_or(0) as i32,
        pad.product_id().unwrap_or(0) as i32,
    );
    if let Some(f) = cb.attached {
        f(
            usize::from(id) as u32,
            name.as_ptr(),
            vendor,
            product,
            AXES,
            BUTTONS,
        );
    }
}

/// Processes pending controller events (`QBNex_GAMEPAD_POLL`).
///
/// # Safety
/// `cb` must point to a valid struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_gamepad_poll(cb: *const Callbacks) {
    if cb.is_null() {
        return;
    }
    // SAFETY: caller passes a valid struct.
    let cb = unsafe { *cb };
    GILRS.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            if STARTED.with(|s| s.replace(true)) {
                return; // initialisation failed before
            }
            match Gilrs::new() {
                Ok(g) => {
                    let ids: Vec<_> = g.gamepads().map(|(id, _)| id).collect();
                    for id in ids {
                        attach(&cb, &g, id);
                    }
                    *slot = Some(g);
                }
                Err(gilrs::Error::NotImplemented(g)) => *slot = Some(g),
                Err(_) => return,
            }
        }
        let Some(g) = slot.as_mut() else { return };
        while let Some(ev) = g.next_event() {
            let id = usize::from(ev.id) as u32;
            match ev.event {
                EventType::Connected => attach(&cb, g, ev.id),
                EventType::Disconnected => {
                    if let Some(f) = cb.removed {
                        f(id);
                    }
                }
                EventType::ButtonPressed(b, _) | EventType::ButtonReleased(b, _) => {
                    if let (Some(i), Some(f)) = (button_index(b), cb.button) {
                        f(
                            id,
                            i,
                            matches!(ev.event, EventType::ButtonPressed(..)) as i32,
                        );
                    }
                }
                EventType::AxisChanged(a, v, _) => {
                    if let (Some((i, sign)), Some(f)) = (axis_index(a), cb.axis) {
                        f(id, i, (v * sign).clamp(-1.0, 1.0));
                    }
                }
                _ => {}
            }
        }
    });
}

/// Drops the gilrs context (`QBNex_GAMEPAD_SHUTDOWN`).
#[unsafe(no_mangle)]
pub extern "C" fn qb_gamepad_shutdown() {
    GILRS.with(|c| c.borrow_mut().take());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_layout_indices() {
        assert_eq!(button_index(Button::South), Some(0));
        assert_eq!(button_index(Button::East), Some(1));
        assert_eq!(button_index(Button::Start), Some(9));
        assert_eq!(button_index(Button::DPadRight), Some(16));
        assert_eq!(button_index(Button::Z), Some(BUTTONS - 1));
        assert_eq!(button_index(Button::Unknown), None);
        assert_eq!(axis_index(Axis::LeftStickX), Some((0, 1.0)));
        assert_eq!(axis_index(Axis::LeftStickY), Some((1, -1.0)));
        assert_eq!(axis_index(Axis::RightStickY), Some((3, -1.0)));
        assert_eq!(axis_index(Axis::DPadY), Some((AXES - 1, -1.0)));
        assert_eq!(axis_index(Axis::Unknown), None);
    }
}
