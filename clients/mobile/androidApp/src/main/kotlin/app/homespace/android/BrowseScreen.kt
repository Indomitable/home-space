package app.homespace.android

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
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
import androidx.compose.material3.Checkbox
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
    // The ids of the selected rows; a long press starts selecting, and it ends once this is empty.
    var selected by remember { mutableStateOf(emptySet<String>()) }
    val selecting = selected.isNotEmpty()

    BackHandler(enabled = path.size > 1) { path.removeAt(path.lastIndex) }
    // Declared last so it goes first: back leaves the selection before it leaves the folder.
    BackHandler(enabled = selecting) { selected = emptySet() }

    LaunchedEffect(session.client, path.lastOrNull()) {
        val folder = path.lastOrNull() ?: return@LaunchedEffect
        error = null
        selected = emptySet()
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
                if (selecting) "${selected.size} selected" else session.user.username,
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.weight(1f),
            )
            if (selecting) {
                if (selected.size < entries.size) {
                    TextButton(onClick = { selected = entries.map { it.id }.toSet() }) {
                        Text("Select all")
                    }
                }
                TextButton(onClick = { selected = emptySet() }) { Text("Cancel") }
            } else {
                TextButton(onClick = model::signOut) { Text("Sign out") }
            }
        }
        Text(path.joinToString(" / ") { it.name }, style = MaterialTheme.typography.titleMedium)
        error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            items(entries, key = { it.id }) { entry ->
                val toggle = {
                    selected = if (entry.id in selected) selected - entry.id else selected + entry.id
                }
                EntryRow(
                    entry = entry,
                    selecting = selecting,
                    isSelected = entry.id in selected,
                    onClick = {
                        when {
                            selecting -> toggle()
                            entry.kind == EntryKind.FOLDER -> path.add(Crumb(entry.id, entry.name))
                        }
                    },
                    onLongClick = toggle,
                )
            }
        }
    }
}

@Composable
private fun EntryRow(
    entry: Entry,
    selecting: Boolean,
    isSelected: Boolean,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
) {
    val background =
        if (isSelected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(background)
            .combinedClickable(onLongClick = onLongClick, onClick = onClick)
            .padding(vertical = if (entry.kind == EntryKind.FOLDER) 8.dp else 4.dp),
        horizontalArrangement = Arrangement.spacedBy(ICON_GAP),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // The whole row takes the tap, so the box only shows the state.
        if (selecting) Checkbox(checked = isSelected, onCheckedChange = null)
        NodeIcon(entry.icon)
        when (entry.kind) {
            EntryKind.FOLDER -> Text(entry.name)
            EntryKind.FILE -> Column {
                Text(entry.name)
                Text(
                    "${entry.sizeLabel} · ${entry.updatedLabel}",
                    style = MaterialTheme.typography.bodySmall,
                )
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
