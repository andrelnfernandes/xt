%%# A log becomes a database. Open access.log, press & and filter with the regular
%%# expression of demo/README.md, then press Ctrl+B and write it to a .sql. Then:
%%#   sqlite3 log.db < log.sql
%%#   sqlite3 log.db 'SELECT status, count(*) FROM hits GROUP BY status'
%%header
BEGIN;
CREATE TABLE hits (line INTEGER, ip TEXT, method TEXT, path TEXT, status INTEGER);
%%record
INSERT INTO hits VALUES (%{#line}, %{ip}, %{method}, %{path}, %{status});
%%footer
COMMIT;
