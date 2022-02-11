package helpers

import (
	"encoding/json"
	"net/http"
)

func WriteResponse(writer http.ResponseWriter, data interface{}) {
	writer.Header().Set("Content-Type", "application/json")
	encoder := json.NewEncoder(writer)
	encoder.Encode(data)
}
