package ioc

import (
	"home-space/db"
	"home-space/user"
)

func (c ContainerImp) RegisterServices() {
	repository := addRepository(c)
	addUserService(c, repository)
}

func addRepository(c ContainerImp) db.Repository {
	repository := db.NewRepository()
	repository.Connect()
	c.AddSingleton("repository", repository)
	c.RegisterClosable(repository)
	return repository
}

func addUserService(c ContainerImp, repo db.Repository) user.UserService {
	service := user.NewUserService(repo, user.BCryptHashService{})
	c.AddSingleton("userService", service)
	return service
}
