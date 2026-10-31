%%# A page with a table. Write it to a .html, so that values are escaped for HTML.
%%header
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>%{#name}</title>
<style>
  body { font-family: sans-serif; margin: 2rem; }
  table { border-collapse: collapse; }
  th, td { border: 1px solid #bbb; padding: .3rem .6rem; }
  th { background: #eee; }
</style>
</head>
<body>
<h1>Report of %{#name}</h1>
<p>Generated at %{#datetime}, with the template %{#template}.</p>
<table>
<tr><th>#</th><th>Code</th><th>Name</th><th>City</th><th>Plan</th><th>Balance</th></tr>
%%record
<tr><td>%{#}</td><td>%{code}</td><td>%{name}</td><td>%{city}</td><td>%{plan}</td><td>%{balance}</td></tr>
%%footer
</table>
<p>%{count} customers.</p>
</body>
</html>
