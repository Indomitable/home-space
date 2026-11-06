import BackgroundTasks
import Foundation
import Network
import Photos

/// Runs auto-upload passes while the app is in front: on opening it, every minute, when the photo
/// library changes and when the network changes. Stops a pass when the network no longer allows it.
@MainActor
@Observable
final class ForegroundSync: NSObject {
    private static let interval = Duration.seconds(60)

    /// "Sync only on Wi-Fi" is on, and the phone isn't on it.
    private(set) var waitingForWifi = false

    @ObservationIgnored private let session: ActiveSession
    @ObservationIgnored private let onSessionEnded: () -> Void
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private var monitor: NWPathMonitor?
    @ObservationIgnored private var path: NWPath?
    @ObservationIgnored private var observingPhotos = false

    init(session: ActiveSession, onSessionEnded: @escaping () -> Void) {
        self.session = session
        self.onSessionEnded = onSessionEnded
    }

    func start() {
        guard timer == nil else { return }
        let monitor = NWPathMonitor()
        monitor.pathUpdateHandler = { [weak self] path in
            Task { @MainActor in self?.networkChanged(path) }
        }
        monitor.start(queue: .global(qos: .utility))
        self.monitor = monitor
        timer = Task { [weak self] in
            while !Task.isCancelled {
                await self?.pass(retryNow: false)
                try? await Task.sleep(for: Self.interval)
            }
        }
        observePhotos()
    }

    /// Leaves a pass running only if syncing in the background is allowed.
    func stop() {
        timer?.cancel()
        timer = nil
        monitor?.cancel()
        monitor = nil
        if let auto = try? session.autoUpload(), (try? auto.settings().background) != true {
            auto.stop()
        }
    }

    /// "Sync now", or a change of settings: tries at once what was waiting to be tried again.
    func syncNow() {
        Task { await pass(retryNow: true) }
    }

    /// The library can be looked at only after the user has allowed it, so `start` isn't enough.
    func observePhotos() {
        let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
        guard !observingPhotos, status == .authorized || status == .limited else { return }
        observingPhotos = true
        PHPhotoLibrary.shared().register(self)
    }

    private func networkChanged(_ path: NWPath) {
        self.path = path
        let allowed = networkAllows()
        waitingForWifi = !allowed
        if allowed {
            Task { await pass(retryNow: false) }
        } else {
            (try? session.autoUpload())?.stop()
        }
    }

    private func networkAllows() -> Bool {
        guard let path else { return false }
        return networkAllowsUse(of: path, session: session)
    }

    private func pass(retryNow: Bool) async {
        guard networkAllows(), let auto = try? session.autoUpload() else { return }
        do {
            _ = try await auto.runPass(retryNow: retryNow)
        } catch CoreError.Unauthorized {
            onSessionEnded()
        } catch {
            // Shown per source on the settings screen; the next pass tries again.
        }
    }
}

extension ForegroundSync: PHPhotoLibraryChangeObserver {
    nonisolated func photoLibraryDidChange(_ changeInstance: PHChange) {
        Task { @MainActor in await self.pass(retryNow: false) }
    }
}

@MainActor
private func networkAllowsUse(of path: NWPath, session: ActiveSession) -> Bool {
    guard path.status == .satisfied else { return false }
    let wifiOnly = (try? session.autoUpload().settings().wifiOnly) ?? true
    return !wifiOnly || !path.isExpensive
}

/// Runs passes while the app isn't open, when "Sync in background" is on. iOS decides when: a
/// refresh is short and comes now and then, and processing tends to wait for the phone to charge.
enum BackgroundSync {
    // Also listed in `project.yml`, as BGTaskSchedulerPermittedIdentifiers.
    private static let refresh = "app.homespace.ios.sync-refresh"
    private static let processing = "app.homespace.ios.sync-processing"

    /// Must be called before the app finishes launching.
    static func register() {
        for identifier in [refresh, processing] {
            BGTaskScheduler.shared.register(forTaskWithIdentifier: identifier, using: nil) { task in
                Task { @MainActor in await run(task) }
            }
        }
    }

    /// Asks iOS for the next run, or takes back the request if the setting is off.
    @MainActor
    static func schedule() {
        guard let auto = try? ClientHolder.shared.active?.autoUpload(),
            (try? auto.settings().background) == true
        else {
            BGTaskScheduler.shared.cancelAllTaskRequests()
            return
        }
        let quick = BGAppRefreshTaskRequest(identifier: refresh)
        try? BGTaskScheduler.shared.submit(quick)
        let long = BGProcessingTaskRequest(identifier: processing)
        long.requiresNetworkConnectivity = true
        try? BGTaskScheduler.shared.submit(long)
    }

    @MainActor
    private static func run(_ task: BGTask) async {
        // Keeps running after the next is asked for, since only one of each can be waiting.
        defer { schedule() }
        guard let session = try? await ClientHolder.shared.resume(),
            let auto = try? session.autoUpload(),
            (try? auto.settings().background) == true,
            await isNetworkAllowed(session)
        else {
            task.setTaskCompleted(success: true)
            return
        }
        task.expirationHandler = { auto.stop() }
        let summary = try? await auto.runPass(retryNow: false)
        task.setTaskCompleted(success: summary != nil)
    }

    @MainActor
    private static func isNetworkAllowed(_ session: ActiveSession) async -> Bool {
        let path = await withCheckedContinuation { (continuation: CheckedContinuation<NWPath, Never>) in
            let monitor = NWPathMonitor()
            monitor.pathUpdateHandler = { path in
                monitor.cancel()
                continuation.resume(returning: path)
            }
            monitor.start(queue: .global(qos: .utility))
        }
        return networkAllowsUse(of: path, session: session)
    }
}
