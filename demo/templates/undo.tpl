%%# The way back of a fix: the same file, with the values as they are now. Generate
%%# it BEFORE changing anything, as undo.sql.
%%header
-- To undo the changes to the customers of %{#name:line}
BEGIN;
%%record
UPDATE customers SET plan = %{plan}, balance = %{balance} WHERE id = %{id};
%%footer
-- %{count} customers. COMMIT when it looks right.
