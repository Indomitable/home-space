package middleware

import (
	"context"
	"net/http"

	"home-space/ioc"
)

func UseContainer(container ioc.Container) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		fn := func(w http.ResponseWriter, r *http.Request) {
			// Logic here
			newRequest := r.WithContext(context.WithValue(r.Context(), ioc.CONTAINER_CONTEXT_KEY, container))
			next.ServeHTTP(w, newRequest)
		}

		return http.HandlerFunc(fn)
	}
}
