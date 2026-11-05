package app.homespace.android

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import uniffi.homespace_core.ConflictSide
import uniffi.homespace_core.CoreClient
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.Entry
import uniffi.homespace_core.EntryDetails
import uniffi.homespace_core.EntryIcon
import uniffi.homespace_core.EntryKind
import uniffi.homespace_core.MoveMode
import uniffi.homespace_core.MoveResult
import uniffi.homespace_core.alreadyIn

private val PICKER_HEIGHT = 240.dp
private const val WORKING = "Working…"

/** The dialog the browser is showing. */
sealed interface FileAction {
    data class Details(val entry: Entry) : FileAction
    data class Rename(val entry: Entry) : FileAction
    data class Move(val entries: List<Entry>) : FileAction
    data class Delete(val entries: List<Entry>) : FileAction
}

/** Shows the dialog of `action`. `onDone` means something changed, so the folder is out of date. */
@Composable
fun FileDialog(
    action: FileAction,
    client: CoreClient,
    onDismiss: () -> Unit,
    onDone: () -> Unit,
    onSessionEnded: () -> Unit,
) {
    when (action) {
        is FileAction.Details -> DetailsSheet(action.entry, client, onDismiss, onSessionEnded)
        is FileAction.Rename -> RenameDialog(action.entry, client, onDismiss, onDone, onSessionEnded)
        is FileAction.Move -> MoveDialog(action.entries, client, onDismiss, onDone, onSessionEnded)
        is FileAction.Delete -> DeleteDialog(action.entries, client, onDismiss, onDone, onSessionEnded)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DetailsSheet(
    entry: Entry,
    client: CoreClient,
    onDismiss: () -> Unit,
    onSessionEnded: () -> Unit,
) {
    var details by remember { mutableStateOf<EntryDetails?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(entry.id) {
        try {
            details = client.details(entry.id)
        } catch (_: CoreException.Unauthorized) {
            onSessionEnded()
        } catch (e: CoreException) {
            error = e.fileMessage()
        }
    }
    // Fully open, because a half-open sheet runs under the navigation bar.
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().navigationBarsPadding().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(entry.name, style = MaterialTheme.typography.titleMedium)
            error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            details?.let { loaded ->
                DetailRow("Type", loaded.typeLabel)
                if (loaded.entry.kind == EntryKind.FILE) {
                    DetailRow("Size", loaded.entry.sizeLabel)
                    DetailRow("Version", loaded.entry.version.toString())
                }
                DetailRow("Created", localDate(loaded.createdAt))
                DetailRow("Modified", localDate(loaded.entry.updatedAt))
                loaded.deviceLabel?.let { DetailRow(loaded.deviceTitle, it) }
            }
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Column {
        Text(label, style = MaterialTheme.typography.labelMedium)
        Text(value)
    }
}

@Composable
private fun RenameDialog(
    entry: Entry,
    client: CoreClient,
    onDismiss: () -> Unit,
    onDone: () -> Unit,
    onSessionEnded: () -> Unit,
) {
    var name by remember {
        mutableStateOf(TextFieldValue(entry.name, TextRange(0, entry.name.length)))
    }
    var saving by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    val submit = submit@{
        val typed = name.text.trim()
        if (typed.isEmpty() || typed == entry.name) {
            onDismiss()
            return@submit
        }
        saving = true
        error = null
        scope.launch {
            try {
                client.rename(entry.id, typed)
                onDone()
            } catch (_: CoreException.Unauthorized) {
                onSessionEnded()
            } catch (e: CoreException) {
                error = e.fileMessage()
                saving = false
            }
        }
        Unit
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Rename “${entry.name}”") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    singleLine = true,
                    placeholder = { Text("Name") },
                    modifier = Modifier.focusRequester(focus),
                )
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = {
            TextButton(onClick = submit, enabled = !saving) {
                Text(if (saving) WORKING else "Rename")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun DeleteDialog(
    entries: List<Entry>,
    client: CoreClient,
    onDismiss: () -> Unit,
    onDone: () -> Unit,
    onSessionEnded: () -> Unit,
) {
    var saving by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val title = entries.singleOrNull()?.let { "Move “${it.name}” to recycle bin?" }
        ?: "Move ${entries.size} items to recycle bin?"
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (entries.any { it.kind == EntryKind.FOLDER }) {
                    Text(
                        "Files inside folders are moved too. " +
                            "You can restore them from the recycle bin.",
                    )
                }
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = {
            TextButton(
                enabled = !saving,
                onClick = {
                    saving = true
                    error = null
                    scope.launch {
                        try {
                            client.delete(entries.map { it.id })
                            onDone()
                        } catch (_: CoreException.Unauthorized) {
                            onSessionEnded()
                        } catch (e: CoreException) {
                            error = e.fileMessage()
                            saving = false
                        }
                    }
                },
            ) { Text(if (saving) WORKING else "Move to bin") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun MoveDialog(
    entries: List<Entry>,
    client: CoreClient,
    onDismiss: () -> Unit,
    onDone: () -> Unit,
    onSessionEnded: () -> Unit,
) {
    // The folders opened on the way down from Home; empty at Home.
    var trail by remember { mutableStateOf(emptyList<Crumb>()) }
    var folders by remember { mutableStateOf<List<Entry>?>(null) }
    var conflicts by remember { mutableStateOf<MoveResult?>(null) }
    var saving by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val moving = remember(entries) { entries.map { it.id } }
    val destination = trail.lastOrNull()?.id

    LaunchedEffect(destination) {
        folders = null
        try {
            folders = client.list(destination)
                .filter { it.kind == EntryKind.FOLDER && it.id !in moving }
        } catch (_: CoreException.Unauthorized) {
            onSessionEnded()
        } catch (e: CoreException) {
            error = e.fileMessage()
        }
    }

    val move = { mode: MoveMode ->
        saving = true
        error = null
        scope.launch {
            try {
                val result = client.moveNodes(moving, destination, mode)
                if (result.conflicts.isEmpty()) {
                    onDone()
                } else {
                    conflicts = result
                    saving = false
                }
            } catch (_: CoreException.Unauthorized) {
                onSessionEnded()
            } catch (e: CoreException) {
                error = e.fileMessage()
                saving = false
            }
        }
        Unit
    }

    conflicts?.let { found ->
        ConflictsDialog(
            found = found,
            saving = saving,
            error = error,
            onCancel = {
                conflicts = null
                error = null
            },
            onResolve = move,
        )
        return
    }

    val title = entries.singleOrNull()?.let { "Move “${it.name}”" } ?: "Move ${entries.size} items"
    val where = if (trail.isEmpty()) "Home" else "/" + trail.joinToString("/") { it.name }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("To $where", style = MaterialTheme.typography.bodySmall)
                LazyColumn(modifier = Modifier.height(PICKER_HEIGHT)) {
                    if (trail.isNotEmpty()) {
                        item(key = "up") {
                            Text(
                                "Up",
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable { trail = trail.dropLast(1) }
                                    .padding(vertical = 8.dp),
                            )
                        }
                    }
                    items(folders.orEmpty(), key = { it.id }) { folder ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { trail = trail + Crumb(folder.id, folder.name) }
                                .padding(vertical = 8.dp),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            NodeIcon(EntryIcon.FOLDER)
                            Text(folder.name, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    if (folders?.isEmpty() == true) {
                        item(key = "empty") {
                            Text("No subfolders", modifier = Modifier.padding(vertical = 8.dp))
                        }
                    }
                }
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = {
            TextButton(
                enabled = !saving && !alreadyIn(entries, destination),
                onClick = { move(MoveMode.ABORT) },
            ) { Text(if (saving) WORKING else "Move here") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun describe(side: ConflictSide): String =
    "${localDate(side.updatedAt)} · ${side.sizeLabel}"

/** Files already at the destination; cancelling goes back to choosing another folder. */
@Composable
private fun ConflictsDialog(
    found: MoveResult,
    saving: Boolean,
    error: String?,
    onCancel: () -> Unit,
    onResolve: (MoveMode) -> Unit,
) {
    AlertDialog(
        onDismissRequest = onCancel,
        title = { Text(found.conflictsTitle) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Override replaces the current content and keeps it as a version.")
                LazyColumn(
                    modifier = Modifier.heightIn(max = PICKER_HEIGHT),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(found.conflicts, key = { it.path }) { conflict ->
                        Column {
                            Text(conflict.name)
                            val small = MaterialTheme.typography.bodySmall
                            Text(conflict.path, style = small)
                            Text("Incoming: ${describe(conflict.incoming)}", style = small)
                            val existing = conflict.existing?.let { describe(it) } ?: "Also selected"
                            Text("Existing: $existing", style = small)
                        }
                    }
                }
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = {
            TextButton(
                enabled = !saving,
                onClick = { onResolve(MoveMode.OVERRIDE) },
            ) { Text(if (saving) WORKING else "Override") }
        },
        dismissButton = {
            Row {
                TextButton(onClick = onCancel) { Text("Cancel") }
                TextButton(
                    enabled = !saving,
                    onClick = { onResolve(MoveMode.SKIP) },
                ) { Text("Skip these files") }
            }
        },
    )
}
