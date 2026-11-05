package app.homespace.android

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.text.format.DateUtils
import android.webkit.MimeTypeMap
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContract
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import java.io.File
import java.time.Instant
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.Entry
import uniffi.homespace_core.EntryIcon
import uniffi.homespace_core.EntryKind

private val ICON_SIZE = 20.dp
private val ICON_GAP = 8.dp
// Room for the badge in the icon's corner, as in the desktop client.
private val ICON_BOX = ICON_SIZE + 6.dp
private val BADGE_SIZE = 12.dp
private val BADGE_PADDING = 1.dp
private val MENU_BUTTON_SIZE = 40.dp

/** The same as the iOS app's rows, which match the Files app. */
private val ROW_MIN_HEIGHT = 64.dp
private const val PROGRESS_ALPHA = 0.25f
private const val OCTET_STREAM = "application/octet-stream"

// The web client's --color-accent.
private val FOLDER_ACCENT_LIGHT = Color(0xFF2563EB)
private val FOLDER_ACCENT_DARK = Color(0xFF6D9EFF)
// The web client's --color-success.
private val SUCCESS_LIGHT = Color(0xFF15803D)
private val SUCCESS_DARK = Color(0xFF5FD39A)

/** What a row's menu can ask for. */
private enum class RowAction { SAVE, DETAILS, RENAME, MOVE, DELETE, REMOVE_DOWNLOAD, CANCEL_DOWNLOAD }

/** Folder browsing through the Rust client (`clients/mobile-core`). */
@Composable
fun BrowseScreen(session: Session.SignedIn, model: SessionViewModel, modifier: Modifier = Modifier) {
    val path = model.path
    val entries = remember { mutableStateListOf<Entry>() }
    var error by remember { mutableStateOf<String?>(null) }
    // The ids of the selected rows; a long press starts selecting, and it ends once this is empty.
    var selected by remember { mutableStateOf(emptySet<String>()) }
    val selecting = selected.isNotEmpty()
    var action by remember { mutableStateOf<FileAction?>(null) }
    val context = LocalContext.current
    val saveTo = rememberLauncherForActivityResult(SaveDocument()) { destination ->
        val entry = model.pendingSave
        model.pendingSave = null
        if (entry != null && destination != null) model.saveFile(session, entry, destination)
    }

    val lifecycle = LocalLifecycleOwner.current.lifecycle
    DisposableEffect(lifecycle) {
        val observer = LifecycleEventObserver { _, _ ->
            model.foreground = lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)
        }
        lifecycle.addObserver(observer)
        onDispose {
            lifecycle.removeObserver(observer)
            model.foreground = false
        }
    }

    model.pendingOpen?.let { request ->
        LaunchedEffect(request) {
            if (!openInViewer(context, request)) {
                Toast.makeText(context, "No app can open this file", Toast.LENGTH_LONG).show()
            }
            model.onOpened()
        }
    }

    BackHandler(enabled = path.size > 1) { path.removeAt(path.lastIndex) }
    // Declared last so it goes first: back leaves the selection before it leaves the folder.
    BackHandler(enabled = selecting) { selected = emptySet() }

    LaunchedEffect(path.lastOrNull()) { selected = emptySet() }

    LaunchedEffect(session.client, path.lastOrNull(), model.reloadKey) {
        val folder = path.lastOrNull() ?: return@LaunchedEffect
        error = null
        try {
            val listed = session.client.list(folder.id)
            entries.clear()
            entries.addAll(listed)
            // A file that is gone can't stay selected.
            val present = listed.map { it.id }.toSet()
            selected = selected intersect present
        } catch (_: CoreException.Unauthorized) {
            model.onSessionEnded()
        } catch (e: CoreException) {
            error = e.fileMessage()
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
        if (selecting) {
            val chosen = entries.filter { it.id in selected }
            SelectionActions(
                chosen = chosen,
                canDelete = chosen.none { it.id == session.user.lostAndFoundId },
                onAction = { action = it },
            )
        }
        CurrentFolder(path, onOpenParent = { index -> path.removeRange(index + 1, path.size) })
        error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        LazyColumn {
            items(entries, key = { it.id }) { entry ->
                val toggle = {
                    selected = if (entry.id in selected) selected - entry.id else selected + entry.id
                }
                EntryRow(
                    entry = entry,
                    selecting = selecting,
                    isSelected = entry.id in selected,
                    progress = model.downloads[entry.id],
                    canDelete = entry.id != session.user.lostAndFoundId,
                    onClick = {
                        when {
                            selecting -> toggle()
                            entry.kind == EntryKind.FOLDER -> path.add(Crumb(entry.id, entry.name))
                            else -> model.openFile(session, entry)
                        }
                    },
                    onLongClick = toggle,
                    onAction = { rowAction ->
                        when (rowAction) {
                            RowAction.SAVE -> {
                                model.pendingSave = entry
                                saveTo.launch(entry)
                            }
                            RowAction.DETAILS -> action = FileAction.Details(entry)
                            RowAction.RENAME -> action = FileAction.Rename(entry)
                            RowAction.MOVE -> action = FileAction.Move(listOf(entry))
                            RowAction.DELETE -> action = FileAction.Delete(listOf(entry))
                            RowAction.REMOVE_DOWNLOAD -> model.removeLocalCopy(session, entry)
                            RowAction.CANCEL_DOWNLOAD -> session.client.cancelDownload(entry.id)
                        }
                    },
                )
                // From the text to the end of the row, as in the iOS app.
                HorizontalDivider(modifier = Modifier.padding(start = ICON_BOX + ICON_GAP))
            }
        }
    }

    action?.let { current ->
        FileDialog(
            action = current,
            client = session.client,
            onDismiss = { action = null },
            onDone = {
                action = null
                selected = emptySet()
                model.reload()
            },
            onSessionEnded = model::onSessionEnded,
        )
    }
}

