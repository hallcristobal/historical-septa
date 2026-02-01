#!/usr/bin/env bash
if [ ! -f .env ]; then
  echo ".env file is missing!"
  exit 1
fi

source .env
export PGPASSWORD="${DATABASE_PASS}"
set -ex

echo "drop database $DATABASE_NAME; create database $DATABASE_NAME;" | psql -U $DATABASE_USER -h localhost postgres
pg_restore --no-privileges --no-owner -h localhost -U $DATABASE_USER -d $DATABASE_NAME -Fc *-dev.dump
