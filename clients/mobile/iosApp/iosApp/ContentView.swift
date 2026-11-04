import SwiftUI

/// Server check, sign-in and folder browsing through the Rust client (`clients/mobile-core`).
struct ContentView: View {
    @State private var serverUrl = "http://localhost:8080"
    @State private var username = ""
    @State private var password = ""
    @State private var status: String?
    @State private var busy = false
    @State private var client: CoreClient?

    var body: some View {
        NavigationStack {
            Form {
                Section("Server") {
                    TextField("Server", text: $serverUrl)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                    Button("Check server") { run { try await check() } }
                }
                Section("Sign in") {
                    TextField("Username", text: $username)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    SecureField("Password", text: $password)
                    Button("Sign in") { run { try await signIn() } }
                }
                if let status {
                    Section { Text(status) }
                }
                if let client {
                    Section {
                        NavigationLink("Browse Home") {
                            FolderView(client: client, folderId: nil, title: "Home")
                        }
                    }
                }
            }
            .disabled(busy)
            .navigationTitle("Home Space")
        }
    }

    private func run(_ action: @escaping () async throws -> Void) {
        Task {
            busy = true
            defer { busy = false }
            do {
                try await action()
            } catch {
                status = "Failed: \(error)"
            }
        }
    }

    private func check() async throws {
        // Rust `async fn` is imported as Swift `async throws`.
        let health = try await CoreClient(baseUrl: serverUrl).health()
        status = "Server status: \(health.status)"
    }

    private func signIn() async throws {
        let client = CoreClient(baseUrl: serverUrl)
        let user = try await client.login(username: username, password: password)
        self.client = client
        status = "Signed in as \(user.username)"
    }
}

private struct FolderView: View {
    let client: CoreClient
    let folderId: String?
    let title: String
    @State private var entries: [Entry] = []
    @State private var error: String?

    var body: some View {
        List {
            if let error {
                Text(error).foregroundStyle(.red)
            }
            ForEach(entries, id: \.id) { entry in
                switch entry.kind {
                case .folder:
                    NavigationLink {
                        FolderView(client: client, folderId: entry.id, title: entry.name)
                    } label: {
                        Label(entry.name, systemImage: "folder")
                    }
                case .file:
                    VStack(alignment: .leading) {
                        Label(entry.name, systemImage: "doc")
                        Text("\(entry.sizeLabel) · \(entry.updatedLabel)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .navigationTitle(title)
        .task {
            do {
                entries = try await client.list(parentId: folderId)
            } catch {
                self.error = "\(error)"
            }
        }
    }
}

#Preview {
    ContentView()
}
