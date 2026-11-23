import QtQuick
import Pingy

Rectangle {
    id: btn
    property string text
    property bool primary: false
    property bool enabledLook: true
    signal clicked()

    implicitWidth: label.implicitWidth + 28
    implicitHeight: 36
    radius: 4
    color: primary ? (area.containsMouse ? Theme.accentSoft : Theme.accent)
                   : (area.containsMouse ? Theme.surfaceHover : Theme.surface)
    border.color: primary ? "transparent" : Theme.border
    opacity: enabledLook ? 1 : 0.4
    Behavior on color { ColorAnimation { duration: 90 } }

    Text {
        id: label
        anchors.centerIn: parent
        text: btn.text
        color: Theme.text
        font.pixelSize: 13
        font.bold: btn.primary
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: if (btn.enabledLook) btn.clicked()
    }
}
