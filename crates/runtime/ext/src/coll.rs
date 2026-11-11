//! Lists and dictionaries addressed by handles, holding strings and numbers.
//!
//! Handles are positive integers; 0 is "no collection". A list is 0-based;
//! a dictionary keeps keys in insertion order (removing a key keeps the
//! order of the others). Keys are byte strings compared exactly.

use indexmap::IndexMap;
use std::collections::HashMap;

/// A stored value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(Vec<u8>),
    Num(f64),
}

impl Value {
    /// Type code for the C ABI: 1 string, 2 number.
    pub fn kind(&self) -> i32 {
        match self {
            Value::Str(_) => 1,
            Value::Num(_) => 2,
        }
    }
    /// Number view: strings are read like `VAL` (leading number, else 0).
    pub fn as_num(&self) -> f64 {
        match self {
            Value::Num(n) => *n,
            Value::Str(s) => leading_number(s),
        }
    }
    /// String view: numbers use the shortest text that reads back the same.
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Value::Str(s) => s.clone(),
            Value::Num(n) => number_text(*n).into_bytes(),
        }
    }
}

fn leading_number(s: &[u8]) -> f64 {
    let t = String::from_utf8_lossy(s);
    let t = t.trim_start();
    let mut end = 0;
    let b = t.as_bytes();
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_e = false;
    while end < b.len() {
        let c = b[end];
        let ok = match c {
            b'0'..=b'9' => {
                seen_digit = true;
                true
            }
            b'+' | b'-' => end == 0 || matches!(b[end - 1], b'e' | b'E'),
            b'.' if !seen_dot && !seen_e => {
                seen_dot = true;
                true
            }
            b'e' | b'E' if seen_digit && !seen_e => {
                seen_e = true;
                true
            }
            _ => false,
        };
        if !ok {
            break;
        }
        end += 1;
    }
    let mut num = &t[..end];
    while !num.is_empty() && num.parse::<f64>().is_err() {
        num = &num[..num.len() - 1];
    }
    num.parse().unwrap_or(0.0)
}

/// Text of a number: integers without a fraction, otherwise Rust's shortest
/// round-trip form.
pub fn number_text(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{n}") }
}

/// All collections of one thread.
#[derive(Default)]
pub struct Store {
    lists: HashMap<i32, Vec<Value>>,
    dicts: HashMap<i32, IndexMap<Vec<u8>, Value>>,
    next: i32,
}

impl Store {
    fn handle(&mut self) -> i32 {
        self.next += 1;
        self.next
    }

    pub fn list_new(&mut self) -> i32 {
        let h = self.handle();
        self.lists.insert(h, Vec::new());
        h
    }
    pub fn dict_new(&mut self) -> i32 {
        let h = self.handle();
        self.dicts.insert(h, IndexMap::new());
        h
    }
    /// Frees a list or dictionary; false if the handle is unknown.
    pub fn free(&mut self, h: i32) -> bool {
        self.lists.remove(&h).is_some() || self.dicts.remove(&h).is_some()
    }
    pub fn list(&mut self, h: i32) -> Option<&mut Vec<Value>> {
        self.lists.get_mut(&h)
    }
    pub fn dict(&mut self, h: i32) -> Option<&mut IndexMap<Vec<u8>, Value>> {
        self.dicts.get_mut(&h)
    }
}

/// Index check for 0-based list positions.
pub fn index(len: usize, i: i64) -> Option<usize> {
    usize::try_from(i).ok().filter(|&i| i < len)
}

/// Sorts a list: numbers before strings, numbers by value, strings by bytes
/// (or case-insensitively by their ASCII upper case).
pub fn sort(list: &mut [Value], descending: bool, ignore_case: bool) {
    list.sort_by(|a, b| {
        let o = match (a, b) {
            (Value::Num(x), Value::Num(y)) => x.total_cmp(y),
            (Value::Num(_), Value::Str(_)) => std::cmp::Ordering::Less,
            (Value::Str(_), Value::Num(_)) => std::cmp::Ordering::Greater,
            (Value::Str(x), Value::Str(y)) => {
                if ignore_case {
                    x.to_ascii_uppercase().cmp(&y.to_ascii_uppercase())
                } else {
                    x.cmp(y)
                }
            }
        };
        if descending { o.reverse() } else { o }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_dicts() {
        let mut s = Store::default();
        let l = s.list_new();
        let d = s.dict_new();
        assert_ne!(l, d);
        let v = s.list(l).unwrap();
        v.push(Value::Str(b"b".to_vec()));
        v.push(Value::Num(2.5));
        v.push(Value::Str(b"A".to_vec()));
        v.push(Value::Num(-1.0));
        sort(v, false, true);
        assert_eq!(v, &vec![Value::Num(-1.0), Value::Num(2.5), Value::Str(b"A".to_vec()), Value::Str(b"b".to_vec())]);
        assert_eq!(index(4, 3), Some(3));
        assert_eq!(index(4, 4), None);
        assert_eq!(index(4, -1), None);
        let m = s.dict(d).unwrap();
        m.insert(b"x".to_vec(), Value::Num(1.0));
        m.insert(b"y".to_vec(), Value::Str(b"2".to_vec()));
        m.insert(b"x".to_vec(), Value::Num(3.0));
        assert_eq!(m.get_index(0).map(|(k, _)| k.as_slice()), Some(&b"x"[..]));
        m.shift_remove(&b"x".to_vec());
        assert_eq!(m.len(), 1);
        assert!(s.free(l));
        assert!(!s.free(l));
        assert!(s.list(l).is_none());
    }

    #[test]
    fn value_views() {
        assert_eq!(Value::Str(b"  42abc".to_vec()).as_num(), 42.0);
        assert_eq!(Value::Str(b"-1.5e2x".to_vec()).as_num(), -150.0);
        assert_eq!(Value::Str(b"1e".to_vec()).as_num(), 1.0);
        assert_eq!(Value::Str(b"abc".to_vec()).as_num(), 0.0);
        assert_eq!(Value::Num(3.0).as_bytes(), b"3");
        assert_eq!(Value::Num(0.1).as_bytes(), b"0.1");
        assert_eq!(Value::Num(-2.5).as_bytes(), b"-2.5");
        assert_eq!(Value::Num(1e20).as_bytes(), b"100000000000000000000");
    }
}
