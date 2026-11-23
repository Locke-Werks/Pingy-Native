import QtQuick
import Pingy

Rectangle {
    id: panel
    required property var snap
    required property var engine
    signal settingsRequested()

    color: Theme.graphBg
    clip: true

    readonly property int stripW: 72
    readonly property int topPad: 64
    readonly property int bottomPad: 30
    readonly property int legendH: 34

    LatencyGraph {
        id: graph
        x: panel.stripW
        y: panel.topPad
        width: panel.width - panel.stripW
        height: panel.height - panel.topPad - panel.bottomPad - panel.legendH
    }

    // Left strip with the latency scale. Labels follow the graph's gliding
    // scale, so they move with the curves instead of jumping ahead of them.
    Rectangle {
        width: panel.stripW
        height: parent.height
        color: "#80000000"
        Repeater {
            model: 5
            Text {
                required property int index
                readonly property real frac: index / 4
                x: 8
                y: graph.y + graph.height * (1 - frac) - height / 2
                text: Math.round(graph.scaleTop * frac) + " ms"
                color: Theme.textSecondary
                font.pixelSize: 12
                font.family: Theme.mono
            }
        }
    }

    Column {
        x: panel.stripW + 12
        y: 10
        spacing: 2
        Text {
            text: "LATENCY"
            color: Theme.accent
            font.pixelSize: 18
            font.bold: true
            font.letterSpacing: 2
        }
        Text {
            text: panel.snap.running ? "RUNNING" : "PAUSED"
            color: panel.snap.running ? Theme.good : Theme.textDim
            font.pixelSize: 12
            font.letterSpacing: 1
        }
    }

    Row {
        anchors.right: parent.right
        anchors.rightMargin: 12
        y: 14
        spacing: 8
        FlatButton {
            text: panel.snap.running ? "Pause" : "Resume"
            onClicked: panel.engine.setRunning(!panel.snap.running)
        }
        FlatButton {
            text: "Clear"
            onClicked: panel.engine.clearHistory()
        }
        FlatButton {
            text: "Settings"
            onClicked: panel.settingsRequested()
        }
    }

    // Time axis: the right edge is "now" minus the render delay, so the labels
    // count back from the right across the visible window.
    Repeater {
        model: 7
        Text {
            required property int index
            readonly property real frac: index / 6
            x: Math.min(graph.x + graph.width * frac - width / 2, panel.width - width - 6)
            y: graph.y + graph.height + 6
            text: panel.fmtAgo(graph.windowMs * (1 - frac))
            color: Theme.textDim
            font.pixelSize: 11
        }
    }

    function fmtAgo(ms) {
        if (ms < 500) return "now"
        const s = ms / 1000
        if (s < 60) return Math.round(s) + "s"
        const m = Math.floor(s / 60), r = Math.round(s % 60)
        return r > 0 ? m + "m" + r + "s" : m + "m"
    }

    // Legend
    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: panel.legendH
        color: "#a6000000"
        Flow {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            anchors.topMargin: 10
            spacing: 16
            Repeater {
                model: panel.snap.targets
                Row {
                    required property var modelData
                    spacing: 6
                    opacity: modelData.active ? 1 : 0.35
                    Rectangle {
                        width: 14
                        height: 4
                        anchors.verticalCenter: parent.verticalCenter
                        color: Theme.lineColor(modelData.color)
                    }
                    Text {
                        text: modelData.name
                        color: Theme.textSecondary
                        font.pixelSize: 12
                    }
                }
            }
        }
    }
}
