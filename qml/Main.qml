import QtQuick
import QtQuick.Window
import Pingy

Window {
    id: win
    width: 1280
    height: 760
    minimumWidth: 640
    minimumHeight: 480
    visible: true
    color: Theme.background
    title: "Pingy"

    Engine { id: engine }

    // Stats and labels only; the graph pulls its own data every frame.
    Timer {
        interval: 200
        running: true
        repeat: true
        onTriggered: engine.poll()
    }

    // --snapshot <file.png>: render for a while, save the window, quit. Used for
    // README screenshots and for checking a build on a machine nobody watches.
    readonly property string snapshotPath: {
        const a = Qt.application.arguments
        const i = a.indexOf("--snapshot")
        return i >= 0 && i + 1 < a.length ? a[i + 1] : ""
    }
    Timer {
        interval: 20000
        running: win.snapshotPath.length > 0
        onTriggered: win.contentItem.grabToImage(r => { r.saveToFile(win.snapshotPath); Qt.quit() })
    }

    readonly property var snap: JSON.parse(engine.snapshot)
    readonly property bool wide: width / Math.max(1, height) >= Theme.aspectThreshold

    GraphPanel {
        id: graph
        snap: win.snap
        engine: engine
        x: win.wide ? sidebar.width : 0
        y: 0
        width: win.wide ? win.width - sidebar.width : win.width
        height: win.wide ? win.height : win.height * 0.58
        onSettingsRequested: settings.open = !settings.open
    }

    Sidebar {
        id: sidebar
        snap: win.snap
        engine: engine
        x: 0
        y: win.wide ? 0 : graph.height
        width: win.wide ? Theme.sidebarWidth : win.width
        height: win.wide ? win.height : win.height - graph.height
        onAddRequested: addDialog.show()
    }

    SettingsPanel {
        id: settings
        snap: win.snap
        engine: engine
        anchors.fill: parent
    }

    AddDialog {
        id: addDialog
        engine: engine
        anchors.fill: parent
    }
}
