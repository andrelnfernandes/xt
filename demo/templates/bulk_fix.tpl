%%# Many fixes as a single UPDATE: the records go into a temporary table, and the
%%# footer joins it. Write it to a .sql; it runs on PostgreSQL and SQLite.
%%header
BEGIN;
CREATE TEMP TABLE fix (id TEXT, plan TEXT);
%%record
INSERT INTO fix VALUES (%{id}, %{plan});
%%footer
UPDATE customers SET plan = fix.plan FROM fix WHERE CAST(customers.id AS TEXT) = fix.id;
SELECT count(*) AS fixed FROM fix;
-- %{count} records read. Check them, then COMMIT or ROLLBACK.
