//! A tiny loopback-only HTTP server that hands wallpaper files to the web view.
//!
//! Each wallpaper is served from its own origin, `http://<id>.localhost:<port>/`,
//! so pages can use fetch(), ES modules and absolute paths, and their
//! localStorage is kept apart. Only GET/HEAD of files inside the wallpaper
//! directory are answered.

use crate::library;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Binds to a free port on 127.0.0.1, serves forever on a background thread, returns the port.
pub fn start() -> std::io::Result<u16> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let _ = handle(stream);
            });
        }
    });
    Ok(port)
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" | "oga" => "audio/ogg",
        "wav" => "audio/wav",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "txt" | "glsl" | "frag" | "vert" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], head_only: bool) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut host = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("host") {
                host = value.trim().to_ascii_lowercase();
            }
        }
    }

    let mut parts = request_line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let head_only = method == "HEAD";
    if method != "GET" && !head_only {
        return respond(&mut stream, "405 Method Not Allowed", "text/plain", b"method not allowed", false);
    }

    // "<id>.localhost:<port>" -> "<id>"
    let hostname = host.split(':').next().unwrap_or("");
    let Some(wallpaper) = hostname.strip_suffix(".localhost").and_then(library::find) else {
        return respond(&mut stream, "404 Not Found", "text/plain", b"unknown wallpaper", head_only);
    };

    let mut rel = percent_decode(target.split(['?', '#']).next().unwrap_or("/"));
    if rel == "/" {
        rel = format!("/{}", wallpaper.entry);
    } else if rel.ends_with('/') {
        rel.push_str("index.html");
    }

    // Resolve symlinks and "..", then insist the result is still inside the wallpaper.
    let path = match wallpaper.dir.join(rel.trim_start_matches('/')).canonicalize() {
        Ok(p) if p.starts_with(&wallpaper.dir) && p.is_file() => p,
        _ => return respond(&mut stream, "404 Not Found", "text/plain", b"not found", head_only),
    };
    match fs::read(&path) {
        Ok(body) => respond(&mut stream, "200 OK", mime(&path.to_string_lossy()), &body, head_only),
        Err(_) => respond(&mut stream, "500 Internal Server Error", "text/plain", b"read failed", head_only),
    }
}
