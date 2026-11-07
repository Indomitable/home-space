package app.homespace.android

import android.webkit.MimeTypeMap
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.homespace_core.CurrentUpload
import uniffi.homespace_core.EntryIcon
import uniffi.homespace_core.OfflineStatus
import uniffi.homespace_core.QueuedUpload
import uniffi.homespace_core.QueuedUploadState
import uniffi.homespace_core.SourceKind
import uniffi.homespace_core.UploadSource

private const val MIME_IMAGE = "image/"
private const val MIME_TEXT = "text/"
private const val MIME_PDF = "application/pdf"
/** A row with Retry or Continue beside it has little width, so its status may wrap. */
private const val ACTION_ROW_MAX_LINES = 2

/** Everything on its way between this phone and the server: picked files, then what syncs by itself. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransfersScreen(
    session: Session.SignedIn,
    model: SessionViewModel,
    navigationBar: @Composable () -> Unit,
    onOpenPage: (SubPage) -> Unit,
) {
    val active = ClientHolder.active ?: return
    val changes by ClientHolder.syncChanges.collectAsState()
    val sources = remember(changes) { runCatching { active.autoUpload.sources() }.getOrDefault(emptyList()) }
    val current = remember(changes) { active.autoUpload.current() }
    val offline by produceState<OfflineStatus?>(null, changes) {
        value = runCatching { session.client.offlineStatus() }.getOrNull()
    }
    val nothing = model.uploads.isEmpty() && sources.isEmpty() && (offline?.kept ?: 0uL) == 0uL
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transfers") },
                actions = {
                    IconButton(onClick = model::syncNow) {
                        Icon(painterResource(R.drawable.ic_sync), contentDescription = "Sync now")
                    }
                },
            )
        },
        bottomBar = navigationBar,
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(bottom = 16.dp),
        ) {
            if (nothing) {
                item {
                    ListItem(
                        headlineContent = { Text("No transfers") },
                        supportingContent = { Text("Files you upload and folders that sync show up here.") },
                    )
                }
            }
            if (model.uploads.isNotEmpty()) {
                item { SectionHeader("Uploads") }
                items(model.uploads, key = { it.key }) { upload ->
                    UploadRow(
                        upload = upload,
                        onRetry = { model.retryUpload(upload) },
                        onCancel = { model.cancelUpload(upload) },
                    )
                }
            }
            if (sources.isNotEmpty()) {
                item { SectionHeader("Auto-upload") }
                items(sources, key = { it.id }) { source ->
                    SourceRow(source, current?.takeIf { it.sourceId == source.id }) {
                        onOpenPage(SubPage.UPLOAD_FOLDERS)
                    }
                }
            }
            offline?.takeIf { it.kept > 0uL }?.let { status ->
                item { SectionHeader("Offline copies") }
                item {
                    val problems = status.problems.isNotEmpty()
                    ListItem(
                        leadingContent = {
                            IconTile(
                                R.drawable.ic_offline_pin,
                                MaterialTheme.colorScheme.surfaceContainerHigh,
                                MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        },
                        headlineContent = { Text(offlineTitle(status)) },
                        supportingContent = {
                            Text(
                                offlineDetail(status),
                                color = if (problems) MaterialTheme.colorScheme.error else Color.Unspecified,
                            )
                        },
                        trailingContent = { Chevron() },
                        modifier = Modifier.clickable { onOpenPage(SubPage.OFFLINE_FILES) },
                    )
                }
            }
        }
    }
}

/** A folder that uploads by itself: the tile of its kind, used for nothing else. */
@Composable
internal fun SourceTile(kind: SourceKind) {
    IconTile(
        if (kind == SourceKind.PHOTOS) R.drawable.ic_photo_camera else R.drawable.ic_folder_open,
        MaterialTheme.colorScheme.tertiaryContainer,
        MaterialTheme.colorScheme.onTertiaryContainer,
    )
}

/** Syncing or done; a failure has no chip, since the status line in `error` says it already. */
@Composable
internal fun SourceChip(source: UploadSource, current: CurrentUpload?) {
    when {
        source.problem != null || source.lastError != null -> Unit
        current != null || source.pending > 0u -> StatusChip("Syncing", ChipTone.NEUTRAL)
        source.lastSyncedAt != null -> StatusChip("Done", ChipTone.SUCCESS, R.drawable.ic_check_circle)
    }
}

