#!/usr/bin/env bash
# Manage the local Postgres container used by the home-space server.
set -euo pipefail

CONTAINER=homespace-db
VOLUME=homespace-pgdata
IMAGE=docker.io/library/postgres:latest
DB_USER=${POSTGRES_USER:-homespace}
DB_PASSWORD=${POSTGRES_PASSWORD:-homespace}
DB_NAME=${POSTGRES_DB:-homespace}
DB_PORT=${POSTGRES_PORT:-5432}

case "${1:-}" in
  up)
    if podman container exists "$CONTAINER"; then
      podman start "$CONTAINER"
    else
      podman run -d --name "$CONTAINER" \
        -e POSTGRES_USER="$DB_USER" \
        -e POSTGRES_PASSWORD="$DB_PASSWORD" \
        -e POSTGRES_DB="$DB_NAME" \
        -p "$DB_PORT":5432 \
        -v "$VOLUME":/var/lib/postgresql \
        "$IMAGE"
    fi
    ;;
  down)
    podman stop "$CONTAINER"
    ;;
  destroy)
    podman rm -f "$CONTAINER"
    podman volume rm "$VOLUME"
    ;;
  psql)
    podman exec -it "$CONTAINER" psql -U "$DB_USER" "$DB_NAME"
    ;;
  logs)
    podman logs -f "$CONTAINER"
    ;;
  *)
    echo "usage: $0 {up|down|destroy|psql|logs}" >&2
    exit 1
    ;;
esac
