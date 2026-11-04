package app.homespace.shared

import kotlinx.serialization.Serializable

/** The body of `GET /api/v1/health`. */
@Serializable
data class ServerHealth(val status: String) {
    val isOk: Boolean get() = status == "ok"
}
