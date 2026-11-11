//! Paths and the small JSON config file (`~/.config/undertow/config.json`).
//!
//! Shape: `{ "current": "<id>", "props": { "<id>": { "<name>": <value> } } }`

use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use std::{env, fs};

static LOCK: Mutex<()> = Mutex::new(());

fn home() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match env::var_os(var) {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => home().join(fallback),
    }
}

pub fn config_home() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
}
pub fn data_home() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share")
}
/// Wallpapers installed by the user (git clones and symlinks).
pub fn wallpaper_dir() -> PathBuf {
    data_home().join("undertow/wallpapers")
}
/// Built-in presets, unpacked from the binary on start.
pub fn preset_dir() -> PathBuf {
    data_home().join("undertow/presets")
}
pub fn thumb_dir() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache").join("undertow/thumbs")
}
fn config_file() -> PathBuf {
    config_home().join("undertow/config.json")
}

fn load() -> Value {
    fs::read_to_string(config_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn save(v: &Value) {
    let path = config_file();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    // Write-then-rename so a crash never leaves a half-written config.
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, serde_json::to_vec_pretty(v).unwrap_or_default()).is_ok() {
        let _ = fs::rename(tmp, path);
    }
}

pub fn current() -> String {
    let _g = LOCK.lock().unwrap();
    load()["current"].as_str().unwrap_or("snow").to_string()
}

pub fn set_current(id: &str) {
    let _g = LOCK.lock().unwrap();
    let mut c = load();
    c["current"] = json!(id);
    save(&c);
}

/// Values the user has changed for one wallpaper.
pub fn saved_props(id: &str) -> Map<String, Value> {
    let _g = LOCK.lock().unwrap();
    load()["props"][id].as_object().cloned().unwrap_or_default()
}

pub fn set_prop(id: &str, name: &str, value: Value) {
    let _g = LOCK.lock().unwrap();
    let mut c = load();
    if !c["props"].is_object() {
        c["props"] = json!({});
    }
    if !c["props"][id].is_object() {
        c["props"][id] = json!({});
    }
    c["props"][id][name] = value;
    save(&c);
}

pub fn reset_props(id: &str) {
    let _g = LOCK.lock().unwrap();
    let mut c = load();
    if let Some(props) = c["props"].as_object_mut() {
        props.remove(id);
        save(&c);
    }
}
