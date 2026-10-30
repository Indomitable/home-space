import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import desktop

ApplicationWindow {
    id: window

    readonly property int formWidth: 360
    readonly property int formSpacing: 12
    readonly property int formPadding: 24

    visible: true
    title: "Home Space"
    width: 640
    height: 480
    minimumWidth: 480
    minimumHeight: 360

    function submit() {
        if (!Login.busy)
            Login.signIn(password.text)
    }

    Pane {
        anchors.centerIn: parent
        width: window.formWidth
        padding: window.formPadding

        ColumnLayout {
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: window.formSpacing

            Label {
                text: "Home Space"
                font.pixelSize: 28
            }

            // The form, until signing in succeeds.
            ColumnLayout {
                visible: Login.signedInAs === ""
                Layout.fillWidth: true
                spacing: window.formSpacing

                TextField {
                    id: server
                    Layout.fillWidth: true
                    placeholderText: "Server"
                    text: Login.server
                    onTextEdited: Login.server = text
                }
                TextField {
                    id: username
                    Layout.fillWidth: true
                    placeholderText: "Username"
                    text: Login.username
                    focus: true
                    onTextEdited: Login.username = text
                    onAccepted: window.submit()
                }
                TextField {
                    id: password
                    Layout.fillWidth: true
                    placeholderText: "Password"
                    echoMode: TextInput.Password
                    onAccepted: window.submit()
                }
                Button {
                    text: Login.busy ? "Signing in…" : "Sign in"
                    enabled: !Login.busy
                    onClicked: window.submit()
                }
                Label {
                    Layout.fillWidth: true
                    visible: Login.error !== ""
                    text: Login.error
                    color: "#d0393e"
                    wrapMode: Text.Wrap
                }
            }

            // Who is signed in, once signing in succeeds.
            ColumnLayout {
                visible: Login.signedInAs !== ""
                Layout.fillWidth: true
                spacing: window.formSpacing

                Label {
                    text: "Signed in as " + Login.signedInAs
                }
                Button {
                    text: "Sign out"
                    onClicked: {
                        password.clear()
                        Login.signOut()
                    }
                }
            }
        }
    }
}
