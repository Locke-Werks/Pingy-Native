import QtQuick
import QtQuick.Controls.Basic
import Pingy

Item {
    id: root
    required property var engine
    property bool shown: false
    property string error: ""

    visible: shown
    focus: shown

    function show() {
        host.text = ""
        name.text = ""
        error = ""
        shown = true
        host.forceActiveFocus()
    }

    function submit() {
        if (host.text.trim().length === 0) {
            error = "Enter a hostname or IP address."
            return
        }
        if (!engine.add(host.text, name.text)) {
            error = "That host is already in the list."
            return
        }
        shown = false
    }

    Keys.onEscapePressed: shown = false

    Rectangle {
        anchors.fill: parent
        color: "#b3000000"
        MouseArea {
            anchors.fill: parent
            onClicked: root.shown = false
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width: 400
        height: col.implicitHeight + 40
        radius: 6
        color: Theme.panel
        border.color: Theme.border

        MouseArea { anchors.fill: parent }

        Column {
            id: col
            anchors.fill: parent
            anchors.margins: 20
            spacing: 12

            Text {
                text: "ADD TARGET"
                color: Theme.text
                font.pixelSize: 15
                font.bold: true
                font.letterSpacing: 2
            }

            Text { text: "Host"; color: Theme.textSecondary; font.pixelSize: 12 }
            TextField {
                id: host
                width: parent.width
                placeholderText: "example.com or 192.0.2.1"
                color: Theme.text
                placeholderTextColor: Theme.textDim
                background: Rectangle { color: Theme.surface; radius: 4; border.color: host.activeFocus ? Theme.accent : Theme.border }
                onAccepted: root.submit()
                Keys.onEscapePressed: root.shown = false
            }

            Text { text: "Display name (optional)"; color: Theme.textSecondary; font.pixelSize: 12 }
            TextField {
                id: name
                width: parent.width
                color: Theme.text
                background: Rectangle { color: Theme.surface; radius: 4; border.color: name.activeFocus ? Theme.accent : Theme.border }
                onAccepted: root.submit()
                Keys.onEscapePressed: root.shown = false
            }

            Text {
                visible: root.error.length > 0
                text: root.error
                color: Theme.accentSoft
                font.pixelSize: 12
            }

            Row {
                anchors.right: parent.right
                spacing: 8
                FlatButton { text: "Cancel"; onClicked: root.shown = false }
                FlatButton { text: "Add"; primary: true; onClicked: root.submit() }
            }
        }
    }
}
