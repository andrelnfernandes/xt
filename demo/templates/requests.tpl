%%# A curl per record. Write it to a .sh, read it, and only then run it: Xylem does
%%# not make it executable. Values are quoted for the shell, and :url escapes them
%%# inside an address. $API_URL is read when the script runs.
%%header
set -eu
%%record
curl -sS -X PUT "$API_URL/customers/%{id:url}?plan=%{plan:url}"
%%footer
echo 'Sent %{count} requests.'
