import Foundation
import Observation
import UIKit

enum Session {
    /// Trying a saved session; `error` is set when that failed but the session may still work.
    case resuming(server: String, error: String?)
    case signedOut
    case signedIn(client: CoreClient, user: SignedInUser)
}

/// What the server said about how to sign in.
enum Probe {
    case checking
    case ready(SignInOptions)
    case failed(String)
}

/// Saves the refresh token as soon as Rust reports it: tokens are single use, so one that was
/// rotated but not saved would sign the user out on the next start.
private final class TokenSaver: SessionListener {
    private let server: String
    private let store: SessionStore

    init(server: String, store: SessionStore) {
        self.server = server
        self.store = store
    }

    func refreshTokenChanged(token: String?) {
        if let token {
            store.saveSession(SavedSession(server: server, refreshToken: token))
        } else {
            store.forgetSession()
        }
    }
}

/// Forwards download progress to the main actor; reports come from the Rust runtime's threads.
private final class ProgressForwarder: DownloadListener {
    private let report: @Sendable (String, Double) -> Void

    init(model: SessionModel) {
        report = { [weak model] id, fraction in
            Task { @MainActor in
                model?.downloadProgressed(id: id, fraction: fraction)
            }
        }
    }

    func progress(id: String, done: Int64, total: Int64) {
        report(id, total > 0 ? min(Double(done) / Double(total), 1) : 1)
    }
}

/// A file ready to hand to the system's export picker; the temporary copy goes when that closes.
struct PendingExport: Identifiable {
    let url: URL
    var id: URL { url }
}

/// The session and the sign-in form. A saved token is only ever spent once, because replaying a
/// rotated refresh token ends the session; that is why resuming lives here and not in a view.
@MainActor
@Observable
final class SessionModel {
    private static let defaultServer = "http://localhost:8080"
    private static let probeDelay = Duration.milliseconds(600)

    private(set) var session: Session = .signedOut
    private(set) var probe: Probe = .checking
    private(set) var server: String
    private(set) var busy = false
    private(set) var error: String?
    var username = ""
    var password = ""

    /// The files being downloaded. Held here so it outlives the folder views.
    private(set) var downloading = Set<String>()
    /// How far each running download is, from 0 to 1. Apart from `downloading` so that only the
    /// row backgrounds redraw on each report: redrawing a row would rebuild its open menu.
    private(set) var progress: [String: Double] = [:]
    /// Changes when the folder on screen is out of date.
    private(set) var reloadKey = 0
    /// The file Quick Look is showing.
    var preview: URL?
    /// The saved file waiting for the user to pick where it goes.
    private(set) var pendingExport: PendingExport?
    /// A failure with no dialog of its own to appear in.
    var notice: String?

    @ObservationIgnored private let store = SessionStore()
    @ObservationIgnored private var started = false
    @ObservationIgnored private var probeTask: Task<Void, Never>?
    @ObservationIgnored private var probeGeneration = 0
    /// Only the file tapped last opens when its download ends.
    @ObservationIgnored private var lastTapped: String?

    init() {
        server = SessionStore().lastServer ?? Self.defaultServer
        if let saved = store.loadSession() {
            server = saved.server
            session = .resuming(server: saved.server, error: nil)
        }
    }

    /// Begins resuming the saved session or checking the server; only the first call does.
    func start() {
        guard !started else { return }
        started = true
        if let saved = store.loadSession() {
            resume(saved)
        } else {
            probeNow()
        }
    }

    func retryResume() {
        guard let saved = store.loadSession() else {
            backToForm()
            return
        }
        session = .resuming(server: saved.server, error: nil)
        resume(saved)
    }

    private func resume(_ saved: SavedSession) {
        Task {
            do {
                let client = try newClient(server: saved.server)
                let user = try await client.restore(refreshToken: saved.refreshToken)
                enter(client, user)
            } catch CoreError.Unauthorized {
                store.forgetSession()
                backToForm()
            } catch {
                session = .resuming(server: saved.server, error: signInMessage(for: error))
            }
        }
    }

