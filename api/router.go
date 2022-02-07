package api

import (
	"home-space/security"

	"github.com/gorilla/mux"
)

func RegisterApis(router *mux.Router) {
	registerAuthApis(router.PathPrefix("/auth").Subrouter())
}

func registerAuthApis(router *mux.Router) {
	router.HandleFunc("/login", security.Login).Methods("POST")
	router.HandleFunc("/register", security.Register).Methods("POST")
}
