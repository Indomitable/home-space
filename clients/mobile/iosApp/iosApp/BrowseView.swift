import QuickLook
import SwiftUI

/// The height of a row, measured from the Files app on an iPhone. The Android client uses the same.
private let rowHeight: CGFloat = 64

/// The look of the app's file lists, taken from the Files app: end to end, with tall rows.
extension View {
    func fileList() -> some View {
        listStyle(.plain).environment(\.defaultMinListRowHeight, rowHeight)
    }

    /// Little vertical padding, so that two lines of text still fit the row height.
    func fileRow() -> some View {
        listRowInsets(EdgeInsets(top: 6, leading: 20, bottom: 6, trailing: 20))
    }
}

/// Folder browsing through the Rust client (`clients/mobile-core`).
struct BrowseView: View {
    let model: SessionModel
    let client: CoreClient
    let user: SignedInUser

    var body: some View {
        NavigationStack {
            FolderView(model: model, client: client, user: user, folderId: nil, title: "Home")
        }
        .background(
            QuickLookPresenter(
                url: Binding(get: { model.preview }, set: { model.preview = $0 }),
                onChange: { model.previewEnded() }
            )
        )
        .alert(
            "Remove “\(model.removalWarning?.entry.name ?? "")” from this iPhone?",
            isPresented: Binding(
                get: { model.removalWarning != nil },
                set: { if !$0 { model.removalWarning = nil } }
            ),
            presenting: model.removalWarning
        ) { warning in
            Button("Remove", role: .destructive) {
                model.removeOfflineCopy(client: client, entry: warning.entry, confirmed: true)
            }
            Button("Cancel", role: .cancel) {}
        } message: { warning in
            Text(unsentEditsMessage(warning.names))
        }
        .alert(
            "Sign out?",
            isPresented: Binding(
                get: { model.signOutWarning != nil },
                set: { if !$0 { model.signOutWarning = nil } }
            ),
            presenting: model.signOutWarning
        ) { _ in
            Button("Sign Out", role: .destructive) { model.signOut() }
            Button("Cancel", role: .cancel) {}
        } message: { names in
            Text(unsentEditsMessage(names))
        }
        .sheet(
            item: Binding(
                get: { model.pendingExport },
                set: { if $0 == nil { model.exportFinished() } }
            )
        ) { export in
            ExportPicker(url: export.url) { model.exportFinished() }
                .ignoresSafeArea()
        }
        .alert(
            "Something went wrong",
            isPresented: Binding(
                get: { model.notice != nil },
                set: { if !$0 { model.notice = nil } }
            ),
            actions: { Button("OK") {} },
            message: { Text(model.notice ?? "") }
        )
    }
}

private func unsentEditsMessage(_ names: [String]) -> String {
    let shown = names.prefix(3).joined(separator: ", ")
    let more = names.count > 3 ? " and \(names.count - 3) more" : ""
    return "Edits to \(shown)\(more) aren't on the server yet and will be lost."
}

/// Shows a file in the system's Quick Look, whose markup tools save into the file. The sync then
/// sends the change to the server. Presented by UIKit, because SwiftUI's `quickLookPreview` offers
/// no way to allow editing.
private struct QuickLookPresenter: UIViewControllerRepresentable {
    @Binding var url: URL?
    let onChange: () -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIViewController(context: Context) -> UIViewController { UIViewController() }

    func updateUIViewController(_ host: UIViewController, context: Context) {
        let coordinator = context.coordinator
        coordinator.onDismiss = {
            url = nil
            onChange()
        }
        coordinator.onChange = onChange
        guard let url else {
            coordinator.preview?.dismiss(animated: true)
            coordinator.preview = nil
            return
        }
        guard coordinator.url != url || coordinator.preview == nil else { return }
        coordinator.url = url
        let preview = QLPreviewController()
        preview.dataSource = coordinator
        preview.delegate = coordinator
        coordinator.preview = preview
        // Not while SwiftUI is updating the view.
        DispatchQueue.main.async { host.present(preview, animated: true) }
    }

    final class Coordinator: NSObject, QLPreviewControllerDataSource, QLPreviewControllerDelegate {
        var url: URL?
        var preview: QLPreviewController?
        var onDismiss: () -> Void = {}
        var onChange: () -> Void = {}

        func numberOfPreviewItems(in controller: QLPreviewController) -> Int { url == nil ? 0 : 1 }

        func previewController(_ controller: QLPreviewController, previewItemAt index: Int) -> QLPreviewItem {
            (url ?? URL(fileURLWithPath: "/")) as NSURL
        }