    func serverChanged(to text: String) {
        guard text != server else { return }
        server = text
        error = nil
        startProbe(after: Self.probeDelay)
    }

    /// Asks the server again straight away, such as after a failure.
    func probeNow() {
        startProbe(after: nil)
    }

    private func startProbe(after delay: Duration?) {
        probe = .checking
        probeGeneration += 1
        let generation = probeGeneration
        let typed = server
        probeTask?.cancel()
        probeTask = Task {
            if let delay {
                try? await Task.sleep(for: delay)
            }
            guard generation == probeGeneration else { return }
            let answer: Probe
            do {
                let client = CoreClient(baseUrl: try normalizeServerUrl(input: typed))
                answer = .ready(try await client.providers())
            } catch {
                answer = .failed(signInMessage(for: error))
            }
            // A newer check owns the screen now, and this late answer is for an old address.
            if generation == probeGeneration {
                probe = answer
            }
        }
    }

    func signInWithPassword() {
        let attempt = password
        password = ""
        error = nil
        busy = true
        Task {
            defer { busy = false }
            do {
                let client = try newClient(server: try normalizeServerUrl(input: server))
                let user = try await client.login(
                    username: username.trimmingCharacters(in: .whitespacesAndNewlines),
                    password: attempt
                )
                enter(client, user)
            } catch {
                self.error = signInMessage(for: error)
            }
        }
    }

    /// Signs in through the identity provider. `authenticate` shows the browser sheet and returns
    /// the URL it came back to, or `nil` if the user closed it.
    func signInWithSso(authenticate: (URL) async throws -> URL?) async {
        error = nil
        busy = true
        defer { busy = false }
        do {
            let address = try normalizeServerUrl(input: server)
            let client = try newClient(server: address)
            let request = try client.beginSso()
            guard let url = URL(string: request.url) else {
                throw CoreError.SsoFailed
            }
            guard let callback = try await authenticate(url) else { return }
            let user = try await client.finishSso(
                callbackUrl: callback.absoluteString,
                verifier: request.verifier
            )
            enter(client, user)
        } catch {
            self.error = signInMessage(for: error)
        }
    }

    func reload() {
        reloadKey += 1
    }

    /// Shows a file: at once if the copy here is current, else after downloading it.
    func openFile(client: CoreClient, entry: Entry) {
        lastTapped = entry.id
        guard downloading.insert(entry.id).inserted else { return }
        Task {
            defer { downloadEnded(entry.id) }
            do {
                let path = try await client.openCopy(
                    id: entry.id,
                    listener: ProgressForwarder(model: self)
                )
                // Nothing pops up over another file or after the user has left the app.
                if lastTapped == entry.id, UIApplication.shared.applicationState == .active {
                    preview = URL(fileURLWithPath: path)
                }
                reload()
            } catch {
                downloadFailed(client: client, entry: entry, error: error)
            }
        }
    }

    /// Downloads a file to a temporary place, then asks where to put it: iOS only lets an app
    /// export a file that already exists.
    func saveFile(client: CoreClient, entry: Entry) {
        guard downloading.insert(entry.id).inserted else { return }
        Task {
            defer { downloadEnded(entry.id) }
            let folder = FileManager.default.temporaryDirectory
                .appendingPathComponent("save", isDirectory: true)
                .appendingPathComponent(entry.id, isDirectory: true)
            do {
                try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
                let file = folder.appendingPathComponent(exportName(of: entry))
                try await client.saveCopy(
                    id: entry.id,
                    destPath: file.path,
                    listener: ProgressForwarder(model: self)
                )
                pendingExport = PendingExport(url: file)
            } catch {
                try? FileManager.default.removeItem(at: folder)
                downloadFailed(client: client, entry: entry, error: error)
            }
        }
    }

