package db

import (
	"context"

	"github.com/jackc/pgtype"
	"github.com/jackc/pgx/v4"
)

type FileModel struct {
	Id             int64  `json:"id"`
	UserId         int64  `json:"user_id"`
	Title          string `json:"title"`
	ParentId       int64  `json:"parent_id"`
	NodeType       int16  `json:"node_type"`
	FileSystemPath string `json:"filesystem_path"`
	MimeType       string `json:"mime_type"`
}

type FileRepository interface {
	FetchTopFiles(context context.Context, user int64) []FileModel
	FetchDirectoryContents(context context.Context, directory int64, user int64) []FileModel
}

type FileRepositoryImp struct {
	repository Repository
}

func NewFileRepository(repository Repository) FileRepository {
	return &FileRepositoryImp{
		repository: repository,
	}
}

func (fs *FileRepositoryImp) FetchTopFiles(context context.Context, user int64) []FileModel {
	sql := "select * from file_nodes fn where fn.parent_id is null and user_id = $1"
	rows, err := fs.repository.Query(context, sql, user)
	return ReadFiles(rows, err)
}

func (fs *FileRepositoryImp) FetchDirectoryContents(context context.Context, directory int64, user int64) []FileModel {
	sql := "select * from file_nodes fn where fn.parent_id = $1 and user_id = $2"
	rows, err := fs.repository.Query(context, sql, directory, user)
	return ReadFiles(rows, err)
}

func ReadFiles(rows pgx.Rows, err error) []FileModel {
	files := make([]FileModel, 0)
	if err != nil {
		return files
	}
	defer rows.Close()
	var (
		id              pgtype.Int8
		user_id         pgtype.Int8
		title           pgtype.Varchar
		parent_id       pgtype.Int8
		node_type       pgtype.Int2
		filesystem_path pgtype.Varchar
		mime_type       pgtype.Varchar
	)
	for rows.Next() {
		rows.Scan(&id, &user_id, &title, &parent_id, &node_type, &filesystem_path, &mime_type)
		var parent_id_int int64 = -1
		if parent_id.Status == pgtype.Present {
			parent_id_int = parent_id.Int
		}
		files = append(files, FileModel{
			Id:             id.Int,
			UserId:         user_id.Int,
			Title:          title.String,
			ParentId:       parent_id_int,
			NodeType:       node_type.Int,
			FileSystemPath: filesystem_path.String,
			MimeType:       filesystem_path.String,
		})
	}
	return files
}
