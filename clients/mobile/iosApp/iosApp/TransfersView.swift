import SwiftUI

/// Everything on its way between this phone and the server: picked files, then what syncs by itself.
struct TransfersView: View {
    let model: SessionModel
    let client: CoreClient
    @State private var sources: [UploadSource] = []
    @State private var current: CurrentUpload?
    @State private var offline: OfflineStatus?

    private var kept: UInt64 { offline?.kept ?? 0 }

    var body: some View {
        List {
            if !model.uploads.isEmpty {
                Section {
                    ForEach(model.uploads, id: \.key) { upload in
                        UploadRow(model: model, upload: upload)
                    }
                } header: {
                    HStack {
                        Text("Uploads")
                        Spacer()
                        if model.uploads.count(where: { $0.state == .paused }) > 1 {
                            Button("Continue All") { model.continueUploads() }
                                .font(.subheadline)
                                .textCase(nil)
                        }
                    }
                }
            }
            if !sources.isEmpty {
                Section("Auto-Upload") {
                    ForEach(sources) { source in
                        NavigationLink {
                            SyncSettingsView(model: model, client: client)
                        } label: {
                            sourceLabel(source, current: current?.sourceId == source.id ? current : nil)
                        }
                    }
                }
            }
            if let offline, offline.kept > 0 {
                Section("Offline Copies") {
                    NavigationLink {
                        SyncSettingsView(model: model, client: client)
                    } label: {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(offline.kept == 1 ? "1 item on this iPhone" : "\(offline.kept) items on this iPhone")
                            Text(detail(of: offline))
                                .font(.caption)
                                .foregroundStyle(offline.problems.isEmpty ? Color.secondary : Color.red)
                        }
                    }
                }
            }
        }
        .overlay {
            if model.uploads.isEmpty && sources.isEmpty && kept == 0 {
                ContentUnavailableView(
                    "No Transfers",
                    systemImage: "arrow.up.arrow.down",
                    description: Text("Files you upload and folders that sync show up here.")
                )
            }
        }
        .navigationTitle("Transfers")
        .task(id: ClientHolder.shared.changes.generation) { await load() }
    }

    private func load() async {
        let auto = try? ClientHolder.shared.active?.autoUpload()
        sources = (try? auto?.sources()) ?? []
        current = auto?.current()
        offline = try? await client.offlineStatus()
    }

    private func sourceLabel(_ source: UploadSource, current: CurrentUpload?) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(source.label).lineLimit(1)
            if let current {
                Text("Uploading \(current.name)\(percent(done: current.done, total: current.total))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                Text(status(of: source))
                    .font(.caption)
                    .foregroundStyle(source.problem != nil || source.lastError != nil ? Color.red : Color.secondary)
            }
        }
    }

    private func status(of source: UploadSource) -> String {
        if let problem = source.problem {
            return problem
        }
        if let failure = source.lastError {
            return "\(source.pending) to go. \(failure)"
        }
        if source.pending > 0 {
            return "\(source.pending) to go"
        }
        return source.lastSyncedAt == nil ? "Waiting to sync" : "Up to date"
    }

    private func detail(of status: OfflineStatus) -> String {
        if !status.problems.isEmpty {
            return "\(status.problems.count) couldn’t sync"
        }
        if status.unreachable {
            return "Waiting for the connection"
        }
        return status.pending > 0 ? "\(status.pending) updating" : "Up to date"
    }
}

/// " · 40%", or nothing if there is nothing to say yet.
private func percent(done: UInt64, total: UInt64) -> String {
    guard total > 0, done > 0 else { return "" }
    return " · \(Int(Double(done) / Double(total) * 100))%"
}

/// A picked file on its way up.
private struct UploadRow: View {
    let model: SessionModel
    let upload: QueuedUpload

    private var status: String {
        let shown = percent(done: upload.done, total: upload.total)
        return switch upload.state {
        case .waiting: "Waiting to upload to \(upload.folderName)\(shown)"
        case .preparing: "Preparing to upload to \(upload.folderName)"
        case .uploading: "Uploading to \(upload.folderName)\(shown)"
        case .offline: "Waiting for the connection to upload to \(upload.folderName)\(shown)"
        case .paused: "Paused upload to \(upload.folderName)\(shown)"
        case .failed: upload.problem ?? "This file couldn’t be uploaded"
        }
    }

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: "arrow.up.doc")
                .frame(width: 26, height: 26)
            VStack(alignment: .leading, spacing: 2) {
                Text(upload.name).lineLimit(1)
                Text(status)
                    .font(.caption)
                    .foregroundStyle(upload.state == .failed ? Color.red : Color.secondary)
            }
            Spacer()
            if model.canRetry(upload) {
                Button(upload.state == .paused ? "Continue" : "Retry") { model.retryUpload(upload: upload) }
                    .buttonStyle(.borderless)
            }
            Button("Cancel Upload", systemImage: "xmark.circle.fill") {
                model.cancelUpload(upload: upload)
            }
            .labelStyle(.iconOnly)
            .foregroundStyle(.secondary)
            .buttonStyle(.borderless)
        }
    }
}

/// The uploads as one row in every folder, so that a failure can't go unseen. Opens Transfers.
struct UploadsSummaryRow: View {
    let uploads: [QueuedUpload]

    private var failed: Int { uploads.count { $0.state == .failed } }
    private var paused: [QueuedUpload] { uploads.filter { $0.state == .paused } }
    private var going: [QueuedUpload] { uploads.filter { $0.state != .failed && $0.state != .paused } }

    private var title: String {
        if going.isEmpty, failed == 0 {
            return paused.count == 1 ? "Upload of \(paused[0].name) paused" : "\(paused.count) uploads paused"
        }
        if going.isEmpty {
            return failed == 1 ? "1 file couldn’t be uploaded" : "\(failed) files couldn’t be uploaded"
        }
        return going.count == 1 ? "Uploading \(going[0].name)" : "Uploading \(going.count) files"
    }

    private var detail: String {
        var parts: [String] = []
        if !going.isEmpty, failed > 0 {
            parts.append("\(failed) failed")
        }
        if !paused.isEmpty, !going.isEmpty || failed > 0 {
            parts.append("\(paused.count) paused")
        }
        if going.contains(where: { $0.state == .offline }) {
            parts.append("Waiting for the connection")
        } else {
            let shown = going.isEmpty && failed == 0 ? paused : going
            let total = shown.reduce(0) { $0 + $1.total }
            let done = shown.reduce(0) { $0 + $1.done }
            if total > 0 {
                parts.append("\(Int(Double(done) / Double(total) * 100))%")
            }
        }
        return parts.joined(separator: " · ")
    }

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: "arrow.up.doc")
                .frame(width: 26, height: 26)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).lineLimit(1)
                if !detail.isEmpty {
                    Text(detail)
                        .font(.caption)
                        .foregroundStyle(failed > 0 ? Color.red : Color.secondary)
                }
            }
            Spacer()
            Image(systemName: "chevron.right")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.tertiary)
        }
        .contentShape(Rectangle())
    }
}
