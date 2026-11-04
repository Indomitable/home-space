package app.homespace.android

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import uniffi.homespace_core.CoreClient
import uniffi.homespace_core.Entry
import uniffi.homespace_core.EntryKind

private const val DEFAULT_SERVER = "http://10.0.2.2:8080"

/** A folder on the way down from Home; `id` is null for Home itself. */
private data class Crumb(val id: String?, val name: String)

/**
 * Server check, sign-in and folder browsing through the Rust client (`clients/mobile-core`).
 */
@Composable
fun MainScreen(modifier: Modifier = Modifier) {
    var serverUrl by rememberSaveable { mutableStateOf(DEFAULT_SERVER) }
    var username by rememberSaveable { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var status by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var client by remember { mutableStateOf<CoreClient?>(null) }
    val path = remember { mutableStateListOf<Crumb>() }
    val entries = remember { mutableStateListOf<Entry>() }
    val scope = rememberCoroutineScope()

    // The Rust object holds native memory, so it is closed rather than left to the GC.
    DisposableEffect(client) {
        val current = client
        onDispose { current?.close() }
    }

    fun run(action: suspend () -> Unit) {
        busy = true
        scope.launch {
            try {
                action()
            } catch (e: Exception) {
                status = "Failed: ${e.message}"
            } finally {
                busy = false
            }
        }
    }

    LaunchedEffect(client, path.lastOrNull()) {
        val signedIn = client ?: return@LaunchedEffect
        val folder = path.lastOrNull() ?: return@LaunchedEffect
        try {
            val listed = signedIn.list(folder.id)
            entries.clear()
            entries.addAll(listed)
        } catch (e: Exception) {
            status = "Failed: ${e.message}"
        }
    }

    Column(
        modifier = modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text("Home Space", style = MaterialTheme.typography.headlineMedium)
        OutlinedTextField(
            value = serverUrl,
            onValueChange = { serverUrl = it },
            label = { Text("Server") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Button(enabled = !busy, onClick = {
            run {
                // Rust `async fn` is a Kotlin `suspend fun`.
                val health = CoreClient(serverUrl).use { it.health() }
                status = "Server status: ${health.status}"
            }
        }) { Text("Check server") }

        if (client == null) {
            OutlinedTextField(
                value = username,
                onValueChange = { username = it },
                label = { Text("Username") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                label = { Text("Password") },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                modifier = Modifier.fillMaxWidth(),
            )
            Button(enabled = !busy, onClick = {
                run {
                    val signingIn = CoreClient(serverUrl)
                    try {
                        val user = signingIn.login(username, password)
                        status = "Signed in as ${user.username}"
                        path.add(Crumb(null, "Home"))
                        client = signingIn
                    } catch (e: Exception) {
                        signingIn.close()
                        throw e
                    }
                }
            }) { Text("Sign in") }
        }

        status?.let { Text(it) }

        if (client != null) {
            Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                if (path.size > 1) {
                    TextButton(onClick = { path.removeAt(path.lastIndex) }) { Text("Back") }
                }
                Text(path.joinToString(" / ") { it.name }, style = MaterialTheme.typography.titleMedium)
            }
            LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                items(entries, key = { it.id }) { entry ->
                    when (entry.kind) {
                        EntryKind.FOLDER -> Text(
                            "📁 ${entry.name}",
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { path.add(Crumb(entry.id, entry.name)) }
                                .padding(vertical = 8.dp),
                        )
                        EntryKind.FILE -> Column(Modifier.padding(vertical = 4.dp)) {
                            Text("📄 ${entry.name}")
                            Text(
                                "${entry.sizeLabel} · ${entry.updatedLabel}",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                }
            }
        }
    }
}
