## historical-septa

My WIP aggregator of the information from septa's train status endpoint

## Endpoints

`/api/train/{train number}`  
Query Options:

|key|type|description|
|-|-|-|
| limit    | number {default: 100, range: [1, 300]} | number of records to return
| before   | unix timestamp {default: null}         | timestamp in seconds to return results before
| after    | unix timestamp {default: null}         | timestamp in seconds to return results after
| order    | asc\|desc {default: desc}              | ordering to return results based on received_at timestamp

`/api/current`  
* If `all` is set to false, or omitted, it will only return trains since 2AM on the current day
Query Options:

|key|type|description|
|-|-|-|
|all      |boolean {default: false}               | should return all trains in cache
|line     |string {default: null}                 | train line to return results for  
|limit    |number {default: 100, range: [1, 300]} | number of records to return

`/api/query`  
Query Options:

|key|type|description|
|-|-|-|
| limit    | number {default: 100, range: [1, 300]} | number of records to return
| before   | unix timestamp {default: null}         | timestamp in seconds to return results before
| after    | unix timestamp {default: null}         | timestamp in seconds to return results after
| order    | asc\|desc {default: desc}              | ordering to return results based on received_at timestamp

Body Schema:  
Format Requirement: `JSON`

|key|type|description|
|-|-|-|
|  id           |  UUID {optional}            | Id Record
|  file_id      |  UUID {optional}            | Id of Record's creating file
|  timestamp    |  unix timestamp {optional}  | Unix timestamp of Record
|  trainno      |  string {optional}          | Septa Train Number
|  line         |  string {optional}          | The Line that the train services (note: Trains that start on one line, but continue to another might arbitrarily change "line" value at any point)
|  consist      |  string {optional}          | The cars that make up a given train
|  late         |  number {optional}          | The reported late time of the train
|  service      |  string {optional}          | The type of Service rendered (ex: LTD, EXPRESS, LOCAL)
|  currentstop  |  string {optional}          | The most recent reported stop for a train
|  nextstop     |  string {optional}          | The reported next stop for a train
|  source       |  string {optional}          | The starting stop of the train
|  dest         |  string {optional}          | The target ending stop for given train
