%%# A table in Markdown, for an issue or a wiki. Write it to a .md.
%%header
# Customers of %{#name}

| Code | Name | City | Balance |
|---|---|---|---|
%%record
| %{code} | %{name} | %{city} | %{balance} |
%%footer

%{count} customers on %{#date}.
