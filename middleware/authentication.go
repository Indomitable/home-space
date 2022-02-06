package middleware

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"strings"

	ctx "home-space/http_context"
	"home-space/security"

	"github.com/golang-jwt/jwt"
)

func AuthMiddleware() func(http.Handler) http.Handler {
	key := os.Getenv("JWT_TOKEN_SEC_KEY")
	return func(next http.Handler) http.Handler {
		fn := func(w http.ResponseWriter, r *http.Request) {
			// Logic here

			auth := GetAuthContext(r, key)
			newRequest := r.WithContext(context.WithValue(r.Context(), ctx.AuthContextKey, &auth))

			next.ServeHTTP(w, newRequest)
		}

		return http.HandlerFunc(fn)
	}
}

func GetAuthContext(r *http.Request, key string) ctx.AuthenticationContext {
	auth_header := r.Header.Get("Authorization")

	if len(auth_header) > 0 {
		token := strings.TrimPrefix(auth_header, "Bearer ")
		parsed_token, err := jwt.ParseWithClaims(token, &security.UserClaims{}, func(t *jwt.Token) (interface{}, error) {
			if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
				return nil, fmt.Errorf("unexpected signing method: %v", t.Header["alg"])
			}

			return []byte(key), nil
		})
		if err == nil {
			if claims, ok := parsed_token.Claims.(*security.UserClaims); ok && parsed_token.Valid {
				return ctx.NewAuthenticated(claims.UserName)
			}
		}
	}

	return ctx.NewUnauthenticated()
}
