package app.homespace.android

import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier

/** Runtime permission for private addresses, from Android 17 (API 37). */
private const val ACCESS_LOCAL_NETWORK = "android.permission.ACCESS_LOCAL_NETWORK"

class MainActivity : ComponentActivity() {
    private val localNetwork =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

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
        setContent {
            MaterialTheme {
                // Spike: the Kotlin (KMP) and Rust (UniFFI) clients side by side.
                var tab by rememberSaveable { mutableIntStateOf(0) }
                Scaffold(
                    bottomBar = {
                        NavigationBar {
                            NavigationBarItem(
                                selected = tab == 0,
                                onClick = { tab = 0 },
                                icon = { Text("K") },
                                label = { Text("Kotlin") },
                            )
                            NavigationBarItem(
                                selected = tab == 1,
                                onClick = { tab = 1 },
                                icon = { Text("R") },
                                label = { Text("Rust") },
                            )
                        }
                    },
                ) { padding ->
                    Box(Modifier.padding(padding)) {
                        when (tab) {
                            0 -> ServerCheckScreen()
                            else -> RustCoreScreen()
                        }
                    }
                }
            }
        }
    }
}
