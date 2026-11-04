package app.homespace.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.homespace.shared.HomeSpaceClient
import kotlinx.coroutines.launch

/** The emulator reaches the host machine's localhost at 10.0.2.2. */
private const val DEFAULT_SERVER = "http://10.0.2.2:8080"

/** Placeholder first screen: checks a server through the shared Kotlin client. */
@Composable
fun ServerCheckScreen() {
    var serverUrl by rememberSaveable { mutableStateOf(DEFAULT_SERVER) }
    var result by remember { mutableStateOf<String?>(null) }
    var checking by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Scaffold { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Home Space", style = MaterialTheme.typography.headlineMedium)
            OutlinedTextField(
                value = serverUrl,
                onValueChange = { serverUrl = it },
                label = { Text("Server") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Button(
                enabled = !checking,
                onClick = {
                    checking = true
                    scope.launch {
                        val client = HomeSpaceClient(serverUrl)
                        result = try {
                            "Server status: ${client.health().status}"
                        } catch (e: Exception) {
                            "Could not reach the server: ${e.message}"
                        } finally {
                            client.close()
                            checking = false
                        }
                    }
                },
            ) {
                Text(if (checking) "Checking…" else "Check server")
            }
            result?.let { Text(it) }
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun ServerCheckScreenPreview() {
    ServerCheckScreen()
}
