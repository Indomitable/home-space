package app.homespace.shared

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class HomeSpaceClientTest {
    private fun client(status: HttpStatusCode, body: String, seen: MutableList<String> = mutableListOf()) =
        HomeSpaceClient(
            "http://server/",
            HttpClient(MockEngine { request ->
                seen += request.url.toString()
                respond(body, status, headersOf(HttpHeaders.ContentType, "application/json"))
            }),
        )

    @Test
    fun healthReadsStatus() = runTest {
        val seen = mutableListOf<String>()
        val health = client(HttpStatusCode.OK, """{"status":"ok"}""", seen).health()

        assertTrue(health.isOk)
        assertEquals(listOf("http://server/api/v1/health"), seen)
    }

    @Test
    fun healthFailsOnErrorStatus() = runTest {
        val error = assertFailsWith<ApiException> {
            client(HttpStatusCode.InternalServerError, """{"error":"db down"}""").health()
        }
        assertEquals(500, error.statusCode)
    }
}
