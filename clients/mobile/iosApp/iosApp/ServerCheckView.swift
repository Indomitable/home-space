import Shared
import SwiftUI

/// Placeholder first screen: checks a server through the shared Kotlin client.
struct ServerCheckView: View {
    /// The simulator shares the Mac's network, so localhost is the dev server.
    @State private var serverUrl = "http://localhost:8080"
    @State private var result: String?
    @State private var checking = false

    var body: some View {
        NavigationStack {
            Form {
                Section("Server") {
                    TextField("Server", text: $serverUrl)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                }
                Section {
                    Button(checking ? "Checking…" : "Check server") {
                        Task { await check() }
                    }
                    .disabled(checking)
                    if let result {
                        Text(result)
                    }
                }
            }
            .navigationTitle("Home Space")
        }
    }

    private func check() async {
        checking = true
        defer { checking = false }
        let client = HomeSpaceClient(baseUrl: serverUrl)
        defer { client.close() }
        do {
            // Kotlin `suspend` functions are imported as Swift `async throws`.
            let health = try await client.health()
            result = "Server status: \(health.status)"
        } catch {
            result = "Could not reach the server: \(error.localizedDescription)"
        }
    }
}

#Preview {
    ServerCheckView()
}