        func previewController(
            _ controller: QLPreviewController,
            editingModeFor previewItem: QLPreviewItem
        ) -> QLPreviewItemEditingMode {
            .updateContents
        }

        func previewController(_ controller: QLPreviewController, didUpdateContentsOf previewItem: QLPreviewItem) {
            onChange()
        }

        func previewControllerDidDismiss(_ controller: QLPreviewController) {
            preview = nil
            url = nil
            onDismiss()
        }
    }
}

/// The screens the header's menu leads to.
private enum AccountPage: String, Identifiable {
    case profile = "Profile"
    case settings = "Settings"

    var id: Self { self }
}

/// The header's menu: who is signed in, and where to go from here.
private struct AccountMenu: View {
    let username: String
    let onOpen: (AccountPage) -> Void
    let onSignOut: () -> Void

    var body: some View {
        Menu("Menu", systemImage: "ellipsis.circle") {
            Section(username) {
                Button(AccountPage.profile.rawValue, systemImage: "person") { onOpen(.profile) }
                Button(AccountPage.settings.rawValue, systemImage: "gearshape") { onOpen(.settings) }
            }
            Section {
                Button(
                    "Sign out",
                    systemImage: "rectangle.portrait.and.arrow.right",
                    role: .destructive,
                    action: onSignOut
                )
            }
        }
    }
}

private struct FolderView: View {
    let model: SessionModel
    let client: CoreClient
    let user: SignedInUser
    let folderId: String?
    let title: String
    @State private var entries: [Entry] = []
    @State private var error: String?
    @State private var selection = Set<String>()
    @State private var editMode = EditMode.inactive
    @State private var action: FileAction?
    @State private var page: AccountPage?

    private var selecting: Bool { editMode.isEditing }
    private var chosen: [Entry] { entries.filter { selection.contains($0.id) } }

    var body: some View {
        List(selection: $selection) {
            if let error {
                Text(error).foregroundStyle(.red)
            }
            ForEach(entries, id: \.id) { entry in
                row(entry)
                    .contextMenu {
                        if !selecting {
                            rowActions(entry)
                        }
                    }
                    .fileRow()
                    .listRowBackground(ProgressBackground(model: model, id: entry.id))
            }
        }
        .fileList()
        .navigationTitle(selecting ? "\(selection.count) selected" : title)
        .navigationBarBackButtonHidden(selecting)
        .environment(\.editMode, $editMode)
        .toolbar {
            if selecting {
                ToolbarItem(placement: .topBarLeading) {
                    if selection.count < entries.count {
                        Button("Select All") { selection = Set(entries.map(\.id)) }
                    } else {
                        Button("Deselect All") { selection = [] }
                    }
                }
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button(selecting ? "Done" : "Select") {
                    selection = []
                    editMode = selecting ? .inactive : .active
                }
                .disabled(entries.isEmpty)
            }
            if !selecting {
                ToolbarItem(placement: .topBarTrailing) {
                    AccountMenu(
                        username: user.username,
                        onOpen: { page = $0 },
                        onSignOut: { model.requestSignOut(client: client) }
                    )
                }
            }
            if selecting && !chosen.isEmpty {
                ToolbarItemGroup(placement: .bottomBar) { selectionActions }
            }
        }
        .toolbar(selecting && !chosen.isEmpty ? .visible : .hidden, for: .bottomBar)
        .navigationDestination(item: $page) { page in
            switch page {
            case .profile:
                Color.clear.navigationTitle(page.rawValue)
            case .settings:
                SettingsView(model: model, client: client)
            }
        }
        .sheet(item: $action) { current in
            FileDialog(action: current, model: model, client: client) {
                action = nil
                selection = []
                editMode = .inactive
                model.reload()
            }
        }
        .task(id: model.reloadKey) { await load() }
        // Updating and failed copies change while the folder is open, with nothing new on the server.
        .onChange(of: ClientHolder.shared.changes.generation) {
            Task {
                let marked = await client.refreshOffline(entries: entries)
                if marked.map(\.id) == entries.map(\.id) {
                    entries = marked
                }
            }
        }
    }

    @ViewBuilder
    private func row(_ entry: Entry) -> some View {
        switch entry.kind {
        case .folder:
            NavigationLink {
                FolderView(
                    model: model,
                    client: client,
                    user: user,
                    folderId: entry.id,
                    title: entry.name
                )
            } label: {
                rowLabel(entry)
            }
        case .file:
            if selecting {
                rowLabel(entry)
            } else {
                Button { model.openFile(client: client, entry: entry) } label: { rowLabel(entry) }
                    .buttonStyle(.plain)
            }
        }
    }

