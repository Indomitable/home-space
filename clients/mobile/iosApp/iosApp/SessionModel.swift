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

    @ObservationIgnored private let store = SessionStore()
    @ObservationIgnored private var started = false
    @ObservationIgnored private var probeTask: Task<Void, Never>?
    @ObservationIgnored private var probeGeneration = 0

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

    func signOut() {
        let current = session
        store.forgetSession()
        backToForm()
        if case .signedIn(let client, _) = current {
            // Already signed out here; this only revokes the token on the server.
            Task { try? await client.logout() }
        }
    }

    /// The server ended the session while the user was browsing.
    func sessionEnded() {
        store.forgetSession()
        backToForm()
    }

    private func enter(_ client: CoreClient, _ user: SignedInUser) {
        username = ""
        error = nil
        session = .signedIn(client: client, user: user)
    }

    private func backToForm() {
        session = .signedOut
        probeNow()
    }

    private func newClient(server: String) throws -> CoreClient {
        try CoreClient.withSession(
            baseUrl: server,
            device: DeviceIdentity(id: store.deviceId, name: UIDevice.current.name),
            listener: TokenSaver(server: server, store: store)
        )
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
    case .SsoFailed, .InvalidId, nil:
        "Signing in failed. Please try again."
    }
}
