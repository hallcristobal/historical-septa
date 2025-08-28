create table fetches (
  id uuid primary key,
  timestamp timestamp not null,
  status varchar not null,
  result varchar 
);

create table files (
  id uuid primary key,
  received_at timestamp not null,
  contents Text not null
);

create table records (
  id uuid primary key,
  file_id uuid not null,
  trainno varchar not null,
  service varchar not null,
  dest varchar not null,
  currentstop varchar not null,
  nextstop varchar not null,
  line varchar not null,
  consist varchar not null,
  late int not null,
  source varchar not null
);
create index records_trainno_idx on records(trainno);
create index records_file_id_idx on records(file_id);

create table changes (
  id uuid primary key,
  trainno varchar not null,
  record_id uuid not null,
  changed_at timestamp not null,
  field varchar not null,
  old_value varchar,
  new_value varchar,
  type varchar not null
);

create index changes_trainno_idx on changes(trainno);
