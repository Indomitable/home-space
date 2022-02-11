package ioc

import (
	"home-space/db"
	"home-space/user"
)

func (c ContainerImp) RegisterServices() {
	repository := addRepository(c)
	addUserService(c, repository)
	addFileRepository(c, repository)
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

func addFileRepository(c ContainerImp, repo db.Repository) db.FileRepository {
	file_repository := db.NewFileRepository(repo)
	c.AddSingleton("fileRepository", file_repository)
	return file_repository
}
