import QtQuick
import Pingy

// Slide-in drawer over the right edge. Changes apply when a slider is released
// rather than on every step, so the workers are not reconfigured mid-drag.
Item {
    id: root
    required property var snap
    required property var engine
    property bool open: false

    visible: open || drawer.x < width

    Rectangle {
        anchors.fill: parent
        color: "#b3000000"
        opacity: root.open ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 160 } }
        MouseArea {
            anchors.fill: parent
            onClicked: root.open = false
        }
    }

    Rectangle {
        id: drawer
        width: 320
        height: parent.height
        x: root.open ? parent.width - width : parent.width
        Behavior on x { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
        color: Theme.panel
        border.color: Theme.border

        MouseArea { anchors.fill: parent }

        Column {
            anchors.fill: parent
            anchors.margins: 20
            spacing: 22

            Text {
                text: "SETTINGS"
                color: Theme.text
                font.pixelSize: 15
                font.bold: true
                font.letterSpacing: 2
            }

            LabeledSlider {
                id: interval
                width: parent.width
                label: "Ping interval"
                unit: "ms"
                from: 250; to: 5000; stepSize: 250
                value: root.snap.settings.intervalMs
                onMoved: root.apply()
            }
            LabeledSlider {
                id: timeout
                width: parent.width
                label: "Timeout"
                unit: "ms"
                from: 100; to: 5000; stepSize: 100
                value: root.snap.settings.timeoutMs
                onMoved: root.apply()
            }
            LabeledSlider {
                id: ttl
                width: parent.width
                label: "TTL"
                from: 1; to: 255; stepSize: 1
                value: root.snap.settings.ttl
                onMoved: root.apply()
            }
            LabeledSlider {
                id: history
                width: parent.width
                label: "History"
                unit: "samples"
                from: 10; to: 600; stepSize: 10
                value: root.snap.settings.historyLength
                onMoved: root.apply()
            }

            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Pingy " + root.engine.version + "\nSettings and targets are saved automatically."
                color: Theme.textDim
                font.pixelSize: 11
            }

            FlatButton {
                text: "Close"
                onClicked: root.open = false
            }
        }
    }

    function apply() {
        engine.applySettings(Math.round(timeout.value), Math.round(interval.value),
                             Math.round(ttl.value), Math.round(history.value))
    }
}
