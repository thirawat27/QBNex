//! Game controllers for `_DEVICES`/`_DEVICEINPUT`/`STRIG`.
//!
//! Backend: gilrs. Buttons and axes use gilrs' standard layout with fixed
//! indices ([`button_index`], [`axis_index`]); legacy reported the raw driver
//! order, which differed per OS and device (documented deviation).
//! Y axes are negated so that down/right stay positive as with the legacy
//! Windows multimedia joystick API.
//!
//! All calls happen on the thread that calls [`qb_gamepad_poll`]; the gilrs
//! context is created lazily on that thread.

use std::cell::RefCell;

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

/// A controller was attached: (id, name, vendor, product, axes, buttons).
pub type AttachedFn = fn(u32, &str, i32, i32, i32, i32);

/// Receivers of controller events: attached, removed (id), button (id,
/// index, down), axis (id, index, value).
#[derive(Clone, Copy)]
pub struct Callbacks {
    pub attached: Option<AttachedFn>,
    pub removed: Option<fn(u32)>,
    pub button: Option<fn(u32, i32, bool)>,
    pub axis: Option<fn(u32, i32, f32)>,
}

thread_local! {
    static GILRS: RefCell<Option<Gilrs>> = const { RefCell::new(None) };
    static STARTED: RefCell<bool> = const { RefCell::new(false) };
}

fn attach(cb: &Callbacks, g: &Gilrs, id: gilrs::GamepadId) {
    let pad = g.gamepad(id);
    let (vendor, product) = (
        pad.vendor_id().unwrap_or(0) as i32,
        pad.product_id().unwrap_or(0) as i32,
    );
    if let Some(f) = cb.attached {
        f(
            usize::from(id) as u32,
            pad.name(),
            vendor,
            product,
            AXES,
            BUTTONS,
        );
    }
}

/// Button events a test script asked for (`pad` lines), waiting for the next poll:
/// (pad, button, down).
static SCRIPTED: std::sync::Mutex<Vec<(u32, i32, bool)>> = std::sync::Mutex::new(Vec::new());
/// The scripted pads already reported as attached.
static SCRIPTED_PADS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());
/// Ids of scripted pads start here, clear of the ids of real controllers.
const SCRIPTED_ID: u32 = 1 << 20;

/// A script presses or releases `button` (from 0) of its pad number `pad`. The pad
/// is attached the first time it is named, before any real controller is looked
/// for, so scripted pads are the first joysticks of the program.
pub fn script_button(pad: u32, button: i32, down: bool) {
    SCRIPTED.lock().unwrap_or_else(|e| e.into_inner()).push((pad, button, down));
}

fn scripted(cb: &Callbacks) {
    let events = std::mem::take(&mut *SCRIPTED.lock().unwrap_or_else(|e| e.into_inner()));
    let mut pads = SCRIPTED_PADS.lock().unwrap_or_else(|e| e.into_inner());
    for (pad, button, down) in events {
        if !pads.contains(&pad) {
            pads.push(pad);
            if let Some(f) = cb.attached {
                f(SCRIPTED_ID + pad, "Scripted pad", 0, 0, AXES, BUTTONS);
            }
        }
        if let Some(f) = cb.button {
            f(SCRIPTED_ID + pad, button, down);
        }
    }
}

/// Processes pending controller events.
pub fn qb_gamepad_poll(cb: &Callbacks) {
    scripted(cb);
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
                        attach(cb, &g, id);
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
                EventType::Connected => attach(cb, g, ev.id),
                EventType::Disconnected => {
                    if let Some(f) = cb.removed {
                        f(id);
                    }
                }
                EventType::ButtonPressed(b, _) | EventType::ButtonReleased(b, _) => {
                    if let (Some(i), Some(f)) = (button_index(b), cb.button) {
                        f(id, i, matches!(ev.event, EventType::ButtonPressed(..)));
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