/** What can be done with the selected rows. */
@Composable
private fun SelectionActions(
    chosen: List<Entry>,
    canDelete: Boolean,
    onAction: (FileAction) -> Unit,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        chosen.singleOrNull()?.let { entry ->
            TextButton(onClick = { onAction(FileAction.Rename(entry)) }) { Text("Rename") }
            TextButton(onClick = { onAction(FileAction.Details(entry)) }) { Text("Details") }
        }
        TextButton(onClick = { onAction(FileAction.Move(chosen)) }) { Text("Move") }
        if (canDelete) {
            TextButton(onClick = { onAction(FileAction.Delete(chosen)) }) { Text("Delete") }
        }
    }
}

/** The folder being browsed; tapping it lists the folders above it, to jump back to any of them. */
@Composable
private fun CurrentFolder(path: List<Crumb>, onOpenParent: (Int) -> Unit) {
    val parents = path.dropLast(1)
    var expanded by remember { mutableStateOf(false) }
    Box {
        Row(
            modifier = Modifier
                .clickable(
                    enabled = parents.isNotEmpty(),
                    onClickLabel = "Show parent folders",
                ) { expanded = true }
                .padding(vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                path.lastOrNull()?.name.orEmpty(),
                style = MaterialTheme.typography.titleMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f, fill = false),
            )
            if (parents.isNotEmpty()) {
                Icon(painterResource(R.drawable.ic_arrow_drop_down), contentDescription = null)
            }
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            parents.forEachIndexed { index, crumb ->
                DropdownMenuItem(
                    text = { Text(crumb.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    leadingIcon = { NodeIcon(EntryIcon.FOLDER) },
                    onClick = {
                        expanded = false
                        onOpenParent(index)
                    },
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
    progress: Float?,
    canDelete: Boolean,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
    onAction: (RowAction) -> Unit,
) {
    val background =
        if (isSelected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent
    val fill = accentColor().copy(alpha = PROGRESS_ALPHA)
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(background)
            .drawBehind {
                if (progress != null) {
                    val width = size.width * progress
                    val left = if (layoutDirection == LayoutDirection.Rtl) size.width - width else 0f
                    drawRect(fill, topLeft = Offset(left, 0f), size = Size(width, size.height))
                }
            }
            .combinedClickable(onLongClick = onLongClick, onClick = onClick)
            .heightIn(min = ROW_MIN_HEIGHT)
            .padding(vertical = 2.dp),
        horizontalArrangement = Arrangement.spacedBy(ICON_GAP),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // The whole row takes the tap, so the box only shows the state.
        if (selecting) Checkbox(checked = isSelected, onCheckedChange = null)
        BadgedIcon(entry.icon, downloaded = entry.localPath != null && progress == null)
        Column(modifier = Modifier.weight(1f)) {
            Text(entry.name)
            if (entry.kind == EntryKind.FILE) {
                Text(
                    "${entry.sizeLabel} · ${localDate(entry.updatedAt)}",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
        if (!selecting) {
            RowMenu(entry, downloading = progress != null, canDelete, onAction)
        }
    }
}

@Composable
private fun RowMenu(
    entry: Entry,
    downloading: Boolean,
    canDelete: Boolean,
    onAction: (RowAction) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    val choose = { action: RowAction ->
        expanded = false
        onAction(action)
    }
    Box {
        IconButton(onClick = { expanded = true }, modifier = Modifier.size(MENU_BUTTON_SIZE)) {
            Icon(
                painterResource(R.drawable.ic_more_vert),
                contentDescription = "Actions for ${entry.name}",
            )
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            val item = @Composable { label: String, icon: Int, action: RowAction ->
                DropdownMenuItem(
                    text = { Text(label) },
                    leadingIcon = { Icon(painterResource(icon), contentDescription = null) },
                    onClick = { choose(action) },
                )
            }
            if (downloading) item("Cancel download", R.drawable.ic_close, RowAction.CANCEL_DOWNLOAD)
            if (entry.kind == EntryKind.FILE && !downloading) {
                item("Save to…", R.drawable.ic_download, RowAction.SAVE)
            }
            item("Details", R.drawable.ic_info, RowAction.DETAILS)
            item("Rename", R.drawable.ic_edit, RowAction.RENAME)
            item("Move", R.drawable.ic_drive_file_move, RowAction.MOVE)
            if (entry.localPath != null && !downloading) {
                item("Remove download", R.drawable.ic_download_off, RowAction.REMOVE_DOWNLOAD)
            }
            if (canDelete) item("Delete", R.drawable.ic_delete, RowAction.DELETE)
        }
    }
}

/** A time in the device's locale, time zone and 12 or 24 hour setting. */
@Composable
internal fun localDate(time: Instant): String = DateUtils.formatDateTime(
    LocalContext.current,
    time.toEpochMilli(),
    DateUtils.FORMAT_SHOW_DATE or DateUtils.FORMAT_SHOW_TIME or
        DateUtils.FORMAT_SHOW_YEAR or DateUtils.FORMAT_ABBREV_MONTH,
)

/** A node's icon, with the desktop client's badge in the corner once the file is on the phone. */
@Composable
private fun BadgedIcon(icon: EntryIcon, downloaded: Boolean) {
    Box(modifier = Modifier.size(ICON_BOX), contentAlignment = Alignment.CenterStart) {
        NodeIcon(icon)
        if (downloaded) {
            Box(
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .background(MaterialTheme.colorScheme.surface, CircleShape)
                    .padding(BADGE_PADDING),
            ) {
                Icon(
                    painterResource(R.drawable.ic_check_circle),
                    contentDescription = "Downloaded",
                    tint = if (isSystemInDarkTheme()) SUCCESS_DARK else SUCCESS_LIGHT,
                    modifier = Modifier.size(BADGE_SIZE),
                )
            }
        }
    }
}

@Composable
private fun accentColor(): Color =
    if (isSystemInDarkTheme()) FOLDER_ACCENT_DARK else FOLDER_ACCENT_LIGHT

/** The same icons and colours as the web client: only folders are coloured. */
@Composable
internal fun NodeIcon(icon: EntryIcon) {
    val drawable = when (icon) {
        EntryIcon.FOLDER -> R.drawable.ic_node_folder
        EntryIcon.IMAGE -> R.drawable.ic_node_image
        EntryIcon.DOCUMENT -> R.drawable.ic_node_description
        EntryIcon.OTHER -> R.drawable.ic_node_draft
    }
    val tint = if (icon == EntryIcon.FOLDER) accentColor() else MaterialTheme.colorScheme.onSurface
    Icon(
        painterResource(drawable),
        contentDescription = null,
        tint = tint,
        modifier = Modifier.size(ICON_SIZE),
    )
}

/** Asks the system where to save a file, offering its name and type. */
private class SaveDocument : ActivityResultContract<Entry, Uri?>() {
    override fun createIntent(context: Context, input: Entry): Intent =
        Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType(input.mime.ifBlank { OCTET_STREAM })
            .putExtra(Intent.EXTRA_TITLE, input.name)

    override fun parseResult(resultCode: Int, intent: Intent?): Uri? =
        intent?.data.takeIf { resultCode == android.app.Activity.RESULT_OK }
}

/** Hands the file to whichever app shows its type, read-only; false when none does. */
private fun openInViewer(context: Context, request: OpenRequest): Boolean {
    val file = File(request.path)
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
    val type = request.mime.takeUnless { it.isBlank() || it == OCTET_STREAM }
        ?: MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase())
        ?: "*/*"
    val intent = Intent(Intent.ACTION_VIEW)
        .setDataAndType(uri, type)
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    return try {
        context.startActivity(intent)
        true
    } catch (_: ActivityNotFoundException) {
        false
    }
}
