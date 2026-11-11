// The picker: a grid of installed wallpapers, an "add from GitHub" box,
// and a side panel with the current wallpaper's settings.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import dev.undertow

ApplicationWindow {
    id: win

    required property var ctl   // the app object from Main.qml

    readonly property color bg: "#0b0f14"
    readonly property color panel: "#111821"
    readonly property color line: "#1e2935"
    readonly property color fg: "#d7e0ea"
    readonly property color dim: "#7d8b9a"
    readonly property color accent: "#5fd7a7"
    readonly property color danger: "#ff6b5e"

    width: 1080
    height: 700
    minimumWidth: 760
    minimumHeight: 480
    visible: false
    title: "undertow"
    color: bg
    background: Rectangle { color: win.bg }

    palette.window: bg
    palette.windowText: fg
    palette.base: "#0e141b"
    palette.text: fg
    palette.button: "#18212c"
    palette.buttonText: fg
    palette.highlight: accent
    palette.highlightedText: "#06110c"
    palette.placeholderText: dim
    palette.mid: "#3a4a5c"
    palette.dark: accent
    palette.light: "#223040"
    palette.midlight: "#1b2530"

    component Btn: Button {
        id: btn
        property color tint: win.fg
        implicitHeight: 34
        leftPadding: 14
        rightPadding: 14
        contentItem: Text {
            text: btn.text
            color: btn.enabled ? btn.tint : win.dim
            font: btn.font
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: 6
            color: btn.down ? "#223040" : btn.hovered ? "#1b2530" : "#151d27"
            border.color: win.line
        }
    }

    function installFromField() {
        const spec = addField.text.trim()
        if (spec === "" || Backend.busy)
            return
        win.ctl.say(true, "Fetching " + spec + "…")
        Backend.add(spec)
        addField.text = ""
    }

    header: Rectangle {
        height: 60
        color: win.panel
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 12
            Image {
                source: "qrc:/qt/qml/dev/undertow/assets/undertow.svg"
                sourceSize: Qt.size(30, 30)
            }
            Label { text: "undertow"; font.pixelSize: 19; font.weight: Font.DemiBold; color: win.fg }
            Item { Layout.fillWidth: true }
            BusyIndicator { running: Backend.busy; visible: running; implicitWidth: 28; implicitHeight: 28 }
            TextField {
                id: addField
                Layout.preferredWidth: 340
                implicitHeight: 34
                placeholderText: "owner/repo or GitHub URL"
                enabled: !Backend.busy
                onAccepted: win.installFromField()
            }
            Btn { text: "Add from GitHub"; tint: win.accent; enabled: !Backend.busy && addField.text.trim() !== ""; onClicked: win.installFromField() }
        }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: win.line }
    }

    footer: Rectangle {
        height: 52
        color: win.panel
        Rectangle { width: parent.width; height: 1; color: win.line }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 18
            CheckBox {
                text: "Start at login"
                checked: Backend.autostart()
                onToggled: Backend.setAutostart(checked)
            }
            CheckBox {
                text: "Show in app launcher"
                checked: Backend.launcher()
                onToggled: Backend.setLauncher(checked)
            }
            Label {
                Layout.fillWidth: true
                text: win.ctl.status
                color: win.ctl.statusOk ? win.dim : win.danger
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignRight
            }
            Btn { text: win.ctl.paused ? "Resume" : "Pause"; onClicked: win.ctl.paused = !win.ctl.paused }
            Btn { text: "Quit"; tint: win.danger; onClicked: Qt.quit() }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            clip: true
            cellWidth: Math.floor(width / Math.max(1, Math.floor(width / 250)))
            cellHeight: Math.round(cellWidth * 0.66) + 44
            model: win.ctl.wallpapers
            ScrollBar.vertical: ScrollBar {}

            delegate: Item {
                id: card
                required property var modelData
                readonly property bool isCurrent: modelData.id === win.ctl.currentId
                width: grid.cellWidth
                height: grid.cellHeight

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 7
                    radius: 8
                    color: win.panel
                    border.width: card.isCurrent ? 2 : 1
                    border.color: card.isCurrent ? win.accent : hover.hovered ? "#324354" : win.line

                    Rectangle {
                        id: art
                        anchors { left: parent.left; right: parent.right; top: parent.top; bottom: caption.top; margins: 2 }
                        radius: 6
                        clip: true
                        gradient: Gradient {
                            GradientStop { position: 0; color: "#152231" }
                            GradientStop { position: 1; color: "#0c1219" }
                        }
                        Label {
                            anchors.centerIn: parent
                            visible: thumb.status !== Image.Ready
                            text: card.modelData.title.charAt(0).toUpperCase()
                            font.pixelSize: 54
                            font.weight: Font.Light
                            color: "#2c3d50"
                        }
                        Image {
                            id: thumb
                            anchors.fill: parent
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            cache: false
                            source: Backend.thumbUrl(card.modelData.id)
                            Connections {
                                target: win.ctl
                                function onThumbRevisionChanged() {
                                    if (card.isCurrent) {
                                        thumb.source = ""
                                        thumb.source = Backend.thumbUrl(card.modelData.id)
                                    }
                                }
                            }
                        }
                    }

                    RowLayout {
                        id: caption
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 12; rightMargin: 10 }
                        height: 40
                        Label {
                            Layout.fillWidth: true
                            text: card.modelData.title
                            elide: Text.ElideRight
                            color: win.fg
                        }
                        Label {
                            text: card.modelData.kind === "git" ? "github" : card.modelData.kind
                            font.pixelSize: 11
                            color: win.dim
                        }
                    }

                    HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: win.ctl.apply(card.modelData.id) }
                }
            }
        }

        Rectangle { Layout.fillHeight: true; width: 1; color: win.line }

        // --- current wallpaper + its settings ---
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 330
            color: win.panel

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 12

                Label {
                    Layout.fillWidth: true
                    text: win.ctl.current ? win.ctl.current.title : "Web page"
                    font.pixelSize: 18
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    color: win.fg
                }
                Label {
                    Layout.fillWidth: true
                    text: win.ctl.current
                          ? (win.ctl.current.kind === "git" ? "from GitHub" : win.ctl.current.kind === "preset" ? "built-in preset" : "local folder")
                            + (win.ctl.current.stats ? "  ·  uses system stats" : "")
                          : win.ctl.currentId
                    color: win.dim
                    elide: Text.ElideMiddle
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: win.line }

                ScrollView {
                    id: scroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    contentWidth: availableWidth

                    ColumnLayout {
                        width: scroll.availableWidth
                        spacing: 18

                        Label {
                            visible: editors.count === 0
                            Layout.fillWidth: true
                            text: "This wallpaper has no settings."
                            color: win.dim
                            wrapMode: Text.WordWrap
                        }
                        Repeater {
                            id: editors
                            // Rebuilt only when the wallpaper changes or its settings are reset.
                            model: {
                                win.ctl.propsRevision
                                const props = win.ctl.current ? win.ctl.current.props : {}
                                return Object.keys(props).filter(k => props[k].value !== undefined
                                    && ["slider", "checkbox", "color", "dropdown", "textbox"].includes(props[k].type))
                            }
                            delegate: PropEditor {
                                required property string modelData
                                Layout.fillWidth: true
                                Layout.rightMargin: 12
                                name: modelData
                                def: win.ctl.current.props[modelData]
                                initial: win.ctl.propValues[modelData]
                                onEdited: value => win.ctl.setProp(name, value)
                            }
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Btn {
                        text: "Reset"
                        visible: editors.count > 0
                        onClicked: win.ctl.resetProps()
                    }
                    Btn {
                        text: "Update"
                        visible: win.ctl.current && win.ctl.current.kind === "git"
                        enabled: !Backend.busy
                        onClicked: { win.ctl.say(true, "Updating…"); Backend.update(win.ctl.currentId) }
                    }
                    Item { Layout.fillWidth: true }
                    Btn {
                        text: "Remove"
                        tint: win.danger
                        visible: win.ctl.current && win.ctl.current.kind !== "preset"
                        enabled: !Backend.busy
                        onClicked: Backend.remove(win.ctl.currentId)
                    }
                }
            }
        }
    }
}
