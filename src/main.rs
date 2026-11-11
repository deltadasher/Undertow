//! undertow - interactive web wallpapers for Wayland.
//!
//! One binary, three roles:
//!   * no arguments: open the picker (starting the background daemon if needed)
//!   * `--background`: start only the daemon (tray icon + wallpapers), for autostart
//!   * anything else: a CLI that edits the library/config and pokes the daemon

mod backend;
mod config;
mod desktop;
mod ipc;
mod library;
mod server;
mod stats;

use serde_json::Value;
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::Ordering;

const USAGE: &str = "\
undertow - interactive web wallpapers for Wayland (niri, Hyprland, Plasma, sway)

  undertow                         open the wallpaper picker
  undertow --background            start in the tray without opening the picker

  undertow list                    show installed wallpapers
  undertow set <id|https://url>    switch wallpaper (starts undertow if needed)
  undertow add <source>            install from GitHub or link a local directory
                                     owner/repo   owner/repo/sub/dir   owner/repo@branch
                                     https://github.com/owner/repo[/tree/ref/sub/dir]
  undertow update [id]             git pull one wallpaper, or all of them
  undertow remove <id>             delete an installed wallpaper
  undertow props                   show the current wallpaper's settings
  undertow prop <name> <value>     change a setting live
  undertow pause | resume | reload | quit
";

fn fail(message: impl AsRef<str>) -> ExitCode {
    eprintln!("undertow: {}", message.as_ref());
    ExitCode::FAILURE
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Runs the Qt application. Returns when the user quits.
fn run_daemon(show_picker: bool) -> ExitCode {
    let listener = match ipc::bind() {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    *backend::LISTENER.lock().unwrap() = Some(listener);
    backend::START_HIDDEN.store(!show_picker, Ordering::Relaxed);
    library::unpack_presets();

    if std::env::var_os("QT_QPA_PLATFORM").is_none() {
        std::env::set_var("QT_QPA_PLATFORM", "wayland");
    }
    let code = backend::qobject::run_app();
    ipc::cleanup();
    if code == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// Starts a detached daemon and returns immediately.
fn spawn_background() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Command::new(exe)
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not start the daemon: {e}"))
}

fn forward(line: &str) -> ExitCode {
    match ipc::send(line) {
        Some(_) => ExitCode::SUCCESS,
        None => fail("not running (start it with: undertow)"),
    }
}

fn cmd_list() -> ExitCode {
    library::unpack_presets();
    let current = config::current();
    for w in library::all() {
        let mark = if w.id == current { "*" } else { " " };
        println!("{mark} {:<28} {:<7} {}", w.id, w.kind, w.title);
    }
    ExitCode::SUCCESS
}

fn cmd_set(target: &str) -> ExitCode {
    library::unpack_presets();
    if target.is_empty() || (!is_url(target) && library::find(target).is_none()) {
        return fail(format!("no wallpaper '{target}' (see: undertow list)"));
    }
    if ipc::send(&format!("set {target}")).is_some() {
        println!("now showing {target}");
        return ExitCode::SUCCESS;
    }
    config::set_current(target);
    match spawn_background() {
        Ok(()) => {
            println!("started undertow with {target}");
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn cmd_props() -> ExitCode {
    let id = config::current();
    let Some(w) = library::find(&id) else { return fail(format!("'{id}' has no settings")) };
    let values = w.prop_values();
    if values.is_empty() {
        println!("{} has no settings", w.id);
    }
    for (name, value) in values {
        let def = &w.props[&name];
        let range = match (def.get("min"), def.get("max")) {
            (Some(min), Some(max)) => format!("  [{min}..{max}]"),
            _ => String::new(),
        };
        let label = def.get("text").or(def.get("type")).and_then(Value::as_str).unwrap_or("");
        println!("{name} = {value}{range}   ({label})");
    }
    ExitCode::SUCCESS
}

fn cmd_prop(name: &str, raw: &str) -> ExitCode {
    let id = config::current();
    let Some(w) = library::find(&id) else { return fail(format!("'{id}' has no settings")) };
    let Some(default) = w.props.get(name).and_then(|d| d.get("value")) else {
        return fail(format!("{} has no setting '{name}' (see: undertow props)", w.id));
    };
    // Keep the type of the default value.
    let value = match default {
        Value::Number(_) => match raw.parse::<f64>() {
            Ok(n) => serde_json::json!(n),
            Err(_) => return fail(format!("{name} expects a number")),
        },
        Value::Bool(_) => Value::Bool(matches!(raw, "1" | "true" | "on" | "yes")),
        _ => Value::String(raw.to_string()),
    };
    config::set_prop(&w.id, name, value.clone());
    ipc::send("refresh-props");
    println!("{name} = {value}");
    ExitCode::SUCCESS
}

fn report(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(message) => {
            println!("{message}");
            ipc::send("refresh");
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or_default().join(" ");

    match cmd {
        "" | "show" => {
            if ipc::send("show").is_some() {
                return ExitCode::SUCCESS; // already running: just raise the picker
            }
            run_daemon(true)
        }
        "--background" | "daemon" | "run" => run_daemon(false),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        "list" => cmd_list(),
        "set" => cmd_set(&rest),
        "add" if !rest.is_empty() => report(library::add(&rest).map(|id| format!("installed {id}\n  undertow set {id}"))),
        "update" => report(library::update(&rest)),
        "remove" if !rest.is_empty() => report(library::remove(&rest)),
        "props" => cmd_props(),
        "prop" if args.len() >= 3 => cmd_prop(&args[1], &args[2..].join(" ")),
        "pause" | "resume" | "reload" | "quit" => forward(cmd),
        _ => {
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
