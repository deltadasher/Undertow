# undertow

Interactive web wallpapers for Wayland. A wallpaper is just a web page (canvas, WebGL, plain DOM);
undertow renders it with Chromium (QtWebEngine) on the compositor's background layer via
`wlr-layer-shell`, one surface per monitor, and runs as a tray app with a picker window.

Works on compositors that implement layer-shell: **niri, Hyprland, KDE Plasma, sway, river**. Not GNOME.

Written in Rust. The windows, picker and tray are QML, bound with [cxx-qt](https://github.com/KDAB/cxx-qt);
one ~30-line C++ file starts the Qt application.

## Build

Needs Rust, a C++ compiler, Qt 6 (base, declarative, webengine), LayerShellQt, and `git` at runtime.

```bash
# Arch
sudo pacman -S --needed rust qt6-base qt6-declarative qt6-webengine layer-shell-qt git

cargo build --release
# the result is one self-contained binary: target/release/undertow
```

Install it with `cargo install --path .`, or system-wide with `sudo make install` (binary, desktop
entry and icon under `/usr/local`; `PREFIX` and `DESTDIR` are honoured for packaging). Presets are
embedded in the binary.

undertow is an ordinary desktop app: after the first run of any `undertow` command it has an entry in
the app launcher, so KRunner, Kickoff, fuzzel, rofi and the like find it under "Undertow", "wallpaper"
or "live". The entry also carries Pause, Resume and Quit actions.

## Use

Run `undertow`. The picker opens: click a wallpaper to apply it, adjust its settings in the side
panel, paste `owner/repo` or a GitHub URL to install one. Closing the window leaves undertow
running in the system tray; the tray icon reopens the picker, pauses, or quits.

Tick **Start at login** in the picker and undertow starts itself in the tray on every login,
with no shell or compositor config involved. **Show in app launcher** is on by default; untick it
to remove the entry and keep it removed.

On niri/Hyprland the tray icon needs a bar with a tray (e.g. waybar); running `undertow` again
always reopens the picker either way.

### Command line (optional)

```bash
undertow --background         # start in the tray without opening the picker
undertow --headless           # no tray and no picker, for shells with their own interface
undertow list --json          # library, settings, thumbnails and state for such shells
undertow list
undertow set snow             # starts undertow if it isn't running
undertow props
undertow prop density 900     # change a setting live
undertow add owner/repo
undertow add owner/repo/path/to/page          # page in a subdirectory
undertow add owner/repo@branch
undertow add https://github.com/owner/repo/tree/main/demo
undertow add ~/code/my-wallpaper              # symlink a local directory (for development)
undertow update [id]
undertow remove <id>
undertow pause | resume | reload | quit
```

Installed wallpapers live in `~/.local/share/undertow/wallpapers/<id>/`. Projects that need a
build step (npm etc.) can't be installed directly; build them, then `undertow add` the output
directory. `undertow set https://example.com` shows a live URL.

Lively Wallpaper packages work as they are: `LivelyInfo.json` and `LivelyProperties.json` are
read, and `livelyPropertyListener(name, value)` is called for every setting.

## Built-in presets

| id | what it is | pointer |
|---|---|---|
| `snow` | layered snowfall with drifts that pile up along the bottom | move to stir the wind, click to kick a drift |
| `procmon` | live system monitor: per-core CPU, history, memory, network, top processes | - |
| `life` | Conway's Game of Life, cells coloured by age, re-seeds itself when it stalls | move to seed cells, click for a burst |
| `flow` | particles following a drifting vector field, with trails | pointer is a whirlpool, click scatters |
| `deepfield` | drifting star field in three depths, twinkling, with occasional meteors | the view leans toward the pointer, click for a meteor |
| `constellation` | drifting points that link to every neighbour in reach, forming and dissolving figures | the pointer links to nearby stars, click pushes them apart |
| `aurora` | two rayed aurora curtains over a starry sky | curtains lift toward the pointer, click for a surge |
| `orrery` | bodies on Keplerian orbits with fading trails around a central star | the system leans toward the pointer, click launches a comet |
| `ink` | white ink sinking and curling through black water (GPU fluid simulation; can be inverted) | drag to stir and trail ink, click to let a large drop go |
| `overview` | time and date, weather for a city you name, CPU, memory, network and the busiest processes | - |
| `glyphs` | columns of falling characters with fading trails (hex, binary, kana, box drawing or mixed) | columns near the pointer run faster, click restarts them |
| `contours` | a topographic map of terrain that keeps shifting, with index contours | the pointer raises a hill, click sends a ripple through the land |
| `flock` | a murmuration of birds against a dusk or dawn sky | the pointer is a hawk they swerve around, click startles them |
| `nebula` | slow clouds of gas over a star field (fragment shader) | clouds part around the pointer, click sends a pulse of light |
| `ripples` | a pond in the rain, simulated with the wave equation and lit from its slope | moving drags a wake, click drops a stone |
| `fireflies` | glowing fireflies over a moonlit meadow | move to nudge them away |
| `tide` | layered waves meeting a sandy shore | move to disturb the water |
| `loom` | gently shifting threads with a copper accent | move to bend the threads |
| `dunes` | wind-sculpted desert under a low sun | - |
| `eclipse` | a dark sun with a moving corona | - |
| `rainroom` | rain on glass with distant city lights | - |

These six scenes are written in Rust in `wallpaper-engine/` and compiled to WebAssembly. Their small HTML adapters connect the Rust renderer to the browser canvas and Undertow settings. To rebuild them, install the `wasm32-unknown-unknown` target, run `cargo build --release --target wasm32-unknown-unknown --manifest-path wallpaper-engine/Cargo.toml`, and copy the resulting `wallpaper-engine/target/wasm32-unknown-unknown/release/undertow_scenes.wasm` into each of the six preset folders as `scene.wasm`.

## Writing a wallpaper

A directory with an `index.html`. Optional `undertow.json`:

```json
{
  "name": "My wallpaper",
  "entry": "index.html",
  "feeds": ["stats"],
  "props": {
    "speed": { "type": "slider", "text": "Speed", "value": 1, "min": 0, "max": 5, "step": 0.1 },
    "glow":  { "type": "checkbox", "text": "Glow", "value": true },
    "tint":  { "type": "color", "text": "Tint", "value": "#88ccff" },
    "mode":  { "type": "dropdown", "text": "Mode", "value": 0, "items": ["calm", "wild"] },
    "label": { "type": "textbox", "text": "Caption", "value": "" }
  }
}
```

`props` become controls in the picker. undertow injects a small `window.undertow` object before
the page's own scripts run:

```js
undertow.on('prop', ({ name, value }) => { /* once per prop on load, then on every change */ });
undertow.on('stats', s => { /* once a second, only with "feeds": ["stats"] */ });
```

`stats` is `{ cpu: { total, cores[] } (0..1), mem: { total, used, swapTotal, swapUsed } (kB),
net: { rx, tx } (bytes/s), load[3], uptime (s), temp (°C or null), host, kernel, nprocs,
procs: [{ pid, name, cpu (% of one core), mem (kB) }] }`.

Each wallpaper is served from its own origin, `http://<id>.localhost:<port>/`, by a loopback-only
server inside undertow, so `fetch()`, ES modules and absolute paths work. Guard with
`window.undertow?.on(...)` and the page also runs in a normal browser while you work on it.

## Layout

```
src/main.rs      CLI and startup
src/backend.rs   the Backend QML singleton (cxx-qt bridge)
src/library.rs   wallpaper discovery, manifests, git install/update/remove
src/server.rs    loopback file server
src/stats.rs     /proc sampler
src/config.rs    paths and config.json
src/ipc.rs       CLI <-> daemon socket, single-instance lock
src/desktop.rs   autostart and launcher entries
qml/             Main (state, tray), WallWindow (layer-shell web view), Picker, PropEditor
cpp/shim.cpp     QApplication startup
presets/         built-in wallpapers
```

## Notes and limits

- **Plasma:** undertow sits on top of Plasma's own desktop layer, so desktop icons and the
  desktop right-click menu are covered while it runs. Pause or quit to get them back.
- No automatic pause when a fullscreen window covers the wallpaper yet.
- Audio is muted; there is no audio-reactive feed yet.
- Third-party wallpapers are ordinary web content in Chromium's sandbox. They can make network
  requests like any web page, so install ones you'd be happy to open in a browser.

`overview` fetches weather from [Open-Meteo](https://open-meteo.com) once a city is set in its settings.
Until then it makes no network request, and it never guesses a location from the network address.
