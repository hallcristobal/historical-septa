#!/usr/bin/env bash
set -x
if [ ! -f .env ]; then
  echo ".env file is missing!"
  exit 1
fi

PID=$(pgrep -f "septa")
if [ -z "$PID" ]; then
  echo "septa isn't running?"
  sleep 2
else
  kill "$PID"
  while ps -p "$PID" > /dev/null; do
    echo "Waiting for PID $PID to terminate..."
    sleep 1 # Wait for 1 second before checking again
  done
  echo "PID $PID has terminated."
fi

mv output.log output.log-$(date -u +%s)
mv septa septa-$(date -u +%s)
cp septa-new septa
./septa >> output.log 2>&1 & disown
