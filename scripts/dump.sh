#!/usr/bin/env bash

if [ ! -f .env ]; then
  echo ".env file is missing!"
  exit 1
fi

source .env
export PGPASSWORD="${DATABASE_PASS}"

set -x
pg_dump -h ${DATABASE_HOST} -p ${DATABASE_PORT} \
  -U ${DATABASE_USER} -d ${DATABASE_NAME} -Fc -fx "$(date -u +%s)-dev.dump"


