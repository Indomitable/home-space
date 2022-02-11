package files

import (
	"home-space/db"
	"home-space/helpers"
	"home-space/http_context"
	"home-space/security"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
)

type FileController struct {
}

func NewFileController() *FileController {
	return &FileController{}
}

func (fs *FileController) HandleTopFiles() security.AuthenticatedHandler {
	var fn security.AuthenticatedHandler = func(writer http.ResponseWriter, request *http.Request, auth *http_context.AuthenticationContext) {
		file_repository := GetFileRepository(request)
		top_files := file_repository.FetchTopFiles(request.Context(), auth.Claims.UserId)
		helpers.WriteResponse(writer, top_files)
	}
	return fn
}

func (fs *FileController) HandleDirectoryContents() security.AuthenticatedHandler {
	var fn security.AuthenticatedHandler = func(writer http.ResponseWriter, request *http.Request, auth *http_context.AuthenticationContext) {
		file_repository := GetFileRepository(request)
		query := mux.Vars(request)
		directory_id, _ := strconv.ParseInt(query["id"], 10, 64)
		directory_contents := file_repository.FetchDirectoryContents(request.Context(), directory_id, auth.Claims.UserId)
		helpers.WriteResponse(writer, directory_contents)
	}
	return fn
}

func GetFileRepository(request *http.Request) db.FileRepository {
	container := helpers.GetInjector(request)
	return container.GetService("fileRepository").(db.FileRepository)
}
