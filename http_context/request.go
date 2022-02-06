package http_context

import (
	"net/http"

	"home-space/ioc"
)

func GetContainer(request *http.Request) ioc.Container {
	return request.Context().Value(ioc.CONTAINER_CONTEXT_KEY).(ioc.Container)
}

func GetAuth(request *http.Request) AuthenticationContext {
	return request.Context().Value(AuthContextKey).(AuthenticationContext)
}
