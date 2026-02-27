#!/usr/bin/env bash
if [ ! -f .env ]; then
  echo ".env file is missing!"
  exit 1
fi

source .env
export PGPASSWORD="${DATABASE_PASS}"

set -x

echo "drop database if exists ${DATABASE_NAME}; create database if not exists ${DATABASE_NAME};" | psql -h localhost postgres
pg_restore --no-privileges --no-owner -h ${DATABASE_HOST} -p ${DATABASE_PORT} \
  -U ${DATABASE_USER} -d ${DATABASE_NAME} -Fc *-dev.dump
