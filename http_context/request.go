package http_context

import (
	"net/http"
)

func GetAuth(request *http.Request) AuthenticationContext {
	return request.Context().Value(AuthContextKey).(AuthenticationContext)
}
