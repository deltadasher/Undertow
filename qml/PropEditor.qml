// Editor for one wallpaper setting. `def` is the manifest entry:
// { "type": "slider|checkbox|color|dropdown|textbox", "text", "value", "min", "max", "step", "items" }
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property string name
    required property var def
    required property var initial
    signal edited(var value)

    readonly property string kind: def.type || ""
    spacing: 6

    RowLayout {
        Layout.fillWidth: true
        Label {
            Layout.fillWidth: true
            text: root.def.text || root.name
            wrapMode: Text.WordWrap
            color: "#d7e0ea"
        }
        Label {
            visible: root.kind === "slider"
            text: Math.round(slider.value * 1000) / 1000
            color: "#7d8b9a"
        }
        Switch {
            visible: root.kind === "checkbox"
            checked: root.initial === true
            onToggled: root.edited(checked)
        }
    }

    Slider {
        id: slider
        visible: root.kind === "slider"
        Layout.fillWidth: true
        from: root.def.min !== undefined ? root.def.min : 0
        to: root.def.max !== undefined ? root.def.max : 100
        stepSize: root.def.step !== undefined ? root.def.step : 0
        snapMode: Slider.SnapAlways
        value: root.kind === "slider" ? Number(root.initial) : 0
        onMoved: root.edited(Math.round(value * 10000) / 10000)
        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + (slider.availableHeight - height) / 2
            width: 18; height: 18; radius: 9
            color: slider.pressed ? "#ffffff" : "#d7e0ea"
            border.color: "#5fd7a7"
            border.width: 2
        }
    }

    RowLayout {
        visible: root.kind === "color"
        Layout.fillWidth: true
        Rectangle {
            id: swatch
            width: 36; height: 28; radius: 4
            color: root.kind === "color" && root.initial ? root.initial : "transparent"
            border.color: "#2a3644"
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: { colorDialog.selectedColor = swatch.color; colorDialog.open() }
            }
        }
        TextField {
            id: hex
            Layout.fillWidth: true
            text: root.kind === "color" && root.initial ? root.initial : ""
            font.family: "monospace"
            onEditingFinished: {
                if (/^#[0-9a-fA-F]{6}$/.test(text)) {
                    swatch.color = text
                    root.edited(text)
                }
            }
        }
    }

    ComboBox {
        visible: root.kind === "dropdown"
        Layout.fillWidth: true
        model: root.def.items || []
        currentIndex: root.kind === "dropdown" ? Number(root.initial) : -1
        onActivated: index => root.edited(index)
    }

    TextField {
        visible: root.kind === "textbox"
        Layout.fillWidth: true
        text: root.kind === "textbox" && root.initial !== undefined ? String(root.initial) : ""
        onEditingFinished: root.edited(text)
    }

    ColorDialog {
        id: colorDialog
        onAccepted: {
            const picked = String(selectedColor).slice(0, 7)
            swatch.color = picked
            hex.text = picked
            root.edited(picked)
        }
    }
}
