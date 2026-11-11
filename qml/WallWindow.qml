// One full-screen web view on an output's background layer (wlr-layer-shell).
import QtQuick
import QtQuick.Window
import QtWebEngine
import org.kde.layershell as LayerShell
import dev.undertow

Window {
    id: win

    required property var ctl          // the app object from Main.qml
    required property var modelData    // the screen, from the Instantiator
    required property int index

    // The window must already be on its output when the layer surface is first
    // mapped, so it stays hidden until the screen has been assigned.
    property bool placed: false
    x: modelData.virtualX
    y: modelData.virtualY
    width: modelData.width
    height: modelData.height
    visible: placed && !ctl.paused
    Component.onCompleted: {
        screen = modelData
        placed = true
    }
    color: "black"
    flags: Qt.FramelessWindowHint
    title: "undertow"

    LayerShell.Window.scope: "undertow"
    LayerShell.Window.layer: LayerShell.Window.LayerBackground
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorBottom
                               | LayerShell.Window.AnchorLeft | LayerShell.Window.AnchorRight
    LayerShell.Window.exclusionZone: -1   // cover the whole output, ignore panels
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityNone

    function emit(event, detailJson) {
        view.runJavaScript("window.undertow && window.undertow._emit(" + JSON.stringify(event) + "," + detailJson + ");")
    }
    function pushAll() {
        for (const name in ctl.propValues)
            emit("prop", JSON.stringify({ name: name, value: ctl.propValues[name] }))
    }

    WebEngineView {
        id: view
        anchors.fill: parent
        url: win.ctl.currentUrl
        audioMuted: true
        backgroundColor: "black"

        settings.showScrollBars: false
        settings.playbackRequiresUserGesture: false
        settings.webGLEnabled: true
        settings.accelerated2dCanvasEnabled: true
        settings.localContentCanAccessRemoteUrls: true

        userScripts.collection: [{
            name: "undertow-bridge",
            sourceCode: win.ctl.bridgeSource,
            injectionPoint: WebEngineScript.DocumentCreation,
            worldId: WebEngineScript.MainWorld
        }]

        onContextMenuRequested: request => { request.accepted = true }
        onLoadingChanged: info => {
            if (info.status === WebEngineView.LoadSucceededStatus) {
                win.pushAll()
                if (win.index === 0)
                    thumbTimer.restart()
            }
        }
    }

    // A few seconds after a wallpaper loads, keep a snapshot of it for the picker.
    Timer {
        id: thumbTimer
        interval: 4000
        onTriggered: {
            const id = win.ctl.currentId
            if (!win.visible || id === "")
                return
            view.grabToImage(result => {
                if (id === win.ctl.currentId && result.saveToFile(Backend.thumbPath(id)))
                    win.ctl.thumbRevision++
            }, Qt.size(480, Math.round(480 * view.height / Math.max(1, view.width))))
        }
    }

    Connections {
        target: win.ctl
        function onPushProp(name, value) { win.emit("prop", JSON.stringify({ name: name, value: value })) }
        function onPushAllProps() { win.pushAll() }
        function onStatsTick(json) { win.emit("stats", json) }
        function onReloadViews() { view.reload() }
    }
}
