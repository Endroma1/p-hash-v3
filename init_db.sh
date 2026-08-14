#!/bin/bash

set -eo pipefail
sqlx --version || {
    echo "sqlx not found - Install with 'cargo install sqlx'"
    exit 1
}

DB_PORT="${DB_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=postgres}"
APP_USER_NAME=${APP_USER_NAME:=phash}
APP_USER_PWD=${APP_USER_PWD:=password}
APP_DB_NAME=${APP_DB_NAME:=phash}

if [[ -z "${SKIP_DOCKER}" ]]; then
    CONTAINER_NAME="postgres-phash"
    podman run \
        --name "${CONTAINER_NAME}" \
        --env POSTGRES_PASSWORD="${SUPERUSER_PWD}" \
        --health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
        --health-interval=1s \
        --health-timeout=5s --health-retries=5 --publish "${DB_PORT}":5432 \
        --detach postgres

    until [ \
        "$(podman inspect -f "{{.State.Health.Status}}" ${CONTAINER_NAME})" == \
        "healthy" \
        ]; do
        >&2 echo "Postgres still unavailable - Sleeping"
        sleep 1
    done

    CREATE_QUERY="CREATE USER ${APP_USER_NAME} WITH PASSWORD '${APP_USER_PWD}';"
    podman exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

    GRANT_QUERY="ALTER USER ${APP_USER_NAME} CREATEDB;"
    podman exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"

    >&2 echo "Postgres is up and running on port ${DB_PORT}"
fi

DATABASE_URL=postgres://${APP_USER_NAME}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

>&2 echo "Postgres migrated"
