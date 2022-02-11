package helpers

import (
	"home-space/ioc"
	"net/http"
)

func GetInjector(request *http.Request) ioc.Container {
	return request.Context().Value(ioc.CONTAINER_CONTEXT_KEY).(ioc.Container)
}
