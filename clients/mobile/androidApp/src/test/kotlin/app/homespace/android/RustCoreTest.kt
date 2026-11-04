package app.homespace.android

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.homespace_core.CoreClient
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.humanSize

/** The generated Kotlin bindings against the host build of the Rust library. */
class RustCoreTest {
    @Test
    fun formatsSizesInRust() {
        assertEquals("1.5 MB", humanSize(1_572_864))
    }

    @Test
    fun unreachableServerIsATypedError() = runTest {
        val client = CoreClient("http://127.0.0.1:9")
        try {
            client.health()
            error("expected a failure")
        } catch (e: CoreException.Unreachable) {
            assertTrue(e.message.orEmpty().isNotEmpty())
        } finally {
            client.close()
        }
    }

    @Test
    fun invalidIdIsRejected() = runTest {
        val client = CoreClient("http://127.0.0.1:9")
        try {
            client.list("nope")
            error("expected a failure")
        } catch (e: CoreException.InvalidId) {
            assertEquals("nope", e.id)
        } finally {
            client.close()
        }
    }
}
