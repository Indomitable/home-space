package api

import (
	"encoding/json"
	"home-space/security"
	"net/http"

	"github.com/gorilla/mux"
)

func RegisterApis(router *mux.Router) {
	router.HandleFunc("/test", func(writer http.ResponseWriter, r *http.Request) {
		hello := struct {
			UserName string `json:"userName"`
		}{
			UserName: "Ventsi",
		}
		writer.Header().Add("Content-Type", "application/json; charset=utf-8")
		json.NewEncoder(writer).Encode(hello)
		writer.WriteHeader(http.StatusOK)
	})

	registerAuthApis(router.PathPrefix("/auth").Subrouter())
}

func registerAuthApis(router *mux.Router) {
	router.HandleFunc("/login", security.Login).Methods("POST")
	router.HandleFunc("/register", security.Register).Methods("POST")
}
