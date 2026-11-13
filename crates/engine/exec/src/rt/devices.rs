//! Input devices of `_DEVICES`, `_DEVICE$`, `_DEVICEINPUT`, `_BUTTON`,
//! `_BUTTONCHANGE`, `_AXIS`, `_WHEEL`, `_LASTBUTTON`, `_LASTAXIS`,
//! `_LASTWHEEL`, `STICK` and `STRIG`.
//!
//! Device 1 is the keyboard (one button per scan code), device 2 the mouse
//! (3 buttons, 2 axes, 3 wheels), the game controllers follow in the order
//! they were attached. Every change of a device is one event holding the
//! whole state of the device; `_DEVICEINPUT` steps through the events and the
//! other functions read the state of the event it stopped at (libqb's
//! `device_struct`, with a queue per device in place of its byte buffer).

// browser build: no game controllers are attached
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use super::{e, R};
use std::collections::VecDeque;

/// `_DEVICEINPUT` numbers of the two devices every program has.
pub(crate) const KEYBOARD: usize = 1;
pub(crate) const MOUSE: usize = 2;

const KEYBOARD_BUTTONS: usize = 512;
/// Mouse wheels: relative x, relative y, the scroll wheel.
const MOUSE_WHEELS: usize = 3;
pub(crate) const SCROLL_WHEEL: usize = 2;
/// Events a device keeps for `_DEVICEINPUT`; the oldest one is dropped beyond it.
const QUEUE_LIMIT: usize = 1024;
/// `STRIG` remembers a press for the first 256 buttons of a controller.
const STRIG_BUTTONS: usize = 256;
/// `STICK` value of a centred axis, of a missing joystick, and its range.
const STICK_CENTRE: i32 = 127;
const STICK_NONE: i32 = 128;
const STICK_MAX: i32 = 254;

#[derive(Clone, Default)]
struct State {
    axes: Vec<f32>,
    wheels: Vec<f32>,
    buttons: Vec<u8>,
    /// position among the events of all devices (`_DEVICEINPUT` without a number reads the oldest)
    order: u64,
}

struct Device {
    controller: bool,
    name: String,
    /// identity of a controller, to give a re-attached one its old number
    id: u32,
    vendor: i32,
    product: i32,
    connected: bool,
    /// `[0]` the state before the current one, `[1]` the current one, `[2..]` not read yet
    events: VecDeque<State>,
    strig_pressed: Vec<bool>,
}

impl Device {
    fn new(controller: bool, name: String, buttons: usize, axes: usize, wheels: usize) -> Device {
        let s = State { axes: vec![0.0; axes], wheels: vec![0.0; wheels], buttons: vec![0; buttons], order: 0 };
        Device {
            controller,
            name,
            id: 0,
            vendor: 0,
            product: 0,
            connected: true,
            events: VecDeque::from([s.clone(), s]),
            strig_pressed: vec![false; if controller { STRIG_BUTTONS } else { 0 }],
        }
    }
    fn latest(&self) -> &State {
        &self.events[self.events.len() - 1]
    }
    fn current(&self) -> &State {
        &self.events[1]
    }
}

pub(crate) struct Devices {
    /// index 0 = device 1
    list: Vec<Device>,
    /// the device `_DEVICEINPUT` selected: 0 none yet, -1 none had an event
    selected: i32,
    order: u64,
    /// a device function was used: game controllers are polled from now on
    pub(crate) controllers: bool,
}

impl Default for Devices {
    fn default() -> Self {
        Devices {
            list: vec![
                Device::new(false, "[KEYBOARD][BUTTON]".into(), KEYBOARD_BUTTONS, 0, 0),
                Device::new(false, "[MOUSE][BUTTON][AXIS][WHEEL]".into(), 3, 2, MOUSE_WHEELS),
            ],
            selected: 0,
            order: 0,
            controllers: false,
        }
    }
}

impl Devices {
    /// Queues an event for device `n` (1-based): the latest state changed by `change`.
    fn event(&mut self, n: usize, change: impl FnOnce(&mut State)) {
        let Some(d) = self.list.get_mut(n - 1) else { return };
        if d.events.len() >= QUEUE_LIMIT {
            d.events.pop_front();
        }
        let mut s = d.latest().clone();
        s.order = self.order;
        self.order += 1;
        change(&mut s);
        d.events.push_back(s);
    }

