package security

import "github.com/golang-jwt/jwt"

type UserClaims struct {
	UserName string `json:"userName"`
	jwt.StandardClaims
}
