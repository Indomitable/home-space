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

private const val INTERVAL_MILLIS = 60_000L

/**
 * Runs auto-upload passes while the app is open: on opening it, every minute, when a new photo
 * appears and when the network changes. Stops a pass when the network no longer allows it.
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
    }

    /** "Sync now", or a change of settings: tries at once what was waiting to be tried again. */
    fun syncNow() {
        wake.trySend(true)
    }

    private fun onNetworkChanged() {
        val allowed = runCatching { networkAllows() }.getOrDefault(false)
        onNetworkAllowed(allowed)
        if (allowed) wake.trySend(false) else session.autoUpload.stop()
    }

    /** Whether the current network may be used. */
    fun networkAllows(): Boolean {
        if (!session.autoUpload.settings().wifiOnly) return true
        val capabilities = connectivity.getNetworkCapabilities(connectivity.activeNetwork)
            ?: return false
        return capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
    }

    private suspend fun pass(retryNow: Boolean) {
        try {
            val allowed = networkAllows()
            onNetworkAllowed(allowed)
            if (allowed) session.autoUpload.runPass(retryNow)
        } catch (_: CoreException.Unauthorized) {
            onSessionEnded()
        } catch (_: CoreException) {
            // Shown per source on the settings screen; the next pass tries again.
        }
    }
}
