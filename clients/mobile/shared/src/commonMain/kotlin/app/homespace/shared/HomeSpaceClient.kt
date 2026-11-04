package app.homespace.shared

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.request.get
import io.ktor.client.statement.bodyAsText
import io.ktor.http.isSuccess
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.json.Json

/**
 * Client for one Home Space server, shared by the Android and iOS apps.
 *
 * [baseUrl] is the server root (for example `http://10.0.2.2:8080`); routes add `/api/v1`.
 * The HTTP engine comes from the platform: OkHttp on Android, NSURLSession on iOS.
 */
class HomeSpaceClient internal constructor(
    baseUrl: String,
    private val http: HttpClient,
) {
    constructor(baseUrl: String) : this(baseUrl, HttpClient())

    private val baseUrl = baseUrl.trimEnd('/')

    private val client = http.config {
        expectSuccess = false
        install(ContentNegotiation) {
            json(Json { ignoreUnknownKeys = true })
        }
    }

    /** Asks the server whether it and its database are up. */
    @Throws(Exception::class)
    suspend fun health(): ServerHealth {
        val response = client.get(baseUrl + Paths.HEALTH)
        if (!response.status.isSuccess()) {
            throw ApiException(response.status.value, response.bodyAsText())
        }
        return response.body()
    }

    /** Releases the connections; the client cannot be used afterwards. */
    fun close() {
        client.close()
        http.close()
    }
}
