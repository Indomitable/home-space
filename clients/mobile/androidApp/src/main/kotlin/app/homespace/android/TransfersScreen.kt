package app.homespace.android

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import uniffi.homespace_core.OfflineStatus
import uniffi.homespace_core.QueuedUpload
import uniffi.homespace_core.QueuedUploadState
import uniffi.homespace_core.UploadSource

/** Everything on its way between this phone and the server: picked files, then what syncs by itself. */
@Composable
fun TransfersScreen(session: Session.SignedIn, model: SessionViewModel, onOpenSync: () -> Unit) {
    val active = ClientHolder.active ?: return
    val changes by ClientHolder.syncChanges.collectAsState()
    val sources = remember(changes) { runCatching { active.autoUpload.sources() }.getOrDefault(emptyList()) }
    val current = remember(changes) { active.autoUpload.current() }
    val offline by produceState<OfflineStatus?>(null, changes) {
        value = runCatching { session.client.offlineStatus() }.getOrNull()
    }
    val nothing = model.uploads.isEmpty() && sources.isEmpty() && (offline?.kept ?: 0uL) == 0uL
    LazyColumn(contentPadding = PaddingValues(bottom = 16.dp)) {
        if (nothing) {
            item {
                ListItem(
                    headlineContent = { Text("No transfers") },
                    supportingContent = { Text("Files you upload and folders that sync show up here.") },
                )
            }
        }
        if (model.uploads.isNotEmpty()) {
            item { SectionTitle("Uploads") }
            items(model.uploads, key = { it.key }) { upload ->
                UploadRow(
                    upload = upload,
                    onRetry = { model.retryUpload(upload) },
                    onCancel = { model.cancelUpload(upload) },
                )
                HorizontalDivider()
            }
        }
        if (sources.isNotEmpty()) {
            item { SectionTitle("Auto-upload") }
            items(sources, key = { it.id }) { source ->
                SourceRow(source, current?.takeIf { it.sourceId == source.id }, onOpenSync)
            }
        }
        offline?.takeIf { it.kept > 0uL }?.let { status ->
            item { SectionTitle("Offline copies") }
            item {
                val problems = status.problems.isNotEmpty()
                ListItem(
                    headlineContent = { Text(offlineTitle(status)) },
                    supportingContent = {
                        Text(
                            offlineDetail(status),
                            color = if (problems) MaterialTheme.colorScheme.error else Color.Unspecified,
                        )
                    },
                    modifier = Modifier.clickable(onClick = onOpenSync),
                )
            }
        }
    }
}

@Composable
private fun SectionTitle(title: String) {
    Text(
        title,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 16.dp, top = 16.dp, end = 16.dp, bottom = 4.dp),
    )
}

@Composable
private fun SourceRow(source: UploadSource, current: CurrentUpload?, onOpenSync: () -> Unit) {
    val failing = source.problem != null || source.lastError != null
    val progress = current?.takeIf { it.total > 0u }?.let { it.done.toFloat() / it.total.toFloat() }
    ListItem(
        headlineContent = { Text(source.label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(
                current?.let { "Uploading ${it.name}${percentOf(progress)}" } ?: sourceStatus(source),
                color = if (current == null && failing) MaterialTheme.colorScheme.error else Color.Unspecified,
            )
        },
        // Clear, or the list item's own background would hide the fill.
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        modifier = Modifier
            .progressFill(progress, accentColor().copy(alpha = PROGRESS_ALPHA))
            .clickable(onClick = onOpenSync),
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

/** A picked file on its way up. */
@Composable
private fun UploadRow(upload: QueuedUpload, onRetry: () -> Unit, onCancel: () -> Unit) {
    val failed = upload.state == QueuedUploadState.FAILED
    val progress = if (upload.total > 0u && upload.done > 0u && !failed) {
        (upload.done.toFloat() / upload.total.toFloat()).coerceAtMost(1f)
    } else {
        null
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .progressFill(progress, accentColor().copy(alpha = PROGRESS_ALPHA))
            .heightIn(min = ROW_MIN_HEIGHT)
            .padding(start = 16.dp, end = 4.dp, top = 2.dp, bottom = 2.dp),
        horizontalArrangement = Arrangement.spacedBy(ICON_GAP),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(modifier = Modifier.size(ICON_BOX), contentAlignment = Alignment.CenterStart) {
            Icon(
                painterResource(R.drawable.ic_upload_file),
                contentDescription = null,
                modifier = Modifier.size(ICON_SIZE),
            )
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(upload.name, maxLines = 1, overflow = TextOverflow.Ellipsis)
            val folder = upload.folderName
            val percent = percentOf(progress)
            val status = when (upload.state) {
                QueuedUploadState.WAITING -> "Waiting to upload to $folder$percent"
                QueuedUploadState.PREPARING -> "Preparing to upload to $folder"
                QueuedUploadState.UPLOADING -> "Uploading to $folder$percent"
                QueuedUploadState.OFFLINE -> "Waiting for the connection to upload to $folder$percent"
                QueuedUploadState.FAILED -> upload.problem ?: "This file couldn't be uploaded"
            }
            Text(
                status,
                style = MaterialTheme.typography.bodySmall,
                color = if (failed) MaterialTheme.colorScheme.error else Color.Unspecified,
            )
        }
        if (upload.state == QueuedUploadState.OFFLINE || (failed && upload.canRetry)) {
            TextButton(onClick = onRetry) { Text("Retry") }
        }
        IconButton(onClick = onCancel, modifier = Modifier.size(MENU_BUTTON_SIZE)) {
            Icon(painterResource(R.drawable.ic_close), contentDescription = "Cancel upload of ${upload.name}")
        }
    }
}
