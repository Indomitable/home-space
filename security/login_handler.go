package security

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"time"

	jwt "github.com/golang-jwt/jwt"

	"home-space/helpers"
	"home-space/user"
)

func Login(writer http.ResponseWriter, r *http.Request) {
	container := helpers.GetInjector(r)
	userService := container.GetService("userService").(user.UserService)
	login := struct {
		UserName string `json:"user_name"`
		Password string `json:"password"`
	}{}
	json.NewDecoder(r.Body).Decode(&login)
	if userService.CheckLogin(r.Context(), login.UserName, login.Password) {
		createToken(writer, login.UserName)
	} else {
		writer.WriteHeader(http.StatusForbidden)
	}
}

func Register(writer http.ResponseWriter, r *http.Request) {
	container := helpers.GetInjector(r)
	userService := container.GetService("userService").(user.UserService)
	login := struct {
		UserName string `json:"user_name"`
		Password string `json:"password"`
	}{}
	json.NewDecoder(r.Body).Decode(&login)
	error := userService.Register(r.Context(), login.UserName, login.Password)
	if error == nil {
		createToken(writer, login.UserName)
		return
	} else {
		writer.WriteHeader(http.StatusBadRequest)
		writer.Write([]byte(error.Error()))
	}
}

func createToken(writer http.ResponseWriter, userName string) {
	os.Getenv("JWT_TOKEN_SEC_KEY")
	schema := os.Getenv("SERVER_SCHEMA")
	port := os.Getenv("SERVER_PORT")
	host := os.Getenv("SERVER_NAME")
	issuer := fmt.Sprintf("%s://%s:%s", schema, host, port)
	claims := UserClaims{
		UserName: userName,
		StandardClaims: jwt.StandardClaims{
			Issuer:    issuer,
			ExpiresAt: time.Now().Add(time.Hour * 3).Unix(),
		},
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	key := os.Getenv("JWT_TOKEN_SEC_KEY")
	signed_token, err := token.SignedString([]byte(key))
	if err != nil {
		writer.Write([]byte(err.Error()))
		writer.WriteHeader(http.StatusInternalServerError)
	} else {
		writer.Header().Add("Content-Type", "application/json; charset=utf-8")
		loginReponse := struct {
			AccessToken string `json:"access_token"`
		}{
			AccessToken: signed_token,
		}
		json.NewEncoder(writer).Encode(loginReponse)
		writer.WriteHeader(http.StatusOK)
	}
}
