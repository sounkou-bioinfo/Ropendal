use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use savvy::savvy;

use crate::r_values::{bool_scalar, str_scalar};

/// Internal HTTP fixture for tests.
/// @export
#[savvy]
pub struct OpendalHttpFixture {
    root: String,
    endpoint: String,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

#[savvy]
impl OpendalHttpFixture {
    /// Start the internal HTTP fixture.
    /// @export
    fn start(root: &str) -> savvy::Result<Self> {
        let root_path = PathBuf::from(root)
            .canonicalize()
            .map_err(|e| savvy::Error::new(&format!("cannot canonicalize HTTP fixture root: {e}")))?;
        if !root_path.is_dir() {
            return Err(savvy::Error::new("HTTP fixture root must be a directory"));
        }

        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| savvy::Error::new(&format!("cannot bind HTTP fixture: {e}")))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| savvy::Error::new(&format!("cannot configure HTTP fixture: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| savvy::Error::new(&format!("cannot read HTTP fixture address: {e}")))?;
        let endpoint = format!("http://{addr}");
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_root = root_path.clone();

        let handle = thread::spawn(move || run_server(listener, thread_root, thread_stop));

        Ok(Self {
            root: root_path.to_string_lossy().to_string(),
            endpoint,
            stop,
            handle: Some(handle),
        })
    }

    /// Server endpoint.
    /// @export
    fn endpoint(&self) -> savvy::Result<savvy::Sexp> {
        str_scalar(&self.endpoint)?.into()
    }

    /// Served root.
    /// @export
    fn root(&self) -> savvy::Result<savvy::Sexp> {
        str_scalar(&self.root)?.into()
    }

    /// Stop the fixture.
    /// @export
    fn stop(&mut self) -> savvy::Result<savvy::Sexp> {
        self.shutdown();
        bool_scalar(true)?.into()
    }
}

impl Drop for OpendalHttpFixture {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl OpendalHttpFixture {
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.endpoint.trim_start_matches("http://"));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_server(listener: TcpListener, root: PathBuf, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = handle_connection(&mut stream, &root);
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(stream: &mut TcpStream, root: &Path) -> std::io::Result<()> {
    let mut buf = [0_u8; 8192];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req.lines();
    let Some(first) = lines.next() else {
        return write_response(stream, 400, "Bad Request", &[], b"bad request", "text/plain");
    };
    let parts = first.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return write_response(stream, 400, "Bad Request", &[], b"bad request", "text/plain");
    }
    let method = parts[0];
    let uri = parts[1];
    if method != "GET" && method != "HEAD" {
        return write_response(stream, 405, "Method Not Allowed", &[], b"method not allowed", "text/plain");
    }

    let range_header = lines.find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case("range") {
            Some(value.trim().to_string())
        } else {
            None
        }
    });

    let path = match request_path(root, uri) {
        Some(path) => path,
        None => return write_response(stream, 403, "Forbidden", &[], b"forbidden", "text/plain"),
    };

    if path.is_dir() {
        let body = directory_index(root, &path);
        if method == "HEAD" {
            return write_response_with_length(
                stream,
                200,
                "OK",
                &[],
                &[],
                body.len(),
                "text/html; charset=utf-8",
            );
        }
        return write_response(stream, 200, "OK", &[], &body, "text/html; charset=utf-8");
    }

    if !path.is_file() {
        return write_response(stream, 404, "Not Found", &[], b"not found", "text/plain");
    }

    let bytes = fs::read(&path)?;
    let len = bytes.len();
    let Some((start, end)) = range_header.as_deref().and_then(|h| parse_range(h, len)) else {
        if method == "HEAD" {
            return write_response_with_length(
                stream,
                200,
                "OK",
                &[accept_ranges_header()],
                &[],
                len,
                "application/octet-stream",
            );
        }
        return write_response(stream, 200, "OK", &[accept_ranges_header()], &bytes, "application/octet-stream");
    };

    if start >= len {
        let header = format!("Content-Range: bytes */{len}\r\n");
        return write_response(stream, 416, "Range Not Satisfied", &[header], b"range not satisfied", "text/plain");
    }

    let end = end.min(len.saturating_sub(1));
    let body = if method == "HEAD" {
        Vec::new()
    } else {
        bytes[start..=end].to_vec()
    };
    let content_range = format!("Content-Range: bytes {start}-{end}/{len}\r\n");
    write_response_with_length(
        stream,
        206,
        "Partial Content",
        &[accept_ranges_header(), content_range],
        &body,
        end - start + 1,
        "application/octet-stream",
    )
}

fn request_path(root: &Path, uri: &str) -> Option<PathBuf> {
    let path_part = uri.split('?').next().unwrap_or(uri);
    let decoded = percent_decode(path_part)?;
    let mut rel = PathBuf::new();
    for part in decoded.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            value => rel.push(value),
        }
    }
    Some(root.join(rel))
}

fn percent_decode(input: &str) -> Option<String> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = hex_val(bytes[i + 1])?;
            let lo = hex_val(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn parse_range(header: &str, len: usize) -> Option<(usize, usize)> {
    let spec = header.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (start, end) = spec.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<usize>().ok()?;
        let suffix = suffix.min(len);
        return Some((len.saturating_sub(suffix), len.saturating_sub(1)));
    }
    let start = start.parse::<usize>().ok()?;
    let end = if end.is_empty() {
        len.saturating_sub(1)
    } else {
        end.parse::<usize>().ok()?
    };
    Some((start, end))
}

fn directory_index(root: &Path, dir: &Path) -> Vec<u8> {
    let mut rows = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let suffix = if path.is_dir() { "/" } else { "" };
            let href = path
                .strip_prefix(root)
                .ok()
                .map(|p| format!("/{}{}", p.to_string_lossy(), suffix))
                .unwrap_or_else(|| format!("/{name}{suffix}"));
            rows.push(format!("<li><a href=\"{href}\">{name}{suffix}</a></li>"));
        }
    }
    rows.sort();
    format!("<html><body><ul>{}</ul></body></html>", rows.join("\n")).into_bytes()
}

fn accept_ranges_header() -> String {
    "Accept-Ranges: bytes\r\n".to_string()
}

fn write_response(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    headers: &[String],
    body: &[u8],
    content_type: &str,
) -> std::io::Result<()> {
    write_response_with_length(stream, code, reason, headers, body, body.len(), content_type)
}

fn write_response_with_length(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    headers: &[String],
    body: &[u8],
    content_length: usize,
    content_type: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Length: {content_length}\r\nContent-Type: {content_type}\r\nConnection: close\r\n",
    )?;
    for header in headers {
        stream.write_all(header.as_bytes())?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)?;
    stream.flush()
}
