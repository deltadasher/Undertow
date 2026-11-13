//! The wallpaper library: discovery, manifests, and install/update/remove.
//!
//! A wallpaper is a directory containing a web page. Recognised layouts, in order:
//!   undertow.json                              { "name", "entry", "feeds": ["stats"], "props": {...} }
//!   LivelyInfo.json (+ LivelyProperties.json)  Lively Wallpaper packages
//!   index.html                                 any plain web project

use crate::config;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct Wallpaper {
    /// Directory name, lowercase. Also the subdomain it is served from.
    pub id: String,
    pub title: String,
    pub dir: PathBuf,
    /// Page to load, relative to `dir`.
    pub entry: String,
    /// "preset", "git" or "local".
    pub kind: &'static str,
    /// Wants the once-a-second system stats feed.
    pub stats: bool,
    /// `{ name: { "type", "text", "value", ... } }`, same schema as Lively.
    pub props: Map<String, Value>,
}

impl Wallpaper {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id, "title": self.title, "entry": self.entry,
            "kind": self.kind, "stats": self.stats, "props": self.props,
        })
    }

    /// Manifest defaults overlaid with whatever the user has changed.
    pub fn prop_values(&self) -> Map<String, Value> {
        let saved = config::saved_props(&self.id);
        let mut out = Map::new();
        for (name, def) in &self.props {
            // Labels and buttons have no value.
            if let Some(default) = def.get("value") {
                out.insert(name.clone(), saved.get(name).unwrap_or(default).clone());
            }
        }
        out
    }
}

// Built-in presets travel inside the binary and are unpacked on start,
// so a bare `undertow` executable is all anyone needs.
const PRESETS: &[(&str, &[u8])] = &[
    ("snow/undertow.json", include_str!("../presets/snow/undertow.json").as_bytes()),
    ("snow/index.html", include_str!("../presets/snow/index.html").as_bytes()),
    ("procmon/undertow.json", include_str!("../presets/procmon/undertow.json").as_bytes()),
    ("procmon/index.html", include_str!("../presets/procmon/index.html").as_bytes()),
    ("life/undertow.json", include_str!("../presets/life/undertow.json").as_bytes()),
    ("life/index.html", include_str!("../presets/life/index.html").as_bytes()),
    ("flow/undertow.json", include_str!("../presets/flow/undertow.json").as_bytes()),
    ("flow/index.html", include_str!("../presets/flow/index.html").as_bytes()),
    ("deepfield/undertow.json", include_str!("../presets/deepfield/undertow.json").as_bytes()),
    ("deepfield/index.html", include_str!("../presets/deepfield/index.html").as_bytes()),
    ("constellation/undertow.json", include_str!("../presets/constellation/undertow.json").as_bytes()),
    ("constellation/index.html", include_str!("../presets/constellation/index.html").as_bytes()),
    ("aurora/undertow.json", include_str!("../presets/aurora/undertow.json").as_bytes()),
    ("aurora/index.html", include_str!("../presets/aurora/index.html").as_bytes()),
    ("orrery/undertow.json", include_str!("../presets/orrery/undertow.json").as_bytes()),
    ("orrery/index.html", include_str!("../presets/orrery/index.html").as_bytes()),
    ("ink/undertow.json", include_str!("../presets/ink/undertow.json").as_bytes()),
    ("ink/index.html", include_str!("../presets/ink/index.html").as_bytes()),
    ("overview/undertow.json", include_str!("../presets/overview/undertow.json").as_bytes()),
    ("overview/index.html", include_str!("../presets/overview/index.html").as_bytes()),
    ("glyphs/undertow.json", include_str!("../presets/glyphs/undertow.json").as_bytes()),
    ("glyphs/index.html", include_str!("../presets/glyphs/index.html").as_bytes()),
    ("contours/undertow.json", include_str!("../presets/contours/undertow.json").as_bytes()),
    ("contours/index.html", include_str!("../presets/contours/index.html").as_bytes()),
    ("flock/undertow.json", include_str!("../presets/flock/undertow.json").as_bytes()),
    ("flock/index.html", include_str!("../presets/flock/index.html").as_bytes()),
    ("nebula/undertow.json", include_str!("../presets/nebula/undertow.json").as_bytes()),
    ("nebula/index.html", include_str!("../presets/nebula/index.html").as_bytes()),
    ("ripples/undertow.json", include_str!("../presets/ripples/undertow.json").as_bytes()),
    ("ripples/index.html", include_str!("../presets/ripples/index.html").as_bytes()),
    ("fireflies/undertow.json", include_str!("../presets/fireflies/undertow.json").as_bytes()),
    ("fireflies/index.html", include_str!("../presets/fireflies/index.html").as_bytes()),
    ("fireflies/scene.wasm", include_bytes!("../presets/fireflies/scene.wasm")),
    ("tide/undertow.json", include_str!("../presets/tide/undertow.json").as_bytes()),
    ("tide/index.html", include_str!("../presets/tide/index.html").as_bytes()),
    ("tide/scene.wasm", include_bytes!("../presets/tide/scene.wasm")),
    ("loom/undertow.json", include_str!("../presets/loom/undertow.json").as_bytes()),
    ("loom/index.html", include_str!("../presets/loom/index.html").as_bytes()),
    ("loom/scene.wasm", include_bytes!("../presets/loom/scene.wasm")),
    ("dunes/undertow.json", include_str!("../presets/dunes/undertow.json").as_bytes()),
    ("dunes/index.html", include_str!("../presets/dunes/index.html").as_bytes()),
    ("dunes/scene.wasm", include_bytes!("../presets/dunes/scene.wasm")),
    ("eclipse/undertow.json", include_str!("../presets/eclipse/undertow.json").as_bytes()),
    ("eclipse/index.html", include_str!("../presets/eclipse/index.html").as_bytes()),
    ("eclipse/scene.wasm", include_bytes!("../presets/eclipse/scene.wasm")),
    ("rainroom/undertow.json", include_str!("../presets/rainroom/undertow.json").as_bytes()),
    ("rainroom/index.html", include_str!("../presets/rainroom/index.html").as_bytes()),
    ("rainroom/scene.wasm", include_bytes!("../presets/rainroom/scene.wasm")),
];

