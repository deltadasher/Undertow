// The application: owns the current-wallpaper state, one WallWindow per screen,
// the picker window and the tray icon. All persistence goes through Backend (Rust).
import QtQml
import QtQuick
import Qt.labs.platform as Platform
import dev.undertow

QtObject {
    id: app

    property var wallpapers: []        // manifests from Backend.wallpapers()
    property string currentId: ""      // library id, or an http(s) URL
    property var current: null         // manifest of currentId; null for a URL
    property string currentUrl: ""
    property var propValues: ({})      // name -> value for the current wallpaper
    property int propsRevision: 0      // bumped when propValues is replaced wholesale
    property int thumbRevision: 0      // bumped when the current wallpaper's thumbnail is rewritten
    property bool paused: false
    property string status: ""
    property bool statusOk: true

    signal pushProp(string name, var value)
    signal pushAllProps()
    signal statsTick(string json)
    signal reloadViews()

    // Injected into every page before its own scripts run.
    readonly property string bridgeSource: `
(function () {
  if (window.undertow) return;
  const props = {};
  window.undertow = {
    stats: null,
    props,
    on(event, fn) {
      window.addEventListener('undertow:' + event, e => fn(e.detail));
      if (event === 'stats' && this.stats) fn(this.stats);
      if (event === 'prop') for (const k in props) fn({ name: k, value: props[k] });
    },
    _emit(event, detail) {
      if (event === 'stats') this.stats = detail;
      if (event === 'prop') {
        props[detail.name] = detail.value;
        // Lively Wallpaper compatibility
        try {
          if (typeof window.livelyPropertyListener === 'function')
            window.livelyPropertyListener(detail.name, detail.value);
        } catch (e) { console.error(e); }
      }
      window.dispatchEvent(new CustomEvent('undertow:' + event, { detail }));
    },
  };
})();`

    function isUrl(id) {
        return id.startsWith("http://") || id.startsWith("https://")
    }

    function refresh() {
        wallpapers = JSON.parse(Backend.wallpapers())
        if (currentId === "" || isUrl(currentId))
            return
        const found = wallpapers.find(w => w.id === currentId)
        if (found)
            current = found
        else
            apply("snow")   // the wallpaper on screen was removed
    }

    function apply(id) {
        let manifest = null
        let url = id
        if (!isUrl(id)) {
            manifest = wallpapers.find(w => w.id === id)
            if (!manifest)
                return false
            url = "http://" + manifest.id + ".localhost:" + Backend.port + "/" + manifest.entry
        }
        currentId = id
        current = manifest
        propValues = manifest ? JSON.parse(Backend.propValues(id)) : ({})
        propsRevision++
        Backend.setStatsEnabled(manifest ? manifest.stats : false)
        Backend.setCurrent(id)
        if (currentUrl === url)
            reloadViews()
        else
            currentUrl = url
        return true
    }

    function setProp(name, value) {
        if (!current)
            return
        Backend.setProp(currentId, name, JSON.stringify(value))
        propValues[name] = value   // in place: the editors must not be rebuilt mid-drag
        pushProp(name, value)
    }

    function reloadProps() {
        if (!current)
            return
        propValues = JSON.parse(Backend.propValues(currentId))
        propsRevision++
        pushAllProps()
    }

    function resetProps() {
        if (!current)
            return
        Backend.resetProps(currentId)
        reloadProps()
    }

    function showPicker() {
        picker.show()
        picker.raise()
        picker.requestActivate()
    }

    function say(ok, message) {
        statusOk = ok
        status = message
    }

    // Commands arriving from the CLI over the socket.
    function handleCommand(line) {
        const space = line.indexOf(" ")
        const cmd = space < 0 ? line : line.slice(0, space)
        const arg = space < 0 ? "" : line.slice(space + 1).trim()
        switch (cmd) {
        case "show": showPicker(); break
        case "set": refresh(); apply(arg); break
        case "refresh": refresh(); break
        case "refresh-props": reloadProps(); break
        case "reload": reloadViews(); break
        case "pause": paused = true; break
        case "resume": paused = false; break
        case "quit": Qt.quit(); break
        }
    }

    property Connections backendEvents: Connections {
        target: Backend
        function onCommand(line) { app.handleCommand(line) }
        function onStats(json) { app.statsTick(json) }
        function onJobFinished(kind, ok, message, detail) {
            app.say(ok, message)
            app.refresh()
            if (ok && kind === "add")
                app.apply(detail)
            else if (ok && kind === "update" && detail === app.currentId)
                app.reloadViews()
        }
    }

    property Instantiator walls: Instantiator {
        model: Qt.application.screens
        delegate: WallWindow { ctl: app }
    }

    property Picker picker: Picker { ctl: app }

    property Platform.SystemTrayIcon tray: Platform.SystemTrayIcon {
        visible: true
        icon.source: "qrc:/qt/qml/dev/undertow/assets/undertow.svg"
        tooltip: "undertow: " + (app.current ? app.current.title : app.currentId)
        onActivated: reason => {
            if (reason !== Platform.SystemTrayIcon.Context)
                app.showPicker()
        }
        menu: Platform.Menu {
            Platform.MenuItem { text: "Choose wallpaper…"; onTriggered: app.showPicker() }
            Platform.MenuItem { text: app.paused ? "Resume" : "Pause"; onTriggered: app.paused = !app.paused }
            Platform.MenuItem { text: "Reload"; onTriggered: app.reloadViews() }
            Platform.MenuSeparator {}
            Platform.MenuItem { text: "Quit undertow"; onTriggered: Qt.quit() }
        }
    }

    Component.onCompleted: {
        wallpapers = JSON.parse(Backend.wallpapers())
        if (!apply(Backend.current()))
            apply("snow")
        if (!Backend.startHidden())
            showPicker()
    }
}
