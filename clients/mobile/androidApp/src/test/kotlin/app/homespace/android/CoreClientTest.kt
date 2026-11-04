package app.homespace.android

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.homespace_core.CoreClient
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.DeviceIdentity
import uniffi.homespace_core.SessionListener
import uniffi.homespace_core.humanSize
import uniffi.homespace_core.normalizeServerUrl

/** The generated Kotlin bindings against the host build of the Rust library. */
class CoreClientTest {
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

    @Test
    fun sessionListenerIsCalledAcrossTheBindings() = runTest {
        val tokens = mutableListOf<String?>()
        val client = CoreClient.withSession(
            "http://127.0.0.1:9",
            DeviceIdentity("00000000-0000-0000-0000-0000000000aa", "Test"),
            object : SessionListener {
                override fun refreshTokenChanged(token: String?) {
                    tokens.add(token)
                }
            },
        )
        try {
            // Signing out with no session needs no network, and still reports the session gone.
            client.logout()
            assertEquals(listOf<String?>(null), tokens)
        } finally {
            client.close()
        }
    }

    @Test
    fun beginSsoReturnsTheMobileRedirect() {
        CoreClient("https://files.example.com").use { client ->
            val request = client.beginSso()
            assertTrue(request.url, request.url.contains("redirect_uri=app.homespace"))
            assertTrue(request.verifier.isNotEmpty())
        }
    }

    @Test
    fun serverAddressesAreTidied() {
        assertEquals("https://files.example.com", normalizeServerUrl(" https://files.example.com/ "))
        assertThrows(CoreException.InvalidServerAddress::class.java) {
            normalizeServerUrl("files.example.com")
        }
    }
}
