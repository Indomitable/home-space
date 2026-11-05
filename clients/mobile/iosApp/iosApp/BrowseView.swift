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
                        Label { Text(entry.name) } icon: { EntryIconView(icon: entry.icon) }
                    }
                case .file:
                    VStack(alignment: .leading) {
                        Label { Text(entry.name) } icon: { EntryIconView(icon: entry.icon) }
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

/// The same icons and colours as the web client: only folders are coloured.
private struct EntryIconView: View {
    let icon: EntryIcon

    private var asset: String {
        switch icon {
        case .folder: "node-folder"
        case .image: "node-image"
        case .document: "node-description"
        case .other: "node-draft"
        }
    }

    var body: some View {
        Image(decorative: asset)
            .renderingMode(.template)
            .resizable()
            .frame(width: 20, height: 20)
            .foregroundStyle(icon == .folder ? Color("FolderAccent") : Color.primary)
    }
}
