package app.homespace.android

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.ForegroundInfo
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.QueuedUpload
import uniffi.homespace_core.QueuedUploadState

private const val WORK = "picked-uploads"
private const val CHANNEL = "uploads"
private const val PROGRESS_NOTIFICATION = 2
private const val FAILED_NOTIFICATION = 3
private const val RETRY_SECONDS = 60L
private const val NOTIFICATION_MILLIS = 1_000L

/**
 * Sends the files the user picked while the app isn't in front, with a notification showing how far
 * it has come. This doesn't depend on "Sync in background", which is about automatic transfers.
 */
class UploadWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    private val notifications = applicationContext.getSystemService(NotificationManager::class.java)

    override suspend fun doWork(): Result {
        val session = try {
            ClientHolder.resume(applicationContext)
        } catch (e: CoreException) {
            return if (e is CoreException.Unauthorized) Result.success() else Result.retry()
        } ?: return Result.success()
        val queue = session.uploads
        if (queue.items().none(QueuedUpload::canGo)) return Result.success()
        return try {
            coroutineScope {
                val updates = launch {
                    ClientHolder.uploadChanges.collectLatest {
                        showProgress(queue.items())
                        delay(NOTIFICATION_MILLIS)
                    }
                }
                try {
                    val summary = queue.run()
                    if (summary.failed > 0uL) showFailed(summary.failed)
                    // Android runs this again once the network is there, or the time limit has passed.
                    if (summary.remaining > 0uL) Result.retry() else Result.success()
                } finally {
                    updates.cancel()
                }
            }
        } catch (e: CancellationException) {
            // Android took back the network or the time. What is left goes on the next run.
            queue.stop()
            throw e
        } catch (_: CoreException.Unauthorized) {
            Result.success() // Opening the app shows it.
        } catch (_: CoreException) {
            Result.retry()
        }
    }

    private suspend fun showProgress(items: List<QueuedUpload>) {
        if (items.none(QueuedUpload::canGo)) return
        try {
            setForeground(foregroundInfo(items))
        } catch (_: IllegalStateException) {
            // Android doesn't allow it right now, after a start in the background. The upload still
            // runs within the job's limits, and carries on from there next time.
        }
    }

    override suspend fun getForegroundInfo(): ForegroundInfo {
        val session = ClientHolder.active
        return foregroundInfo(session?.let { runCatching { it.uploads.items() }.getOrNull() }.orEmpty())
    }

    private fun foregroundInfo(items: List<QueuedUpload>): ForegroundInfo {
        val going = items.filter(QueuedUpload::canGo)
        val total = going.sumOf { it.total.toLong() }
        val done = going.sumOf { it.done.toLong() }
        val title = if (going.size == 1) "Uploading ${going[0].name}" else "Uploading ${going.size} files"
        val waiting = going.isNotEmpty() && going.all { it.state == QueuedUploadState.OFFLINE }
        val notification = builder()
            .setContentTitle(title)
            .setContentText(if (waiting) "Waiting for the connection" else null)
            .setOngoing(true)
            .setProgress(PERCENT, if (total > 0) (done * PERCENT / total).toInt() else 0, total == 0L)
            .build()
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ForegroundInfo(PROGRESS_NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            ForegroundInfo(PROGRESS_NOTIFICATION, notification)
        }
    }

    /** With the app out of sight, this is the only way a failure gets seen. */
    private fun showFailed(failed: ULong) {
        if (ClientHolder.appInFront) return
        val title = if (failed == 1uL) "A file couldn't be uploaded" else "$failed files couldn't be uploaded"
        val notification = builder().setContentTitle(title).setAutoCancel(true).build()
        notifications.notify(FAILED_NOTIFICATION, notification)
    }

    private fun builder(): Notification.Builder {
        notifications.createNotificationChannel(
            NotificationChannel(CHANNEL, "Uploads", NotificationManager.IMPORTANCE_LOW),
        )
        val open = Intent(applicationContext, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP)
        return Notification.Builder(applicationContext, CHANNEL)
            .setSmallIcon(R.drawable.ic_upload_file)
            .setContentIntent(
                PendingIntent.getActivity(applicationContext, 0, open, PendingIntent.FLAG_IMMUTABLE),
            )
    }
}

/** Whether this file goes without the user doing anything. */
private fun QueuedUpload.canGo(): Boolean =
    state != QueuedUploadState.FAILED

/**
 * Makes sure the uploads carry on when the app leaves the front. Call it with the app in front:
 * only then may the notification's foreground service start in all cases.
 */
fun scheduleUploads(context: Context) {
    val request = OneTimeWorkRequestBuilder<UploadWorker>()
        .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
        .setBackoffCriteria(BackoffPolicy.LINEAR, RETRY_SECONDS, TimeUnit.SECONDS)
        .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
        .build()
    WorkManager.getInstance(context).enqueueUniqueWork(WORK, ExistingWorkPolicy.KEEP, request)
}

fun cancelUploadWork(context: Context) {
    WorkManager.getInstance(context).cancelUniqueWork(WORK)
    context.getSystemService(NotificationManager::class.java).cancel(FAILED_NOTIFICATION)
}

/** The app is on screen, so the rows show any failure. */
fun clearUploadFailureNotice(context: Context) {
    context.getSystemService(NotificationManager::class.java).cancel(FAILED_NOTIFICATION)
}
