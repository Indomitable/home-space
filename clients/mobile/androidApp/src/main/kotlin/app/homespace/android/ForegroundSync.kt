package app.homespace.android

import android.content.Context
import android.database.ContentObserver
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Handler
import android.os.Looper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.QueuedUploadState

private const val INTERVAL_MILLIS = 60_000L

/**
 * Runs auto-upload passes and offline copies' sync while the app is open: on opening it, every
 * minute, when a new photo appears and when the network changes. Stops them when the network no
 * longer allows it.
 */
class ForegroundSync(
    context: Context,
    private val session: ActiveSession,
    private val scope: CoroutineScope,
    private val onSessionEnded: () -> Unit,
    private val onNetworkAllowed: (Boolean) -> Unit,
) {
    private val appContext = context.applicationContext
    private val connectivity = appContext.getSystemService(ConnectivityManager::class.java)
    private val wake = Channel<Boolean>(Channel.CONFLATED)
    private var jobs = emptyList<Job>()
    private var uploads: Job? = null

    private val network = object : ConnectivityManager.NetworkCallback() {
        override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
            onNetworkChanged()
        }

        override fun onLost(network: Network) {
            onNetworkChanged()
        }
    }

    private val media = object : ContentObserver(Handler(Looper.getMainLooper())) {
        override fun onChange(selfChange: Boolean) {
            wake.trySend(false)
        }
    }

    fun start() {
        if (jobs.isNotEmpty()) return
        connectivity.registerDefaultNetworkCallback(network)
        appContext.contentResolver.registerContentObserver(imagesUri(), true, media)
        appContext.contentResolver.registerContentObserver(videosUri(), true, media)
        jobs = listOf(
            scope.launch {
                for (retryNow in wake) pass(retryNow)
            },
            scope.launch {
                while (true) {
                    wake.trySend(false)
                    delay(INTERVAL_MILLIS)
                }
            },
        )
    }

    /** Leaves a pass running only if syncing in the background is allowed. */
    fun stop() {
        if (jobs.isEmpty()) return
        jobs.forEach(Job::cancel)
        jobs = emptyList()
        connectivity.unregisterNetworkCallback(network)
        appContext.contentResolver.unregisterContentObserver(media)
        runCatching {
            if (!session.autoUpload.settings().background) session.autoUpload.stop()
        }
        // Nothing watches the network from here on. The background job has its own constraints.
        scope.launch { allowOffline(false) }
    }

    /** "Sync now", or a change of settings: tries at once what was waiting to be tried again. */
    fun syncNow() {
        // Passes run one at a time, so the next can't stop one that the new settings don't allow.
        applyNetworkRule()
        wake.trySend(true)
    }

    /** Sends the picked files, whatever "Sync only on Wi-Fi" says: the user asked for these. */
    fun runUploads() {
        val waiting = runCatching { session.uploads.items() }.getOrNull().orEmpty()
        if (waiting.any { it.state != QueuedUploadState.FAILED }) scheduleUploads(appContext)
        if (uploads?.isActive == true) {
            session.uploads.start()
            return
        }
        uploads = scope.launch {
            try {
                session.uploads.run()
            } catch (_: CoreException.Unauthorized) {
                onSessionEnded()
            } catch (_: CoreException) {
                // The rows show it, and the next try is a minute away at most.
            }
        }
    }

    private fun onNetworkChanged() {
        runUploads()
        if (applyNetworkRule()) wake.trySend(false)
    }

    /** Whether the network may be used, stopping what is under way if not. */
    private fun applyNetworkRule(): Boolean {
        val allowed = runCatching { networkAllows() }.getOrDefault(false)
        onNetworkAllowed(allowed)
        if (!allowed) {
            session.autoUpload.stop()
            scope.launch { allowOffline(false) }
        }
        return allowed
    }

    /** Whether the current network may be used. */
    fun networkAllows(): Boolean {
        if (!session.autoUpload.settings().wifiOnly) return true
        val capabilities = connectivity.getNetworkCapabilities(connectivity.activeNetwork)
            ?: return false
        return capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
    }

    /** Failing only means that the phone can't store the copies, which opening a file shows too. */
    private suspend fun allowOffline(allowed: Boolean) {
        runCatching { session.client.offlineAllow(allowed) }
    }

    private suspend fun pass(retryNow: Boolean) {
        runUploads()
        try {
            val allowed = networkAllows()
            onNetworkAllowed(allowed)
            allowOffline(allowed)
            if (allowed) {
                if (retryNow) session.client.offlineSyncNow(true)
                session.autoUpload.runPass(retryNow)
            }
        } catch (_: CoreException.Unauthorized) {
            onSessionEnded()
        } catch (_: CoreException) {
            // Shown per source on the settings screen; the next pass tries again.
        }
    }
}
