## historical-septa

My WIP aggregator of the information from septa's train status endpoint

## Endpoints

`/api/train/{train number}`  
Query Options:

|key|type|description|
|-|-|-|
| limit    | number {default: 100}            | number of records to return
| before   | unix timestamp {default: null}   | timestamp in seconds to return results before
| after    | unix timestamp {default: null}   | timestamp in seconds to return results after
| order    | asc|desc {default: desc}         | ordering to return results based on received_at timestamp

`/api/current`  
* If `all` is set to false, or omitted, it will only return trains since 2AM on the current day
Query Options:

|key|type|description|
|-|-|-|
|all      |boolean {default: false}         | should return all trains in cache
|line     |string {default: null}           | train line to return results for  
|limit    |number {default: 100}            | number of records to return
