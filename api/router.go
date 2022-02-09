package api

import (
	"encoding/json"
	"home-space/security"
	"net/http"

	"github.com/gorilla/mux"
)

type User struct {
	UserName string `json:"userName"`
}

func RegisterApis(router *mux.Router) {
	router.HandleFunc("/test", func(writer http.ResponseWriter, r *http.Request) {
		var user User
		json.NewDecoder(r.Body).Decode(&user)
		writer.Header().Add("Content-Type", "application/json; charset=utf-8")
		user.UserName += " Mladenov"
		json.NewEncoder(writer).Encode(user)
		writer.WriteHeader(http.StatusOK)
	}).Methods("GET", "POST")

	registerAuthApis(router.PathPrefix("/auth").Subrouter())
}

func registerAuthApis(router *mux.Router) {
	router.HandleFunc("/login", security.Login).Methods("POST")
	router.HandleFunc("/register", security.Register).Methods("POST")
}
