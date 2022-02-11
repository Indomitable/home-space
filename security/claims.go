package security

import "github.com/golang-jwt/jwt"

type UserClaims struct {
	UserId   int64  `json:"userId"`
	UserName string `json:"userName"`
	jwt.StandardClaims
}
