package app.homespace.android

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.SourceKind
import uniffi.homespace_core.SyncSettings
import uniffi.homespace_core.UploadSource

/** A local source picked, whose server folder is still to be chosen. */
private data class PickedSource(val kind: SourceKind, val handle: String, val label: String)

/** What the screen is asking the user. */
private sealed interface SyncDialog {
    data object AddSource : SyncDialog
    data class ChooseTarget(val source: PickedSource) : SyncDialog
    data class ChangeTarget(val source: UploadSource) : SyncDialog
    data class Remove(val source: UploadSource) : SyncDialog
    data object BatteryHint : SyncDialog
}

/** The two sync settings, and the local folders that upload to the server. */
@Composable
fun SyncSettingsScreen(session: Session.SignedIn, model: SessionViewModel) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val active = ClientHolder.active ?: return
    val auto = active.autoUpload
    val changes by ClientHolder.syncChanges.collectAsState()
    var refresh by remember { mutableStateOf(0) }
    var settings by remember { mutableStateOf(auto.settings()) }
    var dialog by remember { mutableStateOf<SyncDialog?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val sources = remember(changes, refresh) { auto.sources() }

    val pickFolder = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocumentTree(),
    ) { uri ->
        if (uri != null) {
            // Without this, the permission ends when the app does.
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
            val picked = PickedSource(SourceKind.FOLDER, uri.toString(), folderLabel(uri))
            dialog = SyncDialog.ChooseTarget(picked)
        }
    }
    val askPhotos = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { granted ->
        // Location is extra, and a photo-picker selection is a grant too.
        if (granted.any { (permission, ok) ->
                ok && permission != android.Manifest.permission.ACCESS_MEDIA_LOCATION
            }
        ) {
            val camera = PickedSource(SourceKind.PHOTOS, CAMERA_HANDLE, "Camera photos and videos")
            dialog = SyncDialog.ChooseTarget(camera)
        } else {
            error = "Home Space needs access to your photos to upload them."
        }
    }

    fun update(changed: SyncSettings) {
        auto.setSettings(changed)
        settings = changed
        applyBackgroundSetting(context, changed)
        model.syncNow()
    }

    val askNotifications = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) {
        // Uploads run without it too, only with no sign of them. So the answer changes nothing.
        update(settings.copy(background = true))
        dialog = SyncDialog.BatteryHint
    }

    Column(modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState())) {
        ListItem(
            headlineContent = { Text("Sync only on Wi-Fi") },
            supportingContent = { Text("Files don't sync over mobile data.") },
            trailingContent = {
                Switch(
                    checked = settings.wifiOnly,
                    onCheckedChange = { update(settings.copy(wifiOnly = it)) },
                )
            },
        )
        ListItem(
            headlineContent = { Text("Sync in background") },
            supportingContent = {
                Text("Keep syncing when the app isn't open. Android decides when this runs.")
            },
            trailingContent = {
                Switch(
                    checked = settings.background,
                    onCheckedChange = { on ->
                        when {
                            !on -> update(settings.copy(background = false))
                            needsNotificationPermission(context) ->
                                askNotifications.launch(android.Manifest.permission.POST_NOTIFICATIONS)
                            else -> {
                                update(settings.copy(background = true))
                                dialog = SyncDialog.BatteryHint
                            }
                        }
                    },
                )
            },
        )
        HorizontalDivider()
        Text(
            "Folders that upload to the server",
            style = MaterialTheme.typography.titleSmall,
            modifier = Modifier.padding(start = 16.dp, top = 16.dp, end = 16.dp),
        )
        Text(
            "New and changed files go up. Changes on the server never affect your phone.",
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )
        sources.forEach { source ->
            ListItem(
                headlineContent = { Text(source.label) },
                supportingContent = {
                    Column {
                        Text("To ${source.targetLabel}")
                        Text(
                            sourceStatus(source),
                            color = if (source.problem != null || source.lastError != null) {
                                MaterialTheme.colorScheme.error
                            } else {
                                MaterialTheme.colorScheme.onSurfaceVariant
                            },
                        )
                    }
                },
                trailingContent = {
                    Column(horizontalAlignment = Alignment.End) {
                        TextButton(onClick = { dialog = SyncDialog.ChangeTarget(source) }) {
                            Text("Change folder")
                        }
                        TextButton(onClick = { dialog = SyncDialog.Remove(source) }) {
                            Text("Remove")
                        }
                    }
                },
            )
        }
        if (model.waitingForWifi && sources.isNotEmpty()) {
            Text(
                "Waiting for Wi-Fi",
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
        }
        error?.let {
            Text(
                it,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
        }
        Row(
            modifier = Modifier.padding(16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Button(onClick = { dialog = SyncDialog.AddSource }) { Text("Add folder") }
            if (sources.isNotEmpty()) {
                OutlinedButton(onClick = { model.syncNow() }) { Text("Sync now") }
            }
        }
    }

    when (val current = dialog) {
        null -> Unit
        SyncDialog.AddSource -> AlertDialog(
            onDismissRequest = { dialog = null },
            title = { Text("What should upload?") },
            text = {
                Column {
                    TextButton(onClick = {
                        dialog = null
                        askPhotos.launch(cameraPermissions().toTypedArray())
                    }) { Text("Camera photos and videos") }
                    TextButton(onClick = {
                        dialog = null
                        pickFolder.launch(null)
                    }) { Text("Choose a folder") }
                }
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { dialog = null }) { Text("Cancel") } },
        )
        is SyncDialog.ChooseTarget -> ServerFolderDialog(
            title = "Upload “${current.source.label}” to",
            confirm = "Add",
            client = session.client,
            askUploadExisting = true,
            onDismiss = { dialog = null },
            onSessionEnded = model::onSessionEnded,
            onChoose = { folder, label, uploadExisting ->
                auto.addSource(
                    current.source.kind,
                    current.source.handle,
                    current.source.label,
                    folder,
                    label,
                    uploadExisting,
                )
                dialog = null
                refresh++
                model.syncNow()
            },
        )
        is SyncDialog.ChangeTarget -> ServerFolderDialog(
            title = "Upload “${current.source.label}” to",
            confirm = "Change",
            client = session.client,
            askUploadExisting = false,
            onDismiss = { dialog = null },
            onSessionEnded = model::onSessionEnded,
            onChoose = { folder, label, _ ->
                auto.setSourceTarget(current.source.id, folder, label)
                dialog = null
                refresh++
                model.syncNow()
            },
        )
        is SyncDialog.Remove -> AlertDialog(
            onDismissRequest = { dialog = null },
            title = { Text("Stop uploading “${current.source.label}”?") },
            text = { Text("Files already uploaded stay on the server. Your phone isn't changed.") },
            confirmButton = {
                TextButton(onClick = {
                    dialog = null
                    scope.launch {
                        try {
                            auto.removeSource(current.source.id)
                            releaseFolderPermission(context, current.source)
                        } catch (e: CoreException) {
                            error = e.fileMessage()
                        }
                        refresh++
                        model.syncNow()
                    }
                }) { Text("Stop uploading") }
            },
            dismissButton = { TextButton(onClick = { dialog = null }) { Text("Cancel") } },
        )
        SyncDialog.BatteryHint -> AlertDialog(
            onDismissRequest = { dialog = null },
            title = { Text("Sync in background") },
            text = {
                Text(
                    "Android may delay background uploads to save battery. For more regular " +
                        "uploads, set Home Space's battery use to Unrestricted.",
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    dialog = null
                    context.startActivity(appBatterySettings(context))
                }) { Text("Open settings") }
            },
            dismissButton = { TextButton(onClick = { dialog = null }) { Text("Not now") } },
        )
    }

    LaunchedEffect(Unit) { model.syncNow() }
}

@Composable
private fun sourceStatus(source: UploadSource): String = when {
    source.problem != null -> source.problem!!
    source.lastError != null -> "${source.pending} to go. ${source.lastError}"
    source.pending > 0u -> "${source.pending} to go"
    else -> source.lastSyncedAt?.let { "Up to date, ${localDate(it)}" } ?: "Waiting to sync"
}

private fun needsNotificationPermission(context: Context): Boolean =
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
        context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) !=
        PackageManager.PERMISSION_GRANTED

private fun appBatterySettings(context: Context): Intent =
    // The per-app page, not the prompt for an exemption, which Google Play restricts.
    Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${context.packageName}"))

private fun folderLabel(tree: Uri): String =
    DocumentsContract.getTreeDocumentId(tree).substringAfterLast(':').substringAfterLast('/')
        .ifEmpty { "Folder" }

private fun releaseFolderPermission(context: Context, source: UploadSource) {
    if (source.kind == SourceKind.FOLDER) {
        runCatching {
            context.contentResolver.releasePersistableUriPermission(
                Uri.parse(source.handle),
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        }
    }
}
