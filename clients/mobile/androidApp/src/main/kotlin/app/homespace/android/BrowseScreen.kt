package app.homespace.android

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.Entry
import uniffi.homespace_core.EntryIcon
import uniffi.homespace_core.EntryKind

private val ICON_SIZE = 20.dp
private val ICON_GAP = 8.dp

// The web client's --color-accent.
private val FOLDER_ACCENT_LIGHT = Color(0xFF2563EB)
private val FOLDER_ACCENT_DARK = Color(0xFF6D9EFF)

/** Folder browsing through the Rust client (`clients/mobile-core`). */
@Composable
fun BrowseScreen(session: Session.SignedIn, model: SessionViewModel, modifier: Modifier = Modifier) {
    val path = model.path
    val entries = remember { mutableStateListOf<Entry>() }
    var error by remember { mutableStateOf<String?>(null) }

    BackHandler(enabled = path.size > 1) { path.removeAt(path.lastIndex) }

    LaunchedEffect(session.client, path.lastOrNull()) {
        val folder = path.lastOrNull() ?: return@LaunchedEffect
        error = null
        try {
            val listed = session.client.list(folder.id)
            entries.clear()
            entries.addAll(listed)
        } catch (_: CoreException.Unauthorized) {
            model.onSessionEnded()
        } catch (e: CoreException) {
            error = "Failed: ${e.message}"
        }
    }

    Column(
        modifier = modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                session.user.username,
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.weight(1f),
            )
            TextButton(onClick = model::signOut) { Text("Sign out") }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            if (path.size > 1) {
                TextButton(onClick = { path.removeAt(path.lastIndex) }) { Text("Back") }
            }
            Text(path.joinToString(" / ") { it.name }, style = MaterialTheme.typography.titleMedium)
        }
        error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            items(entries, key = { it.id }) { entry ->
                when (entry.kind) {
                    EntryKind.FOLDER -> Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { path.add(Crumb(entry.id, entry.name)) }
                            .padding(vertical = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(ICON_GAP),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        NodeIcon(entry.icon)
                        Text(entry.name)
                    }
                    EntryKind.FILE -> Row(
                        modifier = Modifier.padding(vertical = 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(ICON_GAP),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        NodeIcon(entry.icon)
                        Column {
                            Text(entry.name)
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

/** The same icons and colours as the web client: only folders are coloured. */
@Composable
private fun NodeIcon(icon: EntryIcon) {
    val drawable = when (icon) {
        EntryIcon.FOLDER -> R.drawable.ic_node_folder
        EntryIcon.IMAGE -> R.drawable.ic_node_image
        EntryIcon.DOCUMENT -> R.drawable.ic_node_description
        EntryIcon.OTHER -> R.drawable.ic_node_draft
    }
    val tint = when {
        icon != EntryIcon.FOLDER -> MaterialTheme.colorScheme.onSurface
        isSystemInDarkTheme() -> FOLDER_ACCENT_DARK
        else -> FOLDER_ACCENT_LIGHT
    }
    Icon(
        painterResource(drawable),
        contentDescription = null,
        tint = tint,
        modifier = Modifier.size(ICON_SIZE),
    )
}
