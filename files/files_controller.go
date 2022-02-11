package files

import (
	"home-space/db"
	"home-space/helpers"
	"net/http"
)

type FileController struct {
}

func NewFileController() *FileController {
	return &FileController{}
}

func (fs *FileController) HandleTopFiles() http.Handler {
	var fn http.HandlerFunc = func(writer http.ResponseWriter, request *http.Request) {
		container := helpers.GetInjector(request)
		file_service := container.GetService("fileRepository").(db.FileRepository)
		top_files := file_service.FetchTopFiles(request.Context())
		helpers.WriteResponse(writer, top_files)
	}
	return fn
}
