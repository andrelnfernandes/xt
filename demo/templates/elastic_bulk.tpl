%%# Two lines per record: the format of Elasticsearch's _bulk. Write it to a .ndjson,
%%# so that values come out as JSON.
%%record
{"index": {"_index": "customers", "_id": %{id}}}
{"name": %{name}, "email": %{email}, "city": %{city}, "balance": %{balance}, "active": %{active}}
