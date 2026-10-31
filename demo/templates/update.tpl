%%# One UPDATE per record, in a transaction left open for you to check.
%%# Write it to a .sql. For MySQL or MariaDB, use %{plan:mysql} and %{id:mysql}.
%%header
-- Generated at %{#datetime:line} from %{#name:line}, template %{#template:line}
BEGIN;
%%record
UPDATE customers SET plan = %{plan} WHERE id = %{id} AND code = %{code};
%%footer
-- %{count} updates. Check them, then COMMIT or ROLLBACK.
