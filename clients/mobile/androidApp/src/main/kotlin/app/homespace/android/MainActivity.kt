package app.homespace.android

import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.Modifier

/** Runtime permission for private addresses, from Android 17 (API 37). */
private const val ACCESS_LOCAL_NETWORK = "android.permission.ACCESS_LOCAL_NETWORK"

class MainActivity : ComponentActivity() {
    private val model: SessionViewModel by viewModels()

    // The first check of the server ran before the user answered, so it may have been dropped.
    private val localNetwork =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted && model.session is Session.SignedOut) model.probeNow()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // A home server is usually on the LAN; without this grant Android 17 silently drops
        // the connection, which looks like a timeout.
        if (Build.VERSION.SDK_INT >= 37 &&
            checkSelfPermission(ACCESS_LOCAL_NETWORK) != PackageManager.PERMISSION_GRANTED
        ) {
            localNetwork.launch(ACCESS_LOCAL_NETWORK)
        }
        handleCallback(intent)
        setContent {
            MaterialTheme(
                colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme(),
            ) {
                Surface {
                    val screen = Modifier.safeDrawingPadding()
                    when (val session = model.session) {
                        is Session.Resuming -> ResumingScreen(session, model, screen)
                        Session.SignedOut -> LoginScreen(model, screen)
                        is Session.SignedIn -> SignedInScreen(session, model, screen)
                    }
                }
            }
        }
    }

    override fun onStart() {
        super.onStart()
        model.onForegroundChanged(true)
    }

    override fun onStop() {
        model.onForegroundChanged(false)
        super.onStop()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleCallback(intent)
    }

    /** Called from `onCreate` too: Android may have killed the app while the browser was open. */
    private fun handleCallback(intent: Intent?) {
        if (intent?.action == Intent.ACTION_VIEW) {
            intent.dataString?.let(model::onSsoCallback)
        }
    }
}
