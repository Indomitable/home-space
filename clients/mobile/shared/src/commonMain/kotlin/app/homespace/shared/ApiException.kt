package app.homespace.shared

/** The server answered, but not with a success status. */
class ApiException(val statusCode: Int, message: String) : Exception(message)
