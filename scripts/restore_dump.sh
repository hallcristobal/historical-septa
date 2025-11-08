#!/usr/bin/env bash
set -ex

echo "drop database historical_septa_dev; create database historical_septa_dev;" | psql -h localhost postgres
pg_restore --no-privileges --no-owner -h localhost -U $USER -d historical_septa_dev -Fc *-dev.dump
