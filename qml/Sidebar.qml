import QtQuick
import Pingy

Rectangle {
    id: side
    required property var snap
    required property var engine
    signal addRequested()

    color: Theme.panel

    Rectangle {
        id: header
        width: parent.width
        height: 56
        color: Theme.header
        Text {
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            text: "TARGETS"
            color: Theme.text
            font.pixelSize: 15
            font.bold: true
            font.letterSpacing: 2
        }
        Text {
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            text: side.snap.targets.length
            color: Theme.textDim
            font.pixelSize: 13
        }
        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 2
            color: Theme.accent
        }
    }

    ListView {
        id: list
        anchors.top: header.bottom
        anchors.bottom: footer.top
        width: parent.width
        clip: true
        spacing: 2
        boundsBehavior: Flickable.StopAtBounds
        model: side.snap.targets
        delegate: TargetCard {
            width: list.width
            engine: side.engine
        }
        SmoothWheel { flick: list }
    }

    Item {
        id: footer
        anchors.bottom: parent.bottom
        width: parent.width
        height: 52
        FlatButton {
            anchors.fill: parent
            anchors.margins: 8
            primary: true
            text: "+  Add target"
            onClicked: side.addRequested()
        }
    }
}
