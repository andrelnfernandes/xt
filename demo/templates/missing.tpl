%%# Changes nothing: lists the customers of this file that a database does not have.
%%# Write it to a .sql, and run it in the database you want to check.
%%record
SELECT %{id} AS missing WHERE NOT EXISTS (SELECT 1 FROM customers WHERE CAST(id AS TEXT) = %{id});