internal fun currentProgress(current: CurrentUpload?): Float? =
    current?.takeIf { it.total > 0u }?.let { it.done.toFloat() / it.total.toFloat() }

@Composable
private fun SourceRow(source: UploadSource, current: CurrentUpload?, onOpen: () -> Unit) {
    val failing = source.problem != null || source.lastError != null
    val progress = currentProgress(current)
    ListItem(
        leadingContent = { SourceTile(source.kind) },
        headlineContent = { Text(source.label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(
                current?.let { "Uploading ${it.name}${percentOf(progress)}" } ?: sourceStatus(source),
                style = MaterialTheme.typography.bodyMedium.tabular(),
                color = if (current == null && failing) MaterialTheme.colorScheme.error else Color.Unspecified,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
        trailingContent = { SourceChip(source, current) },
        modifier = Modifier.clickable(onClick = onOpen),
    )
}

private fun percentOf(progress: Float?): String =
    progress?.let { " · ${(it * PERCENT).toInt()}%" }.orEmpty()

private fun offlineTitle(status: OfflineStatus): String =
    if (status.kept == 1uL) "1 item on this phone" else "${status.kept} items on this phone"

private fun offlineDetail(status: OfflineStatus): String = when {
    status.problems.isNotEmpty() -> "${status.problems.size} couldn't sync"
    status.unreachable -> "Waiting for the connection"
    status.pending > 0uL -> "${status.pending} updating"
    else -> "Up to date"
}

/** The tile for a file known only by its name, since it isn't on the server yet. */
private fun iconOfName(name: String): EntryIcon {
    val mime = MimeTypeMap.getSingleton()
        .getMimeTypeFromExtension(name.substringAfterLast('.', "").lowercase())
        .orEmpty()
    return when {
        mime.startsWith(MIME_IMAGE) -> EntryIcon.IMAGE
        mime.startsWith(MIME_TEXT) || mime == MIME_PDF -> EntryIcon.DOCUMENT
        else -> EntryIcon.OTHER
    }
}

/** A picked file on its way up. */
@Composable
private fun UploadRow(upload: QueuedUpload, onRetry: () -> Unit, onCancel: () -> Unit) {
    val failed = upload.state == QueuedUploadState.FAILED
    val progress = if (upload.total > 0u && upload.done > 0u && !failed) {
        (upload.done.toFloat() / upload.total.toFloat()).coerceAtMost(1f)
    } else {
        null
    }
    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = ROW_MIN_HEIGHT)
                .padding(start = 16.dp, end = 4.dp, top = 8.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (failed) {
                IconTile(
                    R.drawable.ic_error_outline,
                    MaterialTheme.colorScheme.errorContainer,
                    MaterialTheme.colorScheme.onErrorContainer,
                )
            } else {
                NodeTile(iconOfName(upload.name))
            }
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    upload.name,
                    style = MaterialTheme.typography.bodyLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                val folder = upload.folderName
                val percent = percentOf(progress)
                val status = when (upload.state) {
                    QueuedUploadState.WAITING -> "Waiting to upload to $folder$percent"
                    QueuedUploadState.PREPARING -> "Preparing to upload to $folder"
                    QueuedUploadState.UPLOADING -> "Uploading to $folder$percent"
                    QueuedUploadState.OFFLINE -> "Waiting for the connection to upload to $folder$percent"
                    QueuedUploadState.PAUSED -> "Paused upload to $folder$percent"
                    QueuedUploadState.FAILED -> upload.problem ?: "This file couldn't be uploaded"
                }
                val hasAction = failed || upload.state == QueuedUploadState.OFFLINE || upload.state == QueuedUploadState.PAUSED
                Text(
                    status,
                    style = MaterialTheme.typography.bodyMedium.tabular(),
                    color = if (failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = if (hasAction) ACTION_ROW_MAX_LINES else 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                val paused = upload.state == QueuedUploadState.PAUSED
                if (upload.state == QueuedUploadState.OFFLINE || paused || (failed && upload.canRetry)) {
                    TextButton(onClick = onRetry) { Text(if (paused) "Continue" else "Retry") }
                }
                IconButton(onClick = onCancel) {
                    Icon(painterResource(R.drawable.ic_close), contentDescription = "Cancel upload of ${upload.name}")
                }
            }
        }
        if (progress != null) {
            ProgressLine(
                progress,
                modifier = Modifier.fillMaxWidth().padding(start = ROW_TEXT_START, end = 16.dp, bottom = 4.dp),
            )
        }
    }
}
