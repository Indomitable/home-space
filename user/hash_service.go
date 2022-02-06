package user

import "golang.org/x/crypto/bcrypt"

type HashService interface {
	HashPassword(password string) []byte
	CompareHashAndPassword(hash []byte, password string) bool
}

type BCryptHashService struct{}

func (s BCryptHashService) HashPassword(password string) []byte {
	hash, _ := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	return hash
}

func (s BCryptHashService) CompareHashAndPassword(hash []byte, password string) bool {
	return bcrypt.CompareHashAndPassword(hash, []byte(password)) == nil
}
