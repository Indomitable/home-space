package user

import (
	"context"
	"home-space/db"
)

type UserService interface {
	Register(context context.Context, userName string, password string) bool
	CheckLogin(context context.Context, userName string, password string) bool
}

type UserServiceImpl struct {
	repository  db.Repository
	hashService HashService
}

func NewUserService(repo db.Repository, hashService HashService) UserService {
	return &UserServiceImpl{
		repository:  repo,
		hashService: hashService,
	}
}

func (service *UserServiceImpl) Register(context context.Context, userName string, password string) bool {
	tx, _ := service.repository.BeginTransaction(context)
	var user_id uint64
	err := tx.QueryRow(context, "insert into users (name) values ($1) RETURNING id", userName).Scan(&user_id)
	if err != nil {
		tx.Rollback(context)
		return false
	}
	hash := service.hashService.HashPassword(password)
	var auth_password_id uint64
	err = tx.QueryRow(context, "insert into authentication_password (hash) values ($1) RETURNING id", hash).Scan(&auth_password_id)
	if err != nil {
		tx.Rollback(context)
		return false
	}
	_, err = tx.Exec(context, "insert into authentication (user_id, auth_type_id, auth_password_id) values ($1, 1, $2)", user_id, auth_password_id)
	if err != nil {
		tx.Rollback(context)
		return false
	}
	err = tx.Commit(context)
	if err != nil {
		tx.Rollback(context)
		return false
	}
	return true
}

func (service *UserServiceImpl) CheckLogin(context context.Context, userName string, password string) bool {
	row := service.repository.QueryRow(context, `select ap.hash  from authentication_password ap
	inner join authentication a on a.auth_password_id  = ap.id 
	inner join users u on u.id  = a.user_id 
	where u."name" = $1`, userName)
	if row != nil {
		var hash []byte
		err := row.Scan(&hash)
		if err == nil {
			return service.hashService.CompareHashAndPassword(hash, password)
		}
	}

	return false
}