    pub(crate) fn button(&mut self, n: usize, button: usize, down: bool) {
        // (a held key repeats: only a change is an event)
        let same = self.list.get(n - 1).is_none_or(|d| d.latest().buttons.get(button).is_none_or(|&b| (b != 0) == down));
        if !same {
            self.event(n, |s| s.buttons[button] = u8::from(down));
        }
    }

    pub(crate) fn axes(&mut self, n: usize, values: &[(usize, f32)]) {
        self.event(n, |s| {
            for &(k, v) in values {
                if let Some(a) = s.axes.get_mut(k) {
                    *a = v.clamp(-1.0, 1.0);
                }
            }
        });
    }

    /// A wheel moved: one event with the movement, one that returns it to rest.
    pub(crate) fn wheel(&mut self, n: usize, values: &[(usize, f32)]) {
        for rest in [false, true] {
            self.event(n, |s| {
                for &(k, v) in values {
                    if let Some(w) = s.wheels.get_mut(k) {
                        *w = if rest { 0.0 } else { v };
                    }
                }
            });
        }
    }

    /// A game controller was attached: back at its old number when it was known, else a new device.
    pub(crate) fn attach(&mut self, id: u32, description: &str, vendor: i32, product: i32, axes: usize, buttons: usize) {
        let known = self.list.iter_mut().find(|d| {
            d.controller && !d.connected && d.vendor == vendor && d.product == product && d.latest().axes.len() == axes && d.latest().buttons.len() == buttons
        });
        if let Some(d) = known {
            d.id = id;
            d.connected = true;
            if let Some(n) = d.name.strip_suffix("[DISCONNECTED]") {
                d.name = n.to_string();
            }
            return;
        }
        let mut name = format!("[CONTROLLER][[NAME][{description}]]");
        if buttons > 0 {
            name.push_str("[BUTTON]");
        }
        if axes > 0 {
            name.push_str("[AXIS]");
        }
        let mut d = Device::new(true, name, buttons, axes, 0);
        (d.id, d.vendor, d.product) = (id, vendor, product);
        self.list.push(d);
    }

    pub(crate) fn detach(&mut self, id: u32) {
        for d in self.list.iter_mut().filter(|d| d.controller && d.connected && d.id == id) {
            d.connected = false;
            d.name.push_str("[DISCONNECTED]");
        }
    }

    /// Device number (1-based) of the connected controller `id`.
    fn controller(&self, id: u32) -> Option<usize> {
        self.list.iter().position(|d| d.controller && d.connected && d.id == id).map(|k| k + 1)
    }

    pub(crate) fn pad_button(&mut self, id: u32, button: usize, down: bool) {
        let Some(n) = self.controller(id) else { return };
        self.button(n, button, down);
        if down && let Some(p) = self.list[n - 1].strig_pressed.get_mut(button) {
            *p = true;
        }
    }

    pub(crate) fn pad_axis(&mut self, id: u32, axis: usize, value: f32) {
        if let Some(n) = self.controller(id) {
            self.axes(n, &[(axis, value)]);
        }
    }

    /// `_DEVICES`
    pub(crate) fn count(&self) -> i32 {
        self.list.len() as i32
    }

    fn get(&self, n: i32) -> R<&Device> {
        match usize::try_from(n).ok().filter(|&n| n >= 1).and_then(|n| self.list.get(n - 1)) {
            Some(d) => Ok(d),
            None => e(5),
        }
    }

    /// `_DEVICE$[(n)]`
    pub(crate) fn name(&self, n: i32, passed: bool) -> R<Vec<u8>> {
        Ok(self.get(if passed { n } else { self.selected })?.name.clone().into_bytes())
    }

    /// `_DEVICEINPUT[(n)]`: steps to the next event; the device number that had one
    /// (-1 for a given `n`), 0 when nothing happened.
    pub(crate) fn input(&mut self, n: i32, passed: bool) -> R<i32> {
        self.selected = -1;
        let (n, found) = if passed {
            (n, -1)
        } else {
            let oldest = self.list.iter().enumerate().filter(|(_, d)| d.events.len() > 2).min_by_key(|(_, d)| d.events[2].order);
            match oldest {
                Some((k, _)) => (k as i32 + 1, k as i32 + 1),
                None => return Ok(0),
            }
        };
        self.get(n)?;
        self.selected = n;
        let d = &mut self.list[n as usize - 1];
        if d.events.len() > 2 {
            d.events.pop_front();
            return Ok(found);
        }
        Ok(0)
    }

