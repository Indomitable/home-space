package main

import (
	"fmt"
	"net/http"
	"os"

	"github.com/gorilla/mux"
	"github.com/joho/godotenv"

	ctx "home-space/http_context"
	"home-space/ioc"
	"home-space/middleware"
	"home-space/security"
)

func main() {
	_ = godotenv.Load(".env.development.local")
	router := mux.NewRouter()

	container := ioc.NewContainer()
	container.RegisterServices()
	defer container.Close()

	router.Use(middleware.AuthMiddleware())

	router.Use(middleware.UseContainer(container))

	router.Path("/").Handler(security.NewAuthenticationGuard(func(writer http.ResponseWriter, r *http.Request, auth *ctx.AuthenticationContext) {
		if auth.IsAuthenticated {
			writer.Write([]byte("Wellcome " + auth.Claims.UserName))
		} else {
			writer.Write([]byte("Not authenticated"))
		}
		writer.WriteHeader(http.StatusOK)
	})).Methods("GET")
	router.Path("/api/login").HandlerFunc(security.Login).Methods("POST")
	router.Path("/api/register").HandlerFunc(security.Register).Methods("POST")

	schema := os.Getenv("SERVER_SCHEMA")
	port := os.Getenv("SERVER_PORT")
	host := os.Getenv("SERVER_NAME")
	fmt.Printf("%s://%s:%s", schema, host, port)
	_ = http.ListenAndServe(":"+port, router)
}
