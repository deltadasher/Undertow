//! One-line-per-command Unix socket between the CLI and the running daemon.
//! It doubles as the single-instance lock.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs};

fn socket_path() -> PathBuf {
    match env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("undertow.sock"),
        _ => env::temp_dir().join(format!("undertow-{}.sock", env::var("USER").unwrap_or_default())),
    }
}

/// Sends a command to the running daemon. `None` means no daemon is listening.
pub fn send(line: &str) -> Option<String> {
    let mut stream = UnixStream::connect(socket_path()).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    stream.write_all(format!("{line}\n").as_bytes()).ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    Some(reply.trim().to_string())
}

pub fn is_running() -> bool {
    UnixStream::connect(socket_path()).is_ok()
}

/// Claims the socket. Fails if another daemon already holds it.
pub fn bind() -> Result<UnixListener, String> {
    if is_running() {
        return Err("undertow is already running".into());
    }
    let path = socket_path();
    let _ = fs::remove_file(&path); // stale socket from a crashed run
    UnixListener::bind(&path).map_err(|e| format!("could not bind {}: {e}", path.display()))
}

/// Blocks forever, passing each received command to `on_command`.
pub fn serve(listener: UnixListener, on_command: impl Fn(String)) {
    for stream in listener.incoming().flatten() {
        let mut line = String::new();
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        if BufReader::new(&stream).read_line(&mut line).is_ok() && !line.trim().is_empty() {
            on_command(line.trim().to_string());
            let _ = (&stream).write_all(b"ok\n");
        }
    }
}

pub fn cleanup() {
    let _ = fs::remove_file(socket_path());
}
