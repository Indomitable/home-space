package app.homespace.android

import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import android.provider.MediaStore
import android.system.ErrnoException
import android.system.OsConstants
import java.io.File
import java.io.FileInputStream
import java.io.FileNotFoundException
import java.io.IOException
import java.nio.ByteBuffer
import java.time.Instant
import uniffi.homespace_core.CoreException
import org.json.JSONException
import org.json.JSONObject
import uniffi.homespace_core.LocalChanges
import uniffi.homespace_core.LocalItem
import uniffi.homespace_core.LocalSources
import uniffi.homespace_core.PickedFiles
import uniffi.homespace_core.SourceKind

/** Reads the files picked to upload, by their content URIs. */
class AndroidPickedFiles(context: Context) : PickedFiles {
    private val resolver = context.applicationContext.contentResolver

    override fun copyFile(source: String, dest: String) {
        try {
            val input = resolver.openInputStream(Uri.parse(source)) ?: throw FileNotFoundException(source)
            input.use { from -> File(dest).outputStream().use { to -> from.copyTo(to) } }
        } catch (_: SecurityException) {
            throw CoreException.Failed("file", "Home Space can no longer read this file. Pick it again.")
        } catch (_: FileNotFoundException) {
            throw CoreException.Failed("file", "This file can no longer be found. Pick it again.")
        } catch (e: IOException) {
            val full = (e.cause as? ErrnoException)?.errno == OsConstants.ENOSPC
            throw CoreException.Failed(
                "file",
                if (full) "There isn't enough free space on this phone to prepare this file." else
                    "The file can't be read: ${e.message}",
            )
        }
    }

    override fun release(source: String) {
        try {
            resolver.releasePersistableUriPermission(Uri.parse(source), Intent.FLAG_GRANT_READ_URI_PERMISSION)
        } catch (_: SecurityException) {
            // It was never kept, since not every provider offers it.
        }
    }
}

/** The handle of the camera source, which has only one place to look. */
const val CAMERA_HANDLE = "camera"

private const val CAMERA_FOLDER = "DCIM/"
private const val IMAGE_PREFIX = "image:"
private const val VIDEO_PREFIX = "video:"
private const val MILLIS_PER_SECOND = 1000L
private const val TOKEN_VERSION = "version"
private const val TOKEN_GENERATION = "generation"

private fun failed(detail: String) = CoreException.Failed("local", detail)

/**
 * The phone's files, for auto-upload. A folder is a document tree the user picked, whose
 * permission the app keeps. The camera source is MediaStore's photos and videos under `DCIM/`.
 * Rust calls these on a background thread. Anything other than a `CoreException` would reach it
 * as an unexpected error, so failures are turned into one here.
 */
class AndroidSources(context: Context) : LocalSources {
    private val appContext = context.applicationContext
    private val resolver: ContentResolver = appContext.contentResolver

    override fun list(kind: SourceKind, handle: String): List<LocalItem> = guarded {
        when (kind) {
            SourceKind.FOLDER -> listTree(Uri.parse(handle))
            SourceKind.PHOTOS -> listCamera()
        }
    }

    override fun changeToken(kind: SourceKind, handle: String): String? = guarded {
        if (kind == SourceKind.PHOTOS) cameraToken()?.toString() else null
    }

    override fun changesSince(kind: SourceKind, handle: String, token: String): LocalChanges? = guarded {
        if (kind == SourceKind.PHOTOS) cameraChanges(token) else null
    }

    override fun stat(kind: SourceKind, handle: String, key: String): LocalItem? = guarded {
        when (kind) {
            SourceKind.FOLDER -> statDocument(Uri.parse(handle), key)
            SourceKind.PHOTOS -> statMedia(key)
        }
    }

    override fun read(
        kind: SourceKind,
        handle: String,
        key: String,
        offset: ULong,
        len: ULong,
    ): ByteArray = guarded {
        val uri = when (kind) {
            SourceKind.FOLDER -> DocumentsContract.buildDocumentUriUsingTree(Uri.parse(handle), key)
            SourceKind.PHOTOS -> originalOf(mediaUri(key))
        }
        val descriptor = resolver.openFileDescriptor(uri, "r")
            ?: throw failed("The file can't be opened")
        descriptor.use {
            FileInputStream(it.fileDescriptor).use { stream ->
                val channel = stream.channel
                channel.position(offset.toLong())
                val buffer = ByteBuffer.allocate(len.toInt())
                while (buffer.hasRemaining() && channel.read(buffer) >= 0) {
                    // Keep reading: a short read isn't the end of the file.
                }
                buffer.array().copyOf(buffer.position())
            }
        }
    }

