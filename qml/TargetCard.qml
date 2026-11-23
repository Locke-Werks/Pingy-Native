import QtQuick
import Pingy

Rectangle {
    id: card
    required property var modelData
    required property var engine

    height: 76
    color: hover.hovered ? Theme.surfaceHover : Theme.surface
    opacity: modelData.active ? 1 : 0.45
    Behavior on opacity { NumberAnimation { duration: 120 } }

    HoverHandler { id: hover }

    Rectangle {
        width: 4
        height: parent.height
        color: Theme.lineColor(card.modelData.color)
    }

    Column {
        x: 16
        y: 10
        width: parent.width - 104
        spacing: 2
        Text {
            width: parent.width
            text: card.modelData.name
            color: Theme.text
            font.pixelSize: 14
            font.bold: true
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            text: card.modelData.host + (card.modelData.ttl !== null ? "  ·  ttl " + card.modelData.ttl : "")
            color: Theme.textDim
            font.pixelSize: 11
            font.family: Theme.mono
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            visible: card.modelData.error === null
            text: "avg " + Theme.fmtMs(card.modelData.average)
                  + "  " + Theme.fmtMs(card.modelData.min) + "-" + Theme.fmtMs(card.modelData.max)
                  + "  " + card.modelData.loss.toFixed(0) + "% loss"
            color: Theme.textSecondary
            font.pixelSize: 11
            font.family: Theme.mono
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            visible: card.modelData.error !== null
            text: card.modelData.error || ""
            color: Theme.accentSoft
            font.pixelSize: 11
            elide: Text.ElideRight
        }
    }

    Text {
        anchors.right: parent.right
        anchors.rightMargin: 16
        y: 12
        text: card.modelData.active ? Theme.fmtMs(card.modelData.current) : "off"
        color: card.modelData.current === null && card.modelData.sent > 0 && card.modelData.active
               ? Theme.accent : Theme.latencyColor(card.modelData.current)
        font.pixelSize: 24
        font.bold: true
        font.family: Theme.mono
    }
    Text {
        anchors.right: parent.right
        anchors.rightMargin: 16
        y: 44
        visible: card.modelData.active
        text: card.modelData.current === null && card.modelData.sent > 0 ? "timeout" : "ms"
        color: Theme.textDim
        font.pixelSize: 11
    }

    // Click toggles the target; the cross on hover removes it.
    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: card.engine.setActive(card.modelData.id, !card.modelData.active)
    }

    Rectangle {
        visible: hover.hovered
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 6
        width: 20
        height: 20
        radius: 10
        color: xArea.containsMouse ? Theme.accent : "#262626"
        Text {
            anchors.centerIn: parent
            text: "×"
            color: Theme.text
            font.pixelSize: 14
        }
        MouseArea {
            id: xArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: card.engine.remove(card.modelData.id)
        }
    }
}
