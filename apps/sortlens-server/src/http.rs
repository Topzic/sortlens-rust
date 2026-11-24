//! The smallest HTTP/1.1 that serves the API honestly.
//!
//! Hand-written on `std::net` on purpose: the server binds loopback by default, speaks five
//! routes, and takes no dependency a maintainer would have to audit for the privilege. Only
//! `Content-Length` bodies are accepted — a `Transfer-Encoding: chunked` request is refused
//! with a clear error rather than half-parsed — and every response closes the connection, so
//! the state a resumable upload needs lives on disk, not in the socket.

use std::io::{Read, Write};
use std::net::TcpStream;

/// How big a request head may be. Headers only; the body is bounded separately.
const MAX_HEAD_BYTES: usize = 16 * 1024;

/// A parsed request.
#[derive(Debug)]
pub struct Request {
    /// `GET`, `POST`, …
    pub method: String,
    /// The path without the query string.
    pub path: String,
    /// Header name (lowercased) and value, in arrival order.
    pub headers: Vec<(String, String)>,
    /// Exactly `Content-Length` bytes.
    pub body: Vec<u8>,
}

impl Request {
    /// A header value by lowercase name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// The bearer token of an `Authorization: Bearer <token>` header.
    #[must_use]
    pub fn bearer(&self) -> Option<&str> {
        let value = self.header("authorization")?;
        let rest = value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer "))?;
        Some(rest.trim())
    }
}

/// Why a request could not be read.
#[derive(Debug)]
pub enum ReadError {
    /// The socket closed before or during the request.
    Closed,
    /// The bytes were not HTTP this server can act on.
    Bad(&'static str),
    /// The head or the body was past its bound.
    TooLarge,
    /// The socket failed mid-read.
    Io(std::io::Error),
}

/// Read one request. The body limit is the caller's (`--max-upload-mb` feeds it).
pub fn read_request(stream: &mut TcpStream, max_body: u64) -> Result<Request, ReadError> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if head.len() > MAX_HEAD_BYTES {
            return Err(ReadError::TooLarge);
        }
        match stream.read(&mut byte) {
            Ok(0) => return Err(ReadError::Closed),
            Ok(_) => {
                head.push(byte[0]);
                if head.len() >= 4 && &head[head.len() - 4..] == b"\r\n\r\n" {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock || error.kind() == std::io::ErrorKind::TimedOut => {
                return Err(ReadError::Closed);
            }
            Err(error) => return Err(ReadError::Io(error)),
        }
    }

    let head = String::from_utf8_lossy(&head).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or(ReadError::Bad("empty request"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(ReadError::Bad("no method"))?.to_ascii_uppercase();
    let target = parts.next().ok_or(ReadError::Bad("no target"))?;
    let version = parts.next().ok_or(ReadError::Bad("no version"))?;
    if !version.starts_with("HTTP/1.") {
        return Err(ReadError::Bad("only HTTP/1.x is spoken"));
    }
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ReadError::Bad("a header line without a colon"));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }

    let header = |name: &str| headers.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone());
    if header("transfer-encoding").is_some() {
        return Err(ReadError::Bad("chunked bodies are not accepted; send Content-Length"));
    }
    let length = match header("content-length") {
        Some(value) => value.parse::<u64>().map_err(|_| ReadError::Bad("content-length is not a number"))?,
        None => 0,
    };
    if length > max_body {
        return Err(ReadError::TooLarge);
    }

    let mut body = vec![0u8; length as usize];
    stream.read_exact(&mut body).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            ReadError::Closed
        } else {
            ReadError::Io(error)
        }
    })?;

    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

/// One response, as JSON unless a route says otherwise.
#[derive(Debug)]
pub struct Response {
    status: u16,
    body: Vec<u8>,
    content_type: &'static str,
}

impl Response {
    /// `200` with a JSON object body.
    #[must_use]
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            body: body.to_string().into_bytes(),
            content_type: "application/json",
        }
    }

    /// Write status line, headers, body — and close.
    pub fn write_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let reason = match self.status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            413 => "Payload Too Large",
            422 => "Unprocessable Entity",
            501 => "Not Implemented",
            _ => "Error",
        };
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.status,
            reason,
            self.content_type,
            self.body.len()
        );
        stream.write_all(head.as_bytes())?;
        stream.write_all(&self.body)?;
        stream.flush()
    }
}
