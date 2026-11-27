//! DAP base protocol: `Content-Length: n\r\n\r\n<json>` messages.

use std::io::{self, BufRead, Write};

use serde_json::Value;

/// Largest message body accepted (DAP requests are small JSON objects).
pub const MAX_MESSAGE: usize = 64 * 1024 * 1024;

/// Reads one message; `Ok(None)` at end of input.
pub fn read_message(r: &mut impl BufRead) -> io::Result<Option<Value>> {
    match read_frame(r)? {
        Some(body) => serde_json::from_slice(&body).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
        None => Ok(None),
    }
}

/// Reads the body of one message without parsing it; `Ok(None)` at end of
/// input. An error here means the framing itself is broken (the next message
/// cannot be found); a body that is not JSON is the caller's to report.
pub fn read_frame(r: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
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
            let v = v.trim();
            len = Some(v.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, format!("bad Content-Length `{v}`")))?);
        }
    }
    let len = len.unwrap_or(0);
    // A client-supplied length is not trusted with an allocation.
    if len > MAX_MESSAGE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("DAP message of {len} bytes is too large")));
    }
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    Ok(Some(buf))
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

    /// An absurd Content-Length is an error, not a huge allocation.
    #[test]
    fn huge_content_length_is_rejected() {
        let mut r = io::BufReader::new(&b"Content-Length: 1000000000000\r\n\r\n{}"[..]);
        let e = read_message(&mut r).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    }

    /// A Content-Length that is not a number is broken framing: an error, as in
    /// the language server. Ignoring it read the body as header lines, and every
    /// later message was lost in them.
    #[test]
    fn bad_content_length_is_an_error() {
        let mut buf = b"Content-Length: abc\r\n\r\n{}".to_vec();
        write_message(&mut buf, &serde_json::json!({"a": 3})).unwrap();
        let mut r = io::BufReader::new(&buf[..]);
        assert_eq!(read_frame(&mut r).unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    /// A body that is not JSON is still one whole frame: the next message is
    /// read normally after it.
    #[test]
    fn bad_body_keeps_the_framing() {
        let mut buf = b"Content-Length: 5\r\n\r\n{nope".to_vec();
        write_message(&mut buf, &serde_json::json!({"a": 2})).unwrap();
        let mut r = io::BufReader::new(&buf[..]);
        let body = read_frame(&mut r).unwrap().unwrap();
        assert!(serde_json::from_slice::<Value>(&body).is_err());
        assert_eq!(read_message(&mut r).unwrap().unwrap()["a"], 2);
        assert!(read_frame(&mut r).unwrap().is_none());
    }
}
