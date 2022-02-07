package main

import (
	"net/http"
	"os"
	"path/filepath"
)

type FrontendHandler struct {
	staticPath string
	indexPath  string
}

func (handler FrontendHandler) ServeHTTP(writer http.ResponseWriter, request *http.Request) {
	path, err := filepath.Abs(request.URL.Path)
	if err != nil {
		http.Error(writer, err.Error(), http.StatusBadRequest)
	}
	path = filepath.Join(handler.staticPath, path)
	_, err = os.Stat(path)
	if os.IsNotExist(err) {
		// file does not exist serve index.html
		http.ServeFile(writer, request, filepath.Join(handler.staticPath, handler.indexPath))
		return
	}
	// serve other assets
	http.FileServer(http.Dir(handler.staticPath)).ServeHTTP(writer, request)
}
