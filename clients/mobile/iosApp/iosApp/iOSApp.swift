import SwiftUI

@main
struct iOSApp: App {
    @State private var model = SessionModel()

    var body: some Scene {
        WindowGroup {
            RootView(model: model)
        }
    }
}

/// Picks the screen for the session, which the model tries to resume once at launch.
private struct RootView: View {
    let model: SessionModel

    var body: some View {
        Group {
            switch model.session {
            case .resuming(let server, let error):
                ResumingView(model: model, server: server, error: error)
            case .signedOut:
                LoginView(model: model)
            case .signedIn(let client, let user):
                BrowseView(model: model, client: client, user: user)
            }
        }
        .task { model.start() }
    }
}
