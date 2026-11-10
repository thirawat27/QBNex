//! HTTP/HTTPS client (ureq + rustls with the Mozilla root store; no
//! OpenSSL). Non-2xx statuses are normal results, not errors.

use std::time::Duration;

/// A finished request.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// "Name: value" lines joined with CR LF, in the order received.
    pub headers: String,
    pub body: Vec<u8>,
}

/// Upper bound for a response body (larger bodies are an error).
pub const MAX_BODY: u64 = 512 * 1024 * 1024;

/// Sends one request. `headers` holds "Name: value" lines (CR/LF or LF
/// separated, blank lines ignored). `timeout_ms` <= 0 means 30 seconds; it
/// bounds the whole request (connect, send, receive). Redirects (up to 10)
/// are followed.
pub fn request(method: &str, url: &str, headers: &str, body: &[u8], timeout_ms: i64) -> Result<Response, String> {
    let timeout = Duration::from_millis(if timeout_ms <= 0 { 30_000 } else { timeout_ms as u64 });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .max_redirects(10)
        .user_agent("QBNex")
        .allow_non_standard_methods(true)
        .build()
        .into();
    let mut b = ureq::http::Request::builder().method(method.to_ascii_uppercase().as_str()).uri(url);
    for line in headers.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(format!("bad header line: {line}"));
        };
        b = b.header(k.trim(), v.trim());
    }
    let mut resp = if body.is_empty() && !matches!(method.to_ascii_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
        let req = b.body(()).map_err(|e| e.to_string())?;
        agent.run(req)
    } else {
        let req = b.body(body.to_vec()).map_err(|e| e.to_string())?;
        agent.run(req)
    }
    .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let mut head = String::new();
    for (k, v) in resp.headers() {
        if !head.is_empty() {
            head.push_str("\r\n");
        }
        head.push_str(k.as_str());
        head.push_str(": ");
        head.push_str(&String::from_utf8_lossy(v.as_bytes()));
    }
    let body = resp.body_mut().with_config().limit(MAX_BODY).read_to_vec().map_err(|e| e.to_string())?;
    Ok(Response { status, headers: head, body })
}

#[doc(hidden)]
pub mod testserver {
    //! A tiny in-process HTTP/1.1 server for tests (no internet needed),
    //! also reachable from BASIC through `qb_ext_test_http_server`.
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    /// Serves `count` requests on 127.0.0.1 in a thread; each response
    /// echoes the method, path, a request header and the body. Returns the
    /// base URL ("http://127.0.0.1:port").
    pub fn start(count: usize) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for _ in 0..count {
                let Ok((s, _)) = l.accept() else { return };
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                let mut parts = line.split_whitespace();
                let method = parts.next().unwrap_or("").to_string();
                let path = parts.next().unwrap_or("").to_string();
                let (mut len, mut xtest) = (0usize, String::new());
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    let h = h.trim_end();
                    if h.is_empty() {
                        break;
                    }
                    let (k, v) = h.split_once(':').unwrap_or((h, ""));
                    if k.eq_ignore_ascii_case("content-length") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if k.eq_ignore_ascii_case("x-test") {
                        xtest = v.trim().to_string();
                    }
                }
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                let mut s = s;
                if path == "/redirect" {
                    let _ = s.write_all(b"HTTP/1.1 302 Found\r\nLocation: /target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                }
                if path == "/slow" {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                }
                let status = if path == "/missing" { "404 Not Found" } else { "200 OK" };
                let mut payload = format!("{method} {path} x-test={xtest} body=").into_bytes();
                payload.extend_from_slice(&body);
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nX-Server: qb-test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(&payload);
            }
        });
        format!("http://127.0.0.1:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_post_status_headers_redirect() {
        let base = testserver::start(5);
        let r = request("GET", &format!("{base}/hello?a=1"), "X-Test: yes\n", b"", 5000).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.body, b"GET /hello?a=1 x-test=yes body=");
        assert!(r.headers.contains("x-server: qb-test"));
        let r = request("post", &format!("{base}/p"), "Content-Type: text/plain\r\n", b"data\x00\xff", 5000).unwrap();
        assert_eq!(r.body, b"POST /p x-test= body=data\x00\xff");
        let r = request("GET", &format!("{base}/missing"), "", b"", 5000).unwrap();
        assert_eq!(r.status, 404);
        let r = request("GET", &format!("{base}/redirect"), "", b"", 5000).unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, &b"GET /target x-test= body="[..]));
        assert!(request("GET", &format!("{base}/x"), "no colon", b"", 5000).unwrap_err().contains("bad header"));
    }

    #[test]
    fn timeout_and_refused() {
        let base = testserver::start(1);
        let e = request("GET", &format!("{base}/slow"), "", b"", 300).unwrap_err();
        assert!(e.to_lowercase().contains("timeout") || e.to_lowercase().contains("timed out"), "{e}");
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(request("GET", &format!("http://127.0.0.1:{port}/"), "", b"", 2000).is_err());
        assert!(request("GET", "not a url", "", b"", 2000).is_err());
    }
}
