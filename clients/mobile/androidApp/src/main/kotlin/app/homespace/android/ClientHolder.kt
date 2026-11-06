package app.homespace.android

import android.content.Context
import android.os.Build
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import uniffi.homespace_core.AutoUpload
import uniffi.homespace_core.AutoUploadListener
import uniffi.homespace_core.CoreClient
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.DeviceIdentity
import uniffi.homespace_core.SessionListener
import uniffi.homespace_core.SignedInUser

/** The signed-in client and what hangs off it. */
class ActiveSession(val client: CoreClient, val user: SignedInUser, context: Context) {
    private val appContext = context.applicationContext

    init {
        client.setOfflineListener(
            object : AutoUploadListener {
                override fun changed() {
                    ClientHolder.syncChanged()
                }
            },
        )
    }

    val autoUpload: AutoUpload by lazy {
        AutoUpload.open(
            client,
            File(appContext.filesDir, SYNC_DIR).path,
            AndroidSources(appContext),
            object : AutoUploadListener {
                override fun changed() {
                    ClientHolder.syncChanged()
                }
            },
        )
    }
}

/**
 * The one client of this process, shared by the screens and background work.
 *
 * Refresh tokens are single use and replaying one ends the session, so two clients must never
 * restore from the same saved token: whichever comes second, the app or a background job, has to
 * find the first one here.
 */
object ClientHolder {
    private val restoring = Mutex()
    private val changes = MutableStateFlow(0)

    @Volatile
    var active: ActiveSession? = null
        private set

    /** Changes whenever what auto-upload or the offline copies report may have changed. */
    val syncChanges: StateFlow<Int> = changes

    internal fun syncChanged() {
        changes.value += 1
    }

    fun newClient(context: Context, address: String): CoreClient {
        val appContext = context.applicationContext
        val store = SessionStore(appContext)
        // Copies from before they were synced. They download again when opened.
        Thread { File(appContext.filesDir, LEGACY_DOWNLOADS_DIR).deleteRecursively() }.start()
        return CoreClient.withSession(
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
            File(appContext.filesDir, OFFLINE_DIR).path,
        )
    }

    /** Makes a client that has just signed in the active one. */
    fun adopt(context: Context, client: CoreClient, user: SignedInUser): ActiveSession =
        ActiveSession(client, user, context).also { active = it }

    /**
     * The active session, else one restored from the saved token, else `null` when nobody is
     * signed in. A token the server refused is forgotten.
     */
    suspend fun resume(context: Context): ActiveSession? = restoring.withLock {
        active?.let { return it }
        val store = SessionStore(context.applicationContext)
        val saved = store.loadSession() ?: return null
        val client = newClient(context, saved.server)
        try {
            adopt(context, client, client.restore(saved.refreshToken))
        } catch (e: CoreException) {
            client.close()
            if (e is CoreException.Unauthorized) store.forgetSession()
            throw e
        }
    }

    /** The session ended. Closing waits for calls in progress, so this is safe during a pass. */
    fun release(session: ActiveSession) {
        if (active === session) active = null
    }
}

/** Under the app's files folder; `res/xml/file_paths.xml` shares this folder with viewers. */
private const val OFFLINE_DIR = "offline"

private const val LEGACY_DOWNLOADS_DIR = "downloads"

/** Under the app's files folder; not shared, and not wiped with the offline copies. */
private const val SYNC_DIR = "sync"