    fn selected(&self) -> R<&Device> {
        self.get(self.selected)
    }

    /// `_BUTTON[(n)]`: -1 while button `n` of the selected device is down.
    pub(crate) fn button_down(&self, n: i32, passed: bool) -> R<i32> {
        let d = self.selected()?;
        let n = if passed { n } else { 1 };
        match usize::try_from(n).ok().filter(|&n| n >= 1).and_then(|n| d.current().buttons.get(n - 1)) {
            Some(&b) => Ok(-i32::from(b != 0)),
            None => e(5),
        }
    }

    /// `_BUTTONCHANGE[(n)]`: -1 pressed, 1 released, 0 unchanged since the previous event.
    pub(crate) fn button_change(&self, n: i32, passed: bool) -> R<i32> {
        let d = self.selected()?;
        let n = if passed { n } else { 1 };
        let Some(k) = usize::try_from(n).ok().filter(|&n| n >= 1 && n <= d.current().buttons.len()).map(|n| n - 1) else { return e(5) };
        let (now, before) = (d.current().buttons[k], d.events[0].buttons[k]);
        Ok(i32::from(now < before) - i32::from(now > before))
    }

    /// `_AXIS[(n)]` (-1..1) and `_WHEEL[(n)]` of the selected device.
    pub(crate) fn axis(&self, n: i32, passed: bool, wheel: bool) -> R<f32> {
        let d = self.selected()?;
        let n = if passed { n } else { 1 };
        let values = if wheel { &d.current().wheels } else { &d.current().axes };
        match usize::try_from(n).ok().filter(|&n| n >= 1).and_then(|n| values.get(n - 1)) {
            Some(&v) => Ok(v),
            None => e(5),
        }
    }

    /// `_LASTBUTTON`, `_LASTAXIS`, `_LASTWHEEL` (`what` 0, 1, 2) of device `n` or the selected one.
    pub(crate) fn last(&self, n: i32, passed: bool, what: i32) -> R<i32> {
        let s = self.get(if passed { n } else { self.selected })?.latest();
        Ok(match what {
            0 => s.buttons.len(),
            1 => s.axes.len(),
            _ => s.wheels.len(),
        } as i32)
    }

    fn controllers(&self) -> impl Iterator<Item = &Device> {
        self.list.iter().filter(|d| d.controller)
    }

    /// `STICK(n[, axis_pair])`: 1..254, 127 centred; 128 without that joystick or axis.
    /// `n` 0/1 = x/y of joystick A, 2/3 of joystick B, ...
    pub(crate) fn stick(&self, n: i32, pair: i32) -> i32 {
        let axis = (n & 1) as usize + (pair as usize - 1) * 2;
        let Some(d) = self.controllers().nth((n >> 1) as usize) else { return STICK_NONE };
        let Some(&f) = d.latest().axes.get(axis) else { return STICK_NONE };
        let f = if f > -0.01 && f <= 0.01 { 0.0 } else { f };
        (qb_runtime::round::qb_qbr_f64(f64::from(f) * 127.0) as i32 + STICK_CENTRE).clamp(1, STICK_MAX)
    }