    private func rowLabel(_ entry: Entry) -> some View {
        HStack(spacing: 12) {
            EntryIconView(icon: entry.icon, state: entry.offlineState)
            VStack(alignment: .leading, spacing: 2) {
                Text(entry.name)
                if entry.kind == .file {
                    Text("\(entry.sizeLabel) · \(localDate(entry.updatedAt))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }

    /// What a long press on a row offers.
    @ViewBuilder
    private func rowActions(_ entry: Entry) -> some View {
        let downloading = model.downloading.contains(entry.id)
        if downloading {
            Button("Cancel download", systemImage: "xmark.circle") {
                model.cancelDownload(client: client, entry: entry)
            }
        }
        if entry.kind == .file && !downloading {
            Button("Save to…", systemImage: "square.and.arrow.down") {
                model.saveFile(client: client, entry: entry)
            }
        }
        Button("Details", systemImage: "info.circle") { action = .details(entry) }
        Button("Rename", systemImage: "pencil") { action = .rename(entry) }
        Button("Move", systemImage: "folder") { action = .move([entry]) }
        if entry.offlineKept && !downloading {
            Button("Remove Offline Copy", systemImage: "arrow.down.circle.dotted") {
                model.removeOfflineCopy(client: client, entry: entry)
            }
        } else if entry.offlineState == nil && !downloading {
            Button("Make Available Offline", systemImage: "arrow.down.circle") {
                model.makeAvailableOffline(client: client, entry: entry)
            }
        }
        if entry.id != user.lostAndFoundId {
            Divider()
            Button("Delete", systemImage: "trash", role: .destructive) { action = .delete([entry]) }
        }
    }

    @ViewBuilder
    private var selectionActions: some View {
        if chosen.count == 1, let entry = chosen.first {
            Button("Rename") { action = .rename(entry) }
            Spacer()
            Button("Details") { action = .details(entry) }
            Spacer()
        }
        Button("Move") { action = .move(chosen) }
        if !chosen.contains(where: { $0.id == user.lostAndFoundId }) {
            Spacer()
            Button("Delete", role: .destructive) { action = .delete(chosen) }
        }
    }

    private func load() async {
        do {
            let listed = try await client.list(parentId: folderId)
            error = nil
            entries = listed
            // A file that is gone can't stay selected.
            selection.formIntersection(listed.map(\.id))
        } catch is CancellationError {
            // A newer load took over.
        } catch CoreError.Unauthorized {
            model.sessionEnded()
        } catch {
            self.error = fileMessage(for: error)
        }
    }
}

/// A row's background: it fills from the leading edge as the file downloads.
private struct ProgressBackground: View {
    let model: SessionModel
    let id: String

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .leading) {
                Color(.systemBackground)
                if let progress = model.progress[id] {
                    Color("FolderAccent")
                        .opacity(0.25)
                        .frame(width: geometry.size.width * progress)
                }
            }
        }
    }
}

/// The same icons and colours as the web client: only folders are coloured.
struct EntryIconView: View {
    let icon: EntryIcon
    let state: OfflineState?

    private var asset: String {
        switch icon {
        case .folder: "node-folder"
        case .image: "node-image"
        case .document: "node-description"
        case .other: "node-draft"
        }
    }

    var body: some View {
        // The box and badge sizes are the desktop client's.
        ZStack(alignment: .leading) {
            Image(decorative: asset)
                .renderingMode(.template)
                .resizable()
                .frame(width: 20, height: 20)
                .foregroundStyle(icon == .folder ? Color("FolderAccent") : Color.primary)
            if let state {
                badge(for: state)
                    .frame(width: 12, height: 12)
                    .padding(1)
                    .background(Color(.systemBackground), in: Circle())
                    .frame(width: 26, height: 26, alignment: .bottomTrailing)
            }
        }
        .frame(width: 26, height: 26)
    }

    @ViewBuilder
    private func badge(for state: OfflineState) -> some View {
        switch state {
        case .synced:
            Image(decorative: "sync-check")
                .renderingMode(.template)
                .resizable()
                .foregroundStyle(Color("SyncedBadge"))
                .accessibilityLabel("Available offline")
        case .updating:
            Image(systemName: "arrow.triangle.2.circlepath")
                .resizable()
                .scaledToFit()
                .foregroundStyle(Color("FolderAccent"))
                .accessibilityLabel("Syncing")
        case .failed:
            Image(systemName: "exclamationmark.circle.fill")
                .resizable()
                .foregroundStyle(.red)
                .accessibilityLabel("Sync failed")
        }
    }
}
