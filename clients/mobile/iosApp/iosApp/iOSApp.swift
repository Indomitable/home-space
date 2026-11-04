import SwiftUI

@main
struct iOSApp: App {
    var body: some Scene {
        WindowGroup {
            // Spike: the Kotlin (KMP) and Rust (UniFFI) clients side by side.
            TabView {
                ServerCheckView()
                    .tabItem { Label("Kotlin", systemImage: "k.circle") }
                RustCoreView()
                    .tabItem { Label("Rust", systemImage: "r.circle") }
            }
        }
    }
}
