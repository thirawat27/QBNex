//! DAP base protocol: `Content-Length: n\r\n\r\n<json>` messages.

use std::io::{self, BufRead, Write};

use serde_json::Value;

/// Largest message body accepted (DAP requests are small JSON objects).
pub const MAX_MESSAGE: usize = 64 * 1024 * 1024;

/// Reads one message; `Ok(None)` at end of input.
pub fn read_message(r: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.trim().eq_ignore_ascii_case("Content-Length")
        {
            len = v.trim().parse().ok();
        }
    }
    let len = len.unwrap_or(0);
    // R14: a client-supplied length is not trusted with an allocation.
    if len > MAX_MESSAGE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("DAP message of {len} bytes is too large")));
    }
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    serde_json::from_slice(&buf).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Writes one message.
pub fn write_message(w: &mut impl Write, msg: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(msg)?;
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(&body)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut buf = Vec::new();
        write_message(&mut buf, &serde_json::json!({"a": 1})).unwrap();
        write_message(&mut buf, &serde_json::json!({"b": "é"})).unwrap();
        let mut r = io::BufReader::new(&buf[..]);
        assert_eq!(read_message(&mut r).unwrap().unwrap()["a"], 1);
        assert_eq!(read_message(&mut r).unwrap().unwrap()["b"], "é");
        assert!(read_message(&mut r).unwrap().is_none());
    }

    /// R14: an absurd Content-Length is an error, not a huge allocation.
    #[test]
    fn huge_content_length_is_rejected() {
        let mut r = io::BufReader::new(&b"Content-Length: 1000000000000\r\n\r\n{}"[..]);
        let e = read_message(&mut r).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    }
}