pub fn unpack_presets() {
    let root = config::preset_dir();
    for (rel, content) in PRESETS {
        let path = root.join(rel);
        if fs::read(&path).map(|old| old == *content).unwrap_or(false) {
            continue;
        }
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&path, content);
    }
}

fn read_json(path: &Path) -> Option<Map<String, Value>> {
    let v: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    v.as_object().cloned()
}

fn load(path: &Path, preset: bool) -> Option<Wallpaper> {
    let dir = path.canonicalize().ok()?;
    if !dir.is_dir() {
        return None;
    }
    let name = path.file_name()?.to_string_lossy().to_string();
    let mut w = Wallpaper {
        id: name.to_lowercase(),
        title: name,
        entry: "index.html".into(),
        kind: if preset {
            "preset"
        } else if dir.join(".git").exists() {
            "git"
        } else {
            "local"
        },
        stats: false,
        props: Map::new(),
        dir,
    };

    if let Some(own) = read_json(&w.dir.join("undertow.json")) {
        if let Some(s) = own.get("name").and_then(Value::as_str) {
            w.title = s.into();
        }
        if let Some(s) = own.get("entry").and_then(Value::as_str) {
            w.entry = s.into();
        }
        if let Some(p) = own.get("props").and_then(Value::as_object) {
            w.props = p.clone();
        }
        w.stats = own
            .get("feeds")
            .and_then(Value::as_array)
            .is_some_and(|feeds| feeds.iter().any(|f| f == "stats"));
    } else if let Some(lively) = read_json(&w.dir.join("LivelyInfo.json")) {
        if let Some(s) = lively.get("Title").and_then(Value::as_str) {
            w.title = s.into();
        }
        if let Some(s) = lively.get("FileName").and_then(Value::as_str) {
            w.entry = s.into();
        }
    }
    if w.props.is_empty() {
        w.props = read_json(&w.dir.join("LivelyProperties.json")).unwrap_or_default();
    }
    w.dir.join(&w.entry).is_file().then_some(w)
}

