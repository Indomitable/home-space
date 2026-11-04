import SwiftUI

/// Folder browsing through the Rust client (`clients/mobile-core`).
struct BrowseView: View {
    let model: SessionModel
    let client: CoreClient
    let user: SignedInUser

    var body: some View {
        NavigationStack {
            FolderView(model: model, client: client, folderId: nil, title: "Home")
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Menu(user.username) {
                            Button("Sign out", role: .destructive) { model.signOut() }
                        }
                    }
                }
        }
    }
}

private struct FolderView: View {
    let model: SessionModel
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
                        FolderView(model: model, client: client, folderId: entry.id, title: entry.name)
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
            } catch CoreError.Unauthorized {
                model.sessionEnded()
            } catch {
                self.error = "\(error)"
            }
        }
    }
}