    private inline fun <T> guarded(work: () -> T): T = try {
        work()
    } catch (e: CoreException) {
        throw e
    } catch (e: SecurityException) {
        throw failed("Home Space no longer has access. Add the folder again. (${e.message})")
    } catch (e: IOException) {
        throw failed("The file can't be read: ${e.message}")
    } catch (e: IllegalArgumentException) {
        throw failed("The file can't be found: ${e.message}")
    }

    private fun listTree(tree: Uri): List<LocalItem> {
        val items = mutableListOf<LocalItem>()
        // Folders still to read, with their path below the source.
        val pending = ArrayDeque(listOf(DocumentsContract.getTreeDocumentId(tree) to ""))
        while (pending.isNotEmpty()) {
            val (parent, dir) = pending.removeFirst()
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parent)
            val cursor = resolver.query(children, DOCUMENT_COLUMNS, null, null, null)
                ?: throw failed("The folder can't be read")
            cursor.use {
                while (it.moveToNext()) {
                    val id = it.getString(0)
                    val name = it.getString(1) ?: continue
                    if (name.startsWith(".")) continue
                    if (it.getString(2) == Document.MIME_TYPE_DIR) {
                        pending.add(id to if (dir.isEmpty()) name else "$dir/$name")
                    } else {
                        items += documentItem(tree, it, dir)
                    }
                }
            }
        }
        return items
    }

    private fun statDocument(tree: Uri, key: String): LocalItem? {
        val uri = DocumentsContract.buildDocumentUriUsingTree(tree, key)
        return try {
            resolver.query(uri, DOCUMENT_COLUMNS, null, null, null)?.use {
                if (it.moveToFirst()) documentItem(tree, it, "") else null
            }
        } catch (_: IllegalArgumentException) {
            null // Some providers answer this way for a document that is gone.
        }
    }

    private fun documentItem(tree: Uri, row: Cursor, dir: String): LocalItem {
        val id = row.getString(0)
        val size = if (row.isNull(3)) statSize(tree, id) else row.getLong(3)
        return LocalItem(
            key = id,
            relativeDir = dir,
            name = row.getString(1),
            size = size.toULong(),
            modifiedAt = Instant.ofEpochMilli(row.getLong(4)),
            mime = row.getString(2),
        )
    }

    private fun statSize(tree: Uri, id: String): Long =
        resolver.openFileDescriptor(DocumentsContract.buildDocumentUriUsingTree(tree, id), "r")
            ?.use { it.statSize } ?: 0L

    private fun listCamera(): List<LocalItem> {
        if (!hasMediaAccess()) throw failed("Home Space no longer has access to your photos.")
        return listMedia(IMAGE_PREFIX, imagesUri()) + listMedia(VIDEO_PREFIX, videosUri())
    }

    /**
     * Each volume's database version and generation, or null if the changes can't be told: before
     * Android 11, or with access to only the photos the user selected, since a photo added to that
     * selection hasn't changed and wouldn't be reported.
     */
    private fun cameraToken(): JSONObject? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R || !hasFullMediaAccess()) return null
        val token = JSONObject()
        for (volume in MediaStore.getExternalVolumeNames(appContext)) {
            token.put(
                volume,
                JSONObject()
                    .put(TOKEN_VERSION, MediaStore.getVersion(appContext, volume))
                    .put(TOKEN_GENERATION, MediaStore.getGeneration(appContext, volume)),
            )
        }
        return token
    }

    /** Null means a listing is needed: a card came or went, or a database was rebuilt. */
    private fun cameraChanges(saved: String): LocalChanges? {
        if (!hasMediaAccess()) throw failed("Home Space no longer has access to your photos.")
        val now = cameraToken() ?: return null
        val since = try {
            JSONObject(saved)
        } catch (_: JSONException) {
            return null
        }
        val volumes = now.keys().asSequence().toSet()
        if (volumes != since.keys().asSequence().toSet()) return null
        val items = mutableListOf<LocalItem>()
        for (volume in volumes) {
            val was = since.getJSONObject(volume)
            if (was.getString(TOKEN_VERSION) != now.getJSONObject(volume).getString(TOKEN_VERSION)) {
                return null
            }
            // A generation only means something within its volume.
            val generation = was.getLong(TOKEN_GENERATION)
            items += listMedia(IMAGE_PREFIX, MediaStore.Images.Media.getContentUri(volume), generation)
            items += listMedia(VIDEO_PREFIX, MediaStore.Video.Media.getContentUri(volume), generation)
        }
        return LocalChanges(items, now.toString())
    }

    private fun hasFullMediaAccess(): Boolean {
        val needed = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            listOf(
                android.Manifest.permission.READ_MEDIA_IMAGES,
                android.Manifest.permission.READ_MEDIA_VIDEO,
            )
        } else {
            listOf(android.Manifest.permission.READ_EXTERNAL_STORAGE)
        }
        return needed.all { appContext.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }
    }

    private fun listMedia(
        prefix: String,
        collection: Uri,
        afterGeneration: Long? = null,
    ): List<LocalItem> {
        val items = mutableListOf<LocalItem>()
        val filter = if (afterGeneration == null) {
            mediaFilter()
        } else {
            "${mediaFilter()} AND ${MediaStore.MediaColumns.GENERATION_MODIFIED} > $afterGeneration"
        }
        val cursor = resolver.query(collection, mediaColumns(), filter, null, null)
            ?: throw failed("Your photos can't be read")
        cursor.use {
            while (it.moveToNext()) mediaItem(prefix, it)?.let(items::add)
        }
        return items
    }

    private fun statMedia(key: String): LocalItem? {
        val prefix = if (key.startsWith(VIDEO_PREFIX)) VIDEO_PREFIX else IMAGE_PREFIX
        return resolver.query(mediaUri(key), mediaColumns(), null, null, null)?.use {
            if (it.moveToFirst()) mediaItem(prefix, it) else null
        }
    }

    private fun mediaItem(prefix: String, row: Cursor): LocalItem? {
        val path = row.getString(5) ?: return null
        val name = row.getString(1) ?: return null
        return LocalItem(
            key = "$prefix${row.getLong(0)}",
            relativeDir = cameraDir(path),
            name = name,
            size = row.getLong(2).toULong(),
            modifiedAt = Instant.ofEpochSecond(row.getLong(3)),
            mime = row.getString(4),
        )
    }

    /** The folder of a photo below `DCIM/`, from a relative path or, on old versions, a full one. */
    private fun cameraDir(path: String): String {
        val folder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            path
        } else {
            path.substringBeforeLast('/', "") + "/"
        }
        return folder.substringAfter(CAMERA_FOLDER, "").trimEnd('/')
    }

    private fun mediaUri(key: String): Uri {
        val isVideo = key.startsWith(VIDEO_PREFIX)
        val id = key.substringAfter(':').toLongOrNull() ?: throw failed("Unknown file: $key")
        return ContentUris.withAppendedId(if (isVideo) videosUri() else imagesUri(), id)
    }

    /** Without this, Android takes the location out of the photos it hands over. */
    private fun originalOf(uri: Uri): Uri =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q &&
            appContext.checkSelfPermission(android.Manifest.permission.ACCESS_MEDIA_LOCATION) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            MediaStore.setRequireOriginal(uri)
        } else {
            uri
        }

    private fun hasMediaAccess(): Boolean = cameraPermissions(includeLocation = false).any {
        appContext.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED
    }
}

