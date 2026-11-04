package app.homespace.android

import android.app.Application
import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.homespace_core.CoreClient
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.DeviceIdentity
import uniffi.homespace_core.SessionListener
import uniffi.homespace_core.SignInOptions
import uniffi.homespace_core.SignedInUser
import uniffi.homespace_core.normalizeServerUrl

/** The server on the host machine, as seen from the emulator. */
private const val DEFAULT_SERVER = "http://10.0.2.2:8080"
private const val PROBE_DELAY_MILLIS = 600L

sealed interface Session {
    /** Trying a saved session; `error` is set when that failed but the session may still work. */
    data class Resuming(val server: String, val error: String? = null) : Session
    data object SignedOut : Session
    data class SignedIn(val client: CoreClient, val user: SignedInUser) : Session
}

/** What the server said about how to sign in. */
sealed interface Probe {
    data object Checking : Probe
    data class Ready(val options: SignInOptions) : Probe
    data class Failed(val message: String) : Probe
}

/** A folder on the way down from Home; `id` is null for Home itself. */
data class Crumb(val id: String?, val name: String)

/**
 * The session and the sign-in form. It outlives the activity, so the signed-in client survives
 * rotation, and a saved token is only ever spent once: replaying a rotated refresh token ends
 * the session.
 */
class SessionViewModel(application: Application) : AndroidViewModel(application) {
    private val store = SessionStore(application)
    private var probeJob: Job? = null

    var session by mutableStateOf<Session>(Session.SignedOut)
        private set
    var server by mutableStateOf(store.lastServer ?: DEFAULT_SERVER)
        private set
    var probe by mutableStateOf<Probe>(Probe.Checking)
        private set
    var pendingSso by mutableStateOf(store.loadPendingSso())
        private set
    var busy by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var username by mutableStateOf("")
    var password by mutableStateOf("")

    /** Where the folder browser is, kept here so that rotating the phone doesn't reset it. */
    val path = mutableStateListOf<Crumb>()

    init {
        val saved = store.loadSession()
        val pending = pendingSso
        if (saved != null) {
            session = Session.Resuming(saved.server)
            server = saved.server
            resume(saved)
        } else {
            if (pending != null) server = pending.server
            probeNow()
        }
    }

    fun retryResume() {
        val saved = store.loadSession()
        if (saved == null) {
            backToForm()
        } else {
            session = Session.Resuming(saved.server)
            resume(saved)
        }
    }

    private fun resume(saved: SavedSession) {
        viewModelScope.launch {
            val client = newClient(saved.server)
            try {
                enter(client, client.restore(saved.refreshToken))
            } catch (e: CoreException) {
                client.close()
                if (e is CoreException.Unauthorized) {
                    store.forgetSession()
                    backToForm()
                } else {
                    session = Session.Resuming(saved.server, e.signInMessage())
                }
            }
        }
    }

    fun onServerChanged(text: String) {
        server = text
        error = null
        probe = Probe.Checking
        probeJob?.cancel()
        probeJob = viewModelScope.launch {
            delay(PROBE_DELAY_MILLIS)
            check()
        }
    }

    /** Asks the server again straight away, such as after a failure. */
    fun probeNow() {
        probe = Probe.Checking
        probeJob?.cancel()
        probeJob = viewModelScope.launch { check() }
    }

    // Replacing the job cancels the check before it, so a late answer can't overwrite a new one.
    private suspend fun check() {
        probe = try {
            val answer = CoreClient(normalizeServerUrl(server)).use { it.providers() }
            Probe.Ready(answer)
        } catch (e: CoreException) {
            Probe.Failed(e.signInMessage())
        }
    }

    fun signInWithPassword() {
        val attempt = password
        password = ""
        error = null
        busy = true
        viewModelScope.launch {
            try {
                val client = newClient(normalizeServerUrl(server))
                try {
                    enter(client, client.login(username.trim(), attempt))
                } catch (e: CoreException) {
                    client.close()
                    throw e
                }
            } catch (e: CoreException) {
                error = e.signInMessage()
            } finally {
                busy = false
            }
        }
    }

    /** Starts a single sign-on and returns the URL to open in a browser. */
    fun beginSso(name: String): String? {
        error = null
        return try {
            val address = normalizeServerUrl(server)
            val request = CoreClient(address).use { it.beginSso() }
            val pending = PendingSso(address, request.verifier, name)
            if (store.savePendingSso(pending)) {
                pendingSso = pending
                request.url
            } else {
                error = SIGN_IN_FAILED
                null
            }
        } catch (e: CoreException) {
            error = e.signInMessage()
            null
        }
    }

    fun cancelSso() {
        store.forgetPendingSso()
        pendingSso = null
    }

    fun onBrowserFailed() {
        cancelSso()
        error = "No browser is available to sign in with."
    }

    /** The browser came back. Ignored unless a sign-in is waiting, so a stray link does nothing. */
    fun onSsoCallback(url: String) {
        val pending = store.loadPendingSso() ?: return
        cancelSso()
        server = pending.server
        error = null
        busy = true
        viewModelScope.launch {
            val client = newClient(pending.server)
            try {
                enter(client, client.finishSso(url, pending.verifier))
            } catch (e: CoreException) {
                client.close()
                error = e.signInMessage()
            } finally {
                busy = false
            }
        }
    }

    fun signOut() {
        val current = session
        store.forgetSession()
        backToForm()
        if (current is Session.SignedIn) {
            viewModelScope.launch {
                // Already signed out here; this only revokes the token on the server.
                try {
                    current.client.logout()
                } catch (_: CoreException) {
                } finally {
                    current.client.close()
                }
            }
        }
    }

    /** The server ended the session while the user was browsing. */
    fun onSessionEnded() {
        val current = session
        store.forgetSession()
        backToForm()
        if (current is Session.SignedIn) current.client.close()
    }

    override fun onCleared() {
        (session as? Session.SignedIn)?.client?.close()
    }

    private fun enter(client: CoreClient, user: SignedInUser) {
        username = ""
        error = null
        path.clear()
        path.add(Crumb(null, "Home"))
        session = Session.SignedIn(client, user)
    }

    private fun backToForm() {
        session = Session.SignedOut
        path.clear()
        probeNow()
    }

    private fun newClient(address: String): CoreClient = CoreClient.withSession(
        address,
        DeviceIdentity(store.deviceId, Build.MODEL),
        object : SessionListener {
            override fun refreshTokenChanged(token: String?) {
                if (token == null) {
                    store.forgetSession()
                } else {
                    store.saveSession(address, token)
                }
            }
        },
    )
}

private const val SIGN_IN_FAILED = "Signing in failed. Please try again."

/** The sentence to show for a failure, matching the web client's. */
private fun CoreException.signInMessage(): String = when (this) {
    is CoreException.InvalidCredentials -> "Invalid username or password"
    is CoreException.SsoExpired -> "The sign-in expired. Please try again."
    is CoreException.SsoDenied -> "You are not allowed to sign in with this account."
    is CoreException.SsoFailed -> SIGN_IN_FAILED
    is CoreException.InvalidServerAddress ->
        "The server address must start with http:// or https://"
    is CoreException.Unreachable -> "Could not reach the server: $detail"
    is CoreException.Failed -> detail
    is CoreException.Unauthorized -> "Your session has ended. Please sign in again."
    is CoreException.InvalidId -> SIGN_IN_FAILED
}
