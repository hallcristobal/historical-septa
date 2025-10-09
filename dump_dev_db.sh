pg_dump -h 10.17.0.105 -U pguser -d historical_septa_dev -Fc -f "$(date +%s)-dev.dump"