/** What the camera source asks for, on this version of Android. */
fun cameraPermissions(includeLocation: Boolean = true): List<String> {
    val read = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE -> listOf(
            android.Manifest.permission.READ_MEDIA_IMAGES,
            android.Manifest.permission.READ_MEDIA_VIDEO,
            android.Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED,
        )
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU -> listOf(
            android.Manifest.permission.READ_MEDIA_IMAGES,
            android.Manifest.permission.READ_MEDIA_VIDEO,
        )
        else -> listOf(android.Manifest.permission.READ_EXTERNAL_STORAGE)
    }
    return if (includeLocation && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        read + android.Manifest.permission.ACCESS_MEDIA_LOCATION
    } else {
        read
    }
}

fun imagesUri(): Uri = MediaStore.Images.Media.EXTERNAL_CONTENT_URI

fun videosUri(): Uri = MediaStore.Video.Media.EXTERNAL_CONTENT_URI

private val DOCUMENT_COLUMNS = arrayOf(
    Document.COLUMN_DOCUMENT_ID,
    Document.COLUMN_DISPLAY_NAME,
    Document.COLUMN_MIME_TYPE,
    Document.COLUMN_SIZE,
    Document.COLUMN_LAST_MODIFIED,
)

@Suppress("DEPRECATION") // DATA is the only way to the folder before RELATIVE_PATH (API 29).
private fun mediaColumns(): Array<String> = arrayOf(
    MediaStore.MediaColumns._ID,
    MediaStore.MediaColumns.DISPLAY_NAME,
    MediaStore.MediaColumns.SIZE,
    MediaStore.MediaColumns.DATE_MODIFIED,
    MediaStore.MediaColumns.MIME_TYPE,
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        MediaStore.MediaColumns.RELATIVE_PATH
    } else {
        MediaStore.MediaColumns.DATA
    },
)

@Suppress("DEPRECATION")
private fun mediaFilter(): String = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
    // Pending files are still being written.
    "${MediaStore.MediaColumns.RELATIVE_PATH} LIKE '$CAMERA_FOLDER%' " +
        "AND ${MediaStore.MediaColumns.IS_PENDING} = 0"
} else {
    "${MediaStore.MediaColumns.DATA} LIKE '%/$CAMERA_FOLDER%'"
}
