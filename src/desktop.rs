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

fn exe() -> String {
    env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "undertow".into())
}

fn autostart_entry() -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Undertow\nComment=Interactive live wallpapers\n\
         Exec=\"{}\" --background\nIcon=undertow\nTerminal=false\nNoDisplay=true\n\
         X-GNOME-Autostart-enabled=true\n",
        exe()
    )
}

/// The entry app launchers index (KRunner, Kickoff, fuzzel, rofi, wofi...).
/// The keywords are what make a search for "wallpaper" find it.
fn launcher_entry() -> String {
    let exe = exe();
    format!(
        "[Desktop Entry]\nType=Application\nName=Undertow\nGenericName=Live Wallpaper\n\
         Comment=Pick and install interactive live wallpapers\n\
         Keywords=wallpaper;live;animated;interactive;background;desktop;lively;web;\n\
         Exec=\"{exe}\"\nTryExec={exe}\nIcon=undertow\nTerminal=false\n\
         Categories=Settings;DesktopSettings;\nStartupNotify=true\nStartupWMClass=undertow\n\
         SingleMainWindow=true\nActions=pause;resume;quit;\n\n\
         [Desktop Action pause]\nName=Pause Wallpaper\nExec=\"{exe}\" pause\n\n\
         [Desktop Action resume]\nName=Resume Wallpaper\nExec=\"{exe}\" resume\n\n\
         [Desktop Action quit]\nName=Quit Undertow\nExec=\"{exe}\" quit\n"
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
        write(autostart_file(), &autostart_entry());
    } else {
        let _ = fs::remove_file(autostart_file());
    }
}

pub fn launcher_enabled() -> bool {
    launcher_file().exists() || packaged()
}

/// A copy installed by a package or `make install` already makes undertow searchable.
fn packaged() -> bool {
    let dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.split(':').any(|dir| !dir.is_empty() && PathBuf::from(dir).join("applications/undertow.desktop").exists())
}

/// Called when the app starts with its own interface: undertow is findable in
/// the app launcher unless the user has turned that off. The entry is rewritten
/// when the binary has moved, so it never points at a stale path.
pub fn ensure_launcher() {
    if !config::flag("launcher", true) || packaged() {
        return;
    }
    let wanted = launcher_entry();
    if fs::read_to_string(launcher_file()).ok().as_deref() != Some(wanted.as_str()) {
        write(icon_file(), ICON);
        write(launcher_file(), &wanted);
    }
}

pub fn set_launcher(on: bool) {
    config::set_flag("launcher", on);
    if on {
        write(icon_file(), ICON);
        write(launcher_file(), &launcher_entry());
    } else {
        let _ = fs::remove_file(launcher_file());
    }
}
