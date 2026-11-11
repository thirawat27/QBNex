//! Scripted input: `QBNEX_INPUT_SCRIPT=<file>` feeds keyboard/mouse/window
//! events to a running program through the same path as real OS events.
//! Used by conformance cases and for headless CI (`QBNEX_HEADLESS=1`).
//!
//! One command per line, `#` starts a comment:
//!
//! ```text
//! wait 100            # milliseconds
//! key a               # press and release (also: enter esc tab backspace delete
//! keydown lshift      #   insert home end pgup pgdn left right up down space
//! keyup lshift        #   pause f1..f12 lshift rshift lctrl rctrl lalt ralt)
//! type Hello world    # key press per character
//! mousemove 10 20     # window pixels
//! mousedown 1         # 1 left, 2 right, 3 middle
//! mouseup 1
//! click 1
//! wheel -1            # -1 = up (_MOUSEWHEEL -1), 1 = down
//! resize 800 600
//! close               # window close button
//! snapshot            # print "[frame WxH fnv1a=xxxxxxxx]" for the last presented frame
//! ```

use crate::keymap::Key;

/// One script step.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Wait(u64),
    KeyDown(Key),
    KeyUp(Key),
    MouseMove(i32, i32),
    MouseDown(i32),
    MouseUp(i32),
    Wheel(i32),
    Resize(i32, i32),
    Close,
    Snapshot,
}

/// Parses a key name.
pub fn key_name(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(Key::Char(c));
    }
    let lower = name.to_ascii_lowercase();
    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
        return (1..=12).contains(&n).then_some(Key::F(n));
    }
    Some(match lower.as_str() {
        "enter" | "return" => Key::Enter,
        "esc" | "escape" => Key::Escape,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "insert" | "ins" => Key::Insert,
        "home" => Key::Home,
        "end" => Key::End,
        "pgup" | "pageup" => Key::PageUp,
        "pgdn" | "pagedown" => Key::PageDown,
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "space" => Key::Char(' '),
        "pause" => Key::Pause,
        "lshift" => Key::LShift,
        "rshift" => Key::RShift,
        "lctrl" => Key::LCtrl,
        "rctrl" => Key::RCtrl,
        "lalt" => Key::LAlt,
        "ralt" => Key::RAlt,
        "capslock" => Key::Other(0x14),
        _ => return None,
    })
}

/// Parses a whole script. Errors name the 1-based line.
pub fn parse(text: &str) -> Result<Vec<Cmd>, String> {
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let err = |m: &str| format!("input script line {}: {m}: {raw}", n + 1);
        let line = raw.trim_start();
        let line = line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (cmd, rest) = match line.split_once(' ') {
            Some((c, r)) => (c, r),
            None => (line.trim_end(), ""),
        };
        let nums = |count: usize| -> Result<Vec<i64>, String> {
            let v: Vec<i64> = rest
                .split_whitespace()
                .map(|t| t.parse::<i64>().map_err(|_| err("bad number")))
                .collect::<Result<_, _>>()?;
            if v.len() != count {
                return Err(err("wrong number of arguments"));
            }
            Ok(v)
        };
        let key = || key_name(rest.trim()).ok_or_else(|| err("unknown key"));
        match cmd.to_ascii_lowercase().as_str() {
            "wait" => out.push(Cmd::Wait(nums(1)?[0].max(0) as u64)),
            "key" => {
                let k = key()?;
                out.push(Cmd::KeyDown(k));
                out.push(Cmd::KeyUp(k));
            }
            "keydown" => out.push(Cmd::KeyDown(key()?)),
            "keyup" => out.push(Cmd::KeyUp(key()?)),
            "type" => {
                for c in rest.chars() {
                    out.push(Cmd::KeyDown(Key::Char(c)));
                    out.push(Cmd::KeyUp(Key::Char(c)));
                }
            }
            "mousemove" => {
                let v = nums(2)?;
                out.push(Cmd::MouseMove(v[0] as i32, v[1] as i32));
            }
            "mousedown" => out.push(Cmd::MouseDown(nums(1)?[0] as i32)),
            "mouseup" => out.push(Cmd::MouseUp(nums(1)?[0] as i32)),
            "click" => {
                let b = nums(1)?[0] as i32;
                out.push(Cmd::MouseDown(b));
                out.push(Cmd::MouseUp(b));
            }
            "wheel" => out.push(Cmd::Wheel(nums(1)?[0] as i32)),
            "resize" => {
                let v = nums(2)?;
                out.push(Cmd::Resize(v[0] as i32, v[1] as i32));
            }
            "close" => out.push(Cmd::Close),
            "snapshot" => out.push(Cmd::Snapshot),
            _ => return Err(err("unknown command")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names() {
        assert_eq!(key_name("a"), Some(Key::Char('a')));
        assert_eq!(key_name("A"), Some(Key::Char('A')));
        assert_eq!(key_name("enter"), Some(Key::Enter));
        assert_eq!(key_name("ESC"), Some(Key::Escape));
        assert_eq!(key_name("f12"), Some(Key::F(12)));
        assert_eq!(key_name("lshift"), Some(Key::LShift));
        assert_eq!(key_name("space"), Some(Key::Char(' ')));
        assert_eq!(key_name("pgdn"), Some(Key::PageDown));
        assert_eq!(key_name("f13"), None);
        assert_eq!(key_name("nonsense"), None);
    }

    #[test]
    fn parses_commands() {
        let s = "# demo\nwait 10\nkey a\ntype Hi\nkeydown lshift\nkeyup lshift\n\
                 mousemove 3 4\nclick 2\nwheel -1\nresize 800 600\nsnapshot\nclose\n";
        let c = parse(s).unwrap();
        assert_eq!(
            c,
            vec![
                Cmd::Wait(10),
                Cmd::KeyDown(Key::Char('a')),
                Cmd::KeyUp(Key::Char('a')),
                Cmd::KeyDown(Key::Char('H')),
                Cmd::KeyUp(Key::Char('H')),
                Cmd::KeyDown(Key::Char('i')),
                Cmd::KeyUp(Key::Char('i')),
                Cmd::KeyDown(Key::LShift),
                Cmd::KeyUp(Key::LShift),
                Cmd::MouseMove(3, 4),
                Cmd::MouseDown(2),
                Cmd::MouseUp(2),
                Cmd::Wheel(-1),
                Cmd::Resize(800, 600),
                Cmd::Snapshot,
                Cmd::Close,
            ]
        );
    }

    #[test]
    fn type_keeps_spaces_and_reports_errors() {
        let c = parse("type a b").unwrap();
        assert_eq!(c.len(), 6);
        assert_eq!(c[2], Cmd::KeyDown(Key::Char(' ')));
        assert!(parse("bogus").unwrap_err().contains("line 1"));
        assert!(parse("wait\n").unwrap_err().contains("line 1"));
        assert!(parse("\nkey nope").unwrap_err().contains("line 2"));
    }
}
