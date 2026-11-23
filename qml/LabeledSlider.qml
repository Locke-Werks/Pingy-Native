import QtQuick
import QtQuick.Controls.Basic
import Pingy

Column {
    id: root
    property string label
    property string unit
    property alias from: slider.from
    property alias to: slider.to
    property alias stepSize: slider.stepSize
    property alias value: slider.value
    signal moved()

    spacing: 4

    Item {
        width: parent.width
        height: 18
        Text {
            text: root.label
            color: Theme.textSecondary
            font.pixelSize: 12
        }
        Text {
            anchors.right: parent.right
            text: Math.round(slider.value) + (root.unit ? " " + root.unit : "")
            color: Theme.text
            font.pixelSize: 12
            font.family: Theme.mono
        }
    }

    Slider {
        id: slider
        width: parent.width
        snapMode: Slider.SnapAlways
        onPressedChanged: if (!pressed) root.moved()
        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            width: slider.availableWidth
            height: 6
            radius: 3
            color: Theme.surface
            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: 3
                color: Theme.accentDim
            }
        }
        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            width: 18
            height: 18
            radius: 9
            color: slider.pressed ? Theme.accentSoft : Theme.accent
        }
    }
}