/// User wallpapers first, so one can shadow a preset of the same name.
fn roots() -> [(PathBuf, bool); 2] {
    [(config::wallpaper_dir(), false), (config::preset_dir(), true)]
}

pub fn all() -> Vec<Wallpaper> {
    let mut list: Vec<Wallpaper> = Vec::new();
    for (root, preset) in roots() {
        let Ok(entries) = fs::read_dir(&root) else { continue };
        let mut found: Vec<Wallpaper> = entries
            .flatten()
            .filter_map(|e| load(&e.path(), preset))
            .filter(|w| !list.iter().any(|have| have.id == w.id))
            .collect();
        found.sort_by(|a, b| a.id.cmp(&b.id));
        list.extend(found);
    }
    list
}

pub fn find(id: &str) -> Option<Wallpaper> {
    let id = id.to_lowercase();
    if id.is_empty() || id.contains('/') || id.starts_with('.') {
        return None;
    }
    // Directory names keep their original case on disk, ids are lowercase.
    all().into_iter().find(|w| w.id == id)
}

fn sanitize_id(raw: &str) -> String {
    raw.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn git(args: &[&str], cwd: Option<&Path>) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.args(args).env("GIT_TERMINAL_PROMPT", "0");
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd.output().map_err(|e| format!("could not run git: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(stderr.lines().last().unwrap_or("git failed").trim().to_string())
}

struct Source {
    url: String,
    repo: String,
    reference: String,
    sub: String,
}

/// Accepted specs:
///   owner/repo      owner/repo/sub/dir      owner/repo@branch
///   https://github.com/owner/repo[/tree/<ref>/<sub/dir>]
///   any other git URL
fn parse_spec(spec: &str) -> Option<Source> {
    let spec = spec.trim().trim_end_matches('/');
    let github = spec
        .strip_prefix("https://github.com/")
        .or_else(|| spec.strip_prefix("http://github.com/"))
        .or_else(|| spec.strip_prefix("github.com/"));

    if let Some(rest) = github {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
            return None;
        }
        let repo = parts[1].trim_end_matches(".git").to_string();
        let (reference, sub) = if parts.len() >= 4 && parts[2] == "tree" {
            (parts[3].to_string(), parts[4..].join("/"))
        } else {
            (String::new(), String::new())
        };
        return Some(Source { url: format!("https://github.com/{}/{}.git", parts[0], repo), repo, reference, sub });
    }
    if spec.contains("://") || spec.starts_with("git@") {
        let repo = spec.rsplit(['/', ':']).next()?.trim_end_matches(".git").to_string();
        return Some(Source { url: spec.to_string(), repo, reference: String::new(), sub: String::new() });
    }

    let (path, reference) = spec.split_once('@').unwrap_or((spec, ""));
    let parts: Vec<&str> = path.split('/').collect();
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    if parts.len() < 2 || !ok(parts[0]) || !ok(parts[1]) {
        return None;
    }
    Some(Source {
        url: format!("https://github.com/{}/{}.git", parts[0], parts[1]),
        repo: parts[1].to_string(),
        reference: reference.to_string(),
        sub: parts[2..].join("/"),
    })
}

/// Installs a wallpaper and returns its id.
pub fn add(spec: &str) -> Result<String, String> {
    let root = config::wallpaper_dir();
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    // A local directory is symlinked, which is handy while developing a wallpaper.
    let local = Path::new(spec);
    if local.is_dir() {
        let target = local.canonicalize().map_err(|e| e.to_string())?;
        let id = sanitize_id(&target.file_name().unwrap_or_default().to_string_lossy());
        let dest = root.join(&id);
        if dest.symlink_metadata().is_ok() {
            return Err(format!("'{id}' is already installed"));
        }
        std::os::unix::fs::symlink(&target, &dest).map_err(|e| e.to_string())?;
        return find(&id).map(|w| w.id).ok_or_else(|| {
            let _ = fs::remove_file(&dest);
            format!("{} has no index.html or undertow.json", target.display())
        });
    }

    let src = parse_spec(spec)
        .ok_or_else(|| format!("don't know how to fetch '{spec}' (try owner/repo or a GitHub URL)"))?;
    let mut id = sanitize_id(&src.repo);
    if let Some(leaf) = src.sub.rsplit('/').next().filter(|s| !s.is_empty()) {
        id = sanitize_id(&format!("{id}-{}", leaf.trim_end_matches(".html")));
    }
    if id.is_empty() {
        return Err(format!("can't derive a name from '{spec}'"));
    }
    let dest = root.join(&id);
    if dest.symlink_metadata().is_ok() {
        return Err(format!("'{id}' is already installed"));
    }

    let dest_str = dest.to_string_lossy().to_string();
    let mut args = vec!["clone", "--depth", "1"];
    if !src.reference.is_empty() {
        args.extend(["--branch", &src.reference]);
    }
    args.extend(["--", &src.url, &dest_str]);
    git(&args, None)?;

    // Point at a page in a subdirectory without touching tracked files.
    if !src.sub.is_empty() && !dest.join("undertow.json").exists() {
        let entry = if src.sub.ends_with(".html") || src.sub.ends_with(".htm") {
            src.sub.clone()
        } else {
            format!("{}/index.html", src.sub)
        };
        let manifest = json!({ "name": id, "entry": entry });
        let _ = fs::write(dest.join("undertow.json"), serde_json::to_vec_pretty(&manifest).unwrap_or_default());
    }

    if find(&id).is_none() {
        let _ = fs::remove_dir_all(&dest);
        return Err(format!(
            "{} has no web page at its top level. If the page is in a subdirectory, add it as {}/<subdir>. \
             Projects that need a build step (npm etc.) can't be installed directly.",
            src.repo,
            spec.trim_end_matches('/')
        ));
    }
    Ok(id)
}

/// `git pull` one wallpaper, or every git-installed one when `id` is empty.
pub fn update(id: &str) -> Result<String, String> {
    let targets: Vec<Wallpaper> = if id.is_empty() {
        all().into_iter().filter(|w| w.kind == "git").collect()
    } else {
        let w = find(id).ok_or_else(|| format!("no wallpaper '{id}'"))?;
        if w.kind != "git" {
            return Err(format!("'{}' was not installed from git", w.id));
        }
        vec![w]
    };
    for w in &targets {
        git(&["pull", "--ff-only"], Some(&w.dir)).map_err(|e| format!("{}: {e}", w.id))?;
    }
    Ok(match targets.len() {
        0 => "nothing to update".into(),
        1 => format!("updated {}", targets[0].id),
        n => format!("updated {n} wallpapers"),
    })
}

pub fn remove(id: &str) -> Result<String, String> {
    let w = find(id).ok_or_else(|| format!("no wallpaper '{id}'"))?;
    if w.kind == "preset" {
        return Err("built-in presets can't be removed".into());
    }
    // Find the entry by id rather than trusting the id as a path component.
    let entry = fs::read_dir(config::wallpaper_dir())
        .map_err(|e| e.to_string())?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().to_lowercase() == w.id)
        .ok_or_else(|| format!("no wallpaper '{id}'"))?;
    let path = entry.path();
    let is_link = path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false);
    if is_link { fs::remove_file(&path) } else { fs::remove_dir_all(&path) }.map_err(|e| e.to_string())?;
    config::reset_props(&w.id);
    let _ = fs::remove_file(config::thumb_dir().join(format!("{}.png", w.id)));
    Ok(format!("removed {}", w.id))
}