    /// `STRIG(n[, controller])`: even `n` = pressed since the last call, odd = down now;
    /// `n \ 4` is the button. Without a controller number, bit 1 of `n` selects joystick B.
    pub(crate) fn strig(&mut self, n: i32, controller: Option<i32>) -> i32 {
        let (n, controller) = match controller {
            Some(c) => (n, c),
            None if n & 2 != 0 => (n - 2, 2),
            None => (n, 1),
        };
        let button = (n >> 2) as usize;
        let since_last_call = n & 1 == 0;
        let Some(d) = self.list.iter_mut().filter(|d| d.controller).nth(controller as usize - 1) else { return 0 };
        if button >= d.latest().buttons.len() {
            return 0;
        }
        if since_last_call {
            return match d.strig_pressed.get_mut(button) {
                Some(p) => -i32::from(std::mem::take(p)),
                None => 0,
            };
        }
        -i32::from(d.latest().buttons[button] != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deviceinput_reads_the_oldest_event_of_all_devices() {
        let mut d = Devices::default();
        assert_eq!(d.count(), 2);
        assert_eq!(d.input(0, false).ok(), Some(0));
        d.button(MOUSE, 0, true);
        d.button(KEYBOARD, 30, true);
        d.button(KEYBOARD, 30, true); // a repeat is no event
        d.axes(MOUSE, &[(0, 0.5), (1, -2.0)]);
        assert_eq!(d.input(0, false).ok(), Some(2));
        assert_eq!(d.name(0, false).ok(), Some(b"[MOUSE][BUTTON][AXIS][WHEEL]".to_vec()));
        assert_eq!(d.button_down(1, true).ok(), Some(-1));
        assert_eq!(d.button_change(0, false).ok(), Some(-1));
        assert_eq!(d.axis(1, true, false).ok(), Some(0.0));
        assert_eq!(d.input(0, false).ok(), Some(1));
        assert_eq!(d.button_down(31, true).ok(), Some(-1));
        assert_eq!(d.input(0, false).ok(), Some(2));
        assert_eq!(d.axis(1, true, false).ok(), Some(0.5));
        assert_eq!(d.axis(2, true, false).ok(), Some(-1.0));
        assert_eq!(d.button_change(1, true).ok(), Some(0));
        assert_eq!(d.input(0, false).ok(), Some(0));
        // a given device: -1 for an event, and it stays selected without one
        d.wheel(MOUSE, &[(SCROLL_WHEEL, -1.0)]);
        assert_eq!(d.input(2, true).ok(), Some(-1));
        assert_eq!(d.axis(3, true, true).ok(), Some(-1.0));
        assert_eq!(d.input(2, true).ok(), Some(-1));
        assert_eq!(d.axis(3, true, true).ok(), Some(0.0));
        assert_eq!(d.input(2, true).ok(), Some(0));
        assert!(d.input(3, true).is_err());
        assert_eq!((d.last(1, true, 0).ok(), d.last(2, true, 1).ok(), d.last(2, true, 2).ok()), (Some(512), Some(2), Some(3)));
        assert!(d.button_down(4, true).is_err());
    }

    #[test]
    fn controllers_feed_stick_and_strig() {
        let mut d = Devices::default();
        assert_eq!(d.stick(0, 1), 128);
        assert_eq!(d.strig(1, None), 0);
        d.attach(7, "Pad", 1, 2, 4, 8);
        assert_eq!(d.count(), 3);
        assert_eq!(d.name(3, true).ok(), Some(b"[CONTROLLER][[NAME][Pad]][BUTTON][AXIS]".to_vec()));
        assert_eq!(d.stick(0, 1), 127);
        d.pad_axis(7, 0, 1.0);
        d.pad_axis(7, 1, -1.0);
        d.pad_axis(7, 3, 0.005);
        assert_eq!((d.stick(0, 1), d.stick(1, 1), d.stick(1, 2), d.stick(0, 3), d.stick(2, 1)), (254, 1, 127, 128, 128));
        d.pad_button(7, 0, true);
        assert_eq!(d.strig(1, None), -1);
        d.pad_button(7, 0, false);
        assert_eq!((d.strig(1, None), d.strig(0, None), d.strig(0, None)), (0, -1, 0));
        d.pad_button(7, 1, true);
        assert_eq!((d.strig(5, None), d.strig(5, Some(1)), d.strig(3, None), d.strig(5, Some(2))), (-1, -1, 0, 0));
        // unplugged and back: the same device number
        d.detach(7);
        assert!(d.name(3, true).is_ok_and(|n| n.ends_with(b"[DISCONNECTED]")));
        d.pad_button(7, 2, true);
        d.attach(9, "Pad", 1, 2, 4, 8);
        assert_eq!(d.count(), 3);
        assert_eq!(d.name(3, true).ok(), Some(b"[CONTROLLER][[NAME][Pad]][BUTTON][AXIS]".to_vec()));
        d.pad_button(9, 2, true);
        assert_eq!(d.strig(9, None), -1);
    }
}
