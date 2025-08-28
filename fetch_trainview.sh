#!/bin/bash
# for r in {1..10}; do ./fetch_trainview.sh && sleep 1; done
TS=`date +%s`
DTS=`date -r $TS`
echo "Running $TS - $DTS"

function log_output() {
  echo "=======================" >> output.log
  echo "Fetch: $DTS" >> output.log
  while read -r line; do
    echo "$line" >> output.log
  done
}

function save_output() {
  while read -r line; do
    echo "$line" >> responses/$TS.json
  done
}

curl https://www3.septa.org/api/TrainView/index.php 2> >(log_output) > >(save_output)
echo "done."
