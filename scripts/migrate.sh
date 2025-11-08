#!/usr/bin/env bash
set -x

if [ ! -f .env ]; then
  echo ".env file is missing!"
  exit 1
fi

./migrator | tee -a migrator.log
