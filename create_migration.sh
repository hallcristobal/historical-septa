MESSAGE="$1"
if [ "$MESSAGE" == "" ]; then
  echo "Missing migration description"
  exit 1;
fi
touch "migrations/$(date +%s)_$MESSAGE.sql"
