# undertow

Interactive web wallpapers for Wayland. A wallpaper is just a web page (canvas, WebGL, plain DOM);
undertow renders it with QtWebEngine (Chromium) on the compositor's background layer via
`wlr-layer-shell`, one surface per monitor.

Works on compositors that implement layer-shell: **niri, Hyprland, KDE Plasma, sway, river**. Not GNOME.

## Build

Needs Qt 6 (base, webengine), LayerShellQt, CMake, a C++20 compiler, and `git` at runtime.

```bash
# Arch
sudo pacman -S --needed qt6-base qt6-webengine layer-shell-qt cmake ninja git

cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
sudo cmake --install build      # optional; build/undertow also runs in place
```

## Use

```bash
undertow list                 # what's installed
undertow set snow             # switch wallpaper; starts the daemon if it isn't running
undertow props                # settings of the current wallpaper
undertow prop density 900     # change one live (remembered per wallpaper)
undertow quit
```

Autostart: run `undertow run` from your compositor config
(`spawn-at-startup "undertow" "run"` in niri, `exec-once = undertow run` in Hyprland,
or an autostart entry in Plasma).

### Wallpapers from GitHub

Any repository that contains a web page works:

```bash
undertow add owner/repo
undertow add owner/repo/path/to/page          # page in a subdirectory
undertow add owner/repo@branch
undertow add https://github.com/owner/repo/tree/main/demo
undertow add ~/code/my-wallpaper              # symlink a local directory (for development)
undertow update                               # git pull everything
undertow remove repo
```

They land in `~/.local/share/undertow/wallpapers/<id>/`. Projects that need a build step
(npm, etc.) have to be built first; then point `"entry"` in that directory's `undertow.json`
at the built `index.html`. `undertow set https://example.com` shows a live URL instead.

Lively Wallpaper packages work as they are: `LivelyInfo.json` and `LivelyProperties.json`
are read, and `livelyPropertyListener(name, value)` is called for every setting.

## Built-in presets

| id | what it is | pointer |
|---|---|---|
| `snow` | layered snowfall with drifts that pile up along the bottom | move to stir the wind, click to kick a drift |
| `procmon` | live system monitor: per-core CPU, history, memory, network, top processes | - |
| `life` | Conway's Game of Life, cells coloured by age, re-seeds itself when it stalls | move to seed cells, click for a burst |
| `flow` | particles following a drifting vector field, with trails | pointer is a whirlpool, click scatters |

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
    "tint":  { "type": "color", "text": "Tint", "value": "#88ccff" }
  }
}
```

undertow injects a small `window.undertow` object before the page's own scripts run:

```js
undertow.on('prop', ({ name, value }) => { /* called once per prop on load, then on every change */ });
undertow.on('stats', s => { /* once a second, only with "feeds": ["stats"] */ });
```

`stats` is `{ cpu: { total, cores[] } (0..1), mem: { total, used, swapTotal, swapUsed } (kB),
net: { rx, tx } (bytes/s), load[3], uptime (s), temp (°C, if found), host, kernel, nprocs,
procs: [{ pid, name, cpu (% of one core), mem (kB) }] }`.

Pages are served from `undertow://<id>/`, so `fetch()` and ES modules work. Guard with
`window.undertow?.on(...)` and the page also runs in a normal browser while you work on it.

## Notes and limits

- **Plasma:** undertow sits on top of Plasma's own desktop layer, so desktop icons and the
  desktop right-click menu are covered while it runs. Panels and windows are unaffected.
- No automatic pause when a fullscreen window covers the wallpaper yet.
- Audio is muted; there is no audio-reactive feed yet.
- Settings are CLI-only for now (`undertow props` / `undertow prop`).
- Third-party wallpapers are ordinary web content running in Chromium's sandbox. They can make
  network requests like any web page, so install ones you'd be happy to open in a browser.