    /// The export picker closed, however that happened, so its temporary copy can go.
    func exportFinished() {
        guard let export = pendingExport else { return }
        try? FileManager.default.removeItem(at: export.url.deletingLastPathComponent())
        pendingExport = nil
    }

    func cancelDownload(client: CoreClient, entry: Entry) {
        try? client.cancelDownload(id: entry.id)
    }

    /// Deletes this device's copy of a file.
    func removeLocalCopy(client: CoreClient, entry: Entry) {
        Task {
            do {
                try await client.removeLocalCopy(id: entry.id)
            } catch {
                notice = fileMessage(for: error)
            }
            reload()
        }
    }

    fileprivate func downloadProgressed(id: String, fraction: Double) {
        // A report can arrive after the download has ended.
        if downloading.contains(id) {
            progress[id] = fraction
        }
    }

    private func downloadEnded(_ id: String) {
        downloading.remove(id)
        progress[id] = nil
    }

    private func downloadFailed(client: CoreClient, entry: Entry, error: Error) {
        switch error as? CoreError {
        case .Cancelled:
            break
        case .Unauthorized:
            // A late failure from a session that has already been replaced changes nothing.
            if case .signedIn(let current, _) = session, current === client {
                sessionEnded()
            }
        default:
            notice = "Could not download “\(entry.name)”: \(fileMessage(for: error))"
        }
    }

    func signOut() {
        let current = session
        store.forgetSession()
        forgetDownloads()
        backToForm()
        if case .signedIn(let client, _) = current {
            // Already signed out here; this only revokes the token on the server.
            Task { try? await client.logout() }
        }
    }

    /// The server ended the session while the user was browsing.
    func sessionEnded() {
        store.forgetSession()
        forgetDownloads()
        backToForm()
    }

    private func enter(_ client: CoreClient, _ user: SignedInUser) {
        username = ""
        error = nil
        session = .signedIn(client: client, user: user)
    }

    private func forgetDownloads() {
        downloading = []
        progress = [:]
        lastTapped = nil
        preview = nil
    }

    private func backToForm() {
        session = .signedOut
        probeNow()
    }

    private func newClient(server: String) throws -> CoreClient {
        try CoreClient.withSession(
            baseUrl: server,
            device: DeviceIdentity(id: store.deviceId, name: UIDevice.current.name),
            listener: TokenSaver(server: server, store: store),
            filesDir: try Self.downloadsFolder().path
        )
    }

    /// Where opened files are kept. They can be downloaded again, so they stay out of backups.
    private static func downloadsFolder() throws -> URL {
        var folder = try FileManager.default
            .url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
            .appendingPathComponent("downloads", isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try folder.setResourceValues(values)
        return folder
    }
}

/// A name that stays inside the temporary folder, whatever the server sent.
private func exportName(of entry: Entry) -> String {
    let name = (entry.name as NSString).lastPathComponent
    return name.isEmpty || name == "." || name == ".." || name == "/" ? entry.id : name
}

/// The sentence to show when a file operation fails.
func fileMessage(for error: Error) -> String {
    switch error as? CoreError {
    case .Failed(_, let detail):
        detail.prefix(1).uppercased() + detail.dropFirst()
    case .Unreachable:
        "Could not reach the server"
    default:
        error.localizedDescription
    }
}

/// The sentence to show for a failure, matching the web client's.
private func signInMessage(for error: Error) -> String {
    switch error as? CoreError {
    case .InvalidCredentials:
        "Invalid username or password"
    case .SsoExpired:
        "The sign-in expired. Please try again."
    case .SsoDenied:
        "You are not allowed to sign in with this account."
    case .InvalidServerAddress:
        "The server address must start with http:// or https://"
    case .Unreachable(let detail):
        "Could not reach the server: \(detail)"
    case .Failed(_, let detail):
        detail
    case .Unauthorized:
        "Your session has ended. Please sign in again."
    case .SsoFailed, .InvalidId, .Cancelled, nil:
        "Signing in failed. Please try again."
    }
}
