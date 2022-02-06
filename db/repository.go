package db

import (
	"context"
	"fmt"
	"os"

	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/pgxpool"
)

const REPOSITORY_CONTEXT = "REPOSITORY"

type Repository interface {
	Connect()
	Close()
	Query(context context.Context, sql string, args ...interface{}) (pgx.Rows, error)
	QueryRow(context context.Context, sql string, args ...interface{}) pgx.Row
	Execute(context context.Context, sql string, args ...interface{}) error
	BeginTransaction(context context.Context) (pgx.Tx, error)
}

type RepositoryImp struct {
	pool *pgxpool.Pool
}

func NewRepository() Repository {
	return &RepositoryImp{}
}

func (repository *RepositoryImp) connectionString() string {
	server := os.Getenv("DB_SERVER_NAME")
	database := os.Getenv("DB_SERVER_DATABASE")
	userName := os.Getenv("DB_SERVER_USER_NAME")
	password := os.Getenv("DB_SERVER_PASSWORD")

	return fmt.Sprintf("postgresql://%s:%s@%s/%s?connect_timeout=10&application_name=home-space", userName, password, server, database)
}

func (repository *RepositoryImp) Connect() {
	pool, err := pgxpool.Connect(context.Background(), repository.connectionString())
	if err != nil {
		fmt.Printf("Can not connect to database: %s", repository.connectionString())
	}
	repository.pool = pool
}

func (repository *RepositoryImp) Close() {
	repository.pool.Close()
}

func (repository *RepositoryImp) Query(context context.Context, sql string, args ...interface{}) (pgx.Rows, error) {
	return repository.pool.Query(context, sql, args...)
}

func (repository *RepositoryImp) QueryRow(context context.Context, sql string, args ...interface{}) pgx.Row {
	return repository.pool.QueryRow(context, sql, args...)
}

func (repository *RepositoryImp) Execute(context context.Context, sql string, args ...interface{}) error {
	_, err := repository.pool.Exec(context, sql, args...)
	return err
}

func (repository *RepositoryImp) BeginTransaction(context context.Context) (pgx.Tx, error) {
	return repository.pool.Begin(context)
}
