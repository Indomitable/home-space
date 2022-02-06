package security

import (
	"net/http"

	ctx "home-space/http_context"
)

type AuthenticatedHandler func(http.ResponseWriter, *http.Request, *ctx.AuthenticationContext)

type AuthenticatedGuard struct {
	handler AuthenticatedHandler
}

func NewAuthenticationGuard(handler AuthenticatedHandler) *AuthenticatedGuard {
	return &AuthenticatedGuard{
		handler: handler,
	}
}

func (guard AuthenticatedGuard) ServeHTTP(writer http.ResponseWriter, request *http.Request) {
	if auth, ok := request.Context().Value(ctx.AuthContextKey).(*ctx.AuthenticationContext); ok && auth.IsAuthenticated {
		guard.handler(writer, request, auth)
	} else {
		writer.WriteHeader(http.StatusUnauthorized)
		return
	}
}
