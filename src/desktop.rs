//! Desktop integration: the "start at login" entry and the app-launcher entry.
//! Both are plain .desktop files pointing at wherever this binary currently lives.

use crate::config;
use std::path::PathBuf;
use std::{env, fs};

const ICON: &str = include_str!("../assets/undertow.svg");

fn autostart_file() -> PathBuf {
    config::config_home().join("autostart/undertow.desktop")
}
fn launcher_file() -> PathBuf {
    config::data_home().join("applications/undertow.desktop")
}
fn icon_file() -> PathBuf {
    config::data_home().join("icons/hicolor/scalable/apps/undertow.svg")
}

fn entry(args: &str, extra: &str) -> String {
    let exe = env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "undertow".into());
    format!(
        "[Desktop Entry]\nType=Application\nName=undertow\nComment=Interactive web wallpapers\n\
         Exec=\"{exe}\"{args}\nIcon=undertow\nCategories=Utility;\n{extra}"
    )
}

fn write(path: PathBuf, content: &str) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Err(e) = fs::write(&path, content) {
        eprintln!("undertow: could not write {}: {e}", path.display());
    }
}

pub fn autostart_enabled() -> bool {
    autostart_file().exists()
}

pub fn set_autostart(on: bool) {
    if on {
        write(icon_file(), ICON);
        write(autostart_file(), &entry(" --background", "X-GNOME-Autostart-enabled=true\n"));
    } else {
        let _ = fs::remove_file(autostart_file());
    }
}

pub fn launcher_enabled() -> bool {
    launcher_file().exists()
}

pub fn set_launcher(on: bool) {
    if on {
        write(icon_file(), ICON);
        write(launcher_file(), &entry("", "StartupWMClass=undertow\n"));
    } else {
        let _ = fs::remove_file(launcher_file());
    }
}
