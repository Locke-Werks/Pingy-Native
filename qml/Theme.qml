pragma Singleton
import QtQuick

QtObject {
    readonly property color background: "#0a0a0a"
    readonly property color panel: "#121212"
    readonly property color graphBg: "#111111"
    readonly property color surface: "#1f1f1f"
    readonly property color surfaceHover: "#292929"
    readonly property color border: "#333333"
    readonly property color header: "#0f0f0f"

    readonly property color accent: "#ff1744"
    readonly property color accentSoft: "#ff5252"
    readonly property color accentDim: "#991a1a"

    readonly property color text: "#ffffff"
    readonly property color textSecondary: "#b3b3b3"
    readonly property color textDim: "#666666"

    readonly property color good: "#4de666"
    readonly property color medium: "#ffcc33"
    readonly property color bad: accent

    // Same order as the graph traces in latency_graph.cpp.
    readonly property var lineColors: ["#ff1744", "#ff7373", "#ff99cc", "#e6e6e6",
                                       "#ffb34d", "#cc3380", "#ffd9d9", "#b31a1a"]

    readonly property int sidebarWidth: 320
    readonly property real aspectThreshold: 1.2

    readonly property string mono: Qt.platform.os === "windows" ? "Consolas"
                                 : Qt.platform.os === "osx" ? "Menlo" : "monospace"

    function lineColor(i) { return lineColors[i % lineColors.length] }

    function latencyColor(ms) {
        if (ms === null || ms === undefined) return textDim
        if (ms < 50) return good
        if (ms < 150) return medium
        return bad
    }

    function fmtMs(ms) {
        if (ms === null || ms === undefined) return "--"
        return ms < 10 ? ms.toFixed(1) : Math.round(ms).toString()
    }
}
