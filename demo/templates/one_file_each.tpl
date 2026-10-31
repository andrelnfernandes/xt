%%# One file for each record, like a mail merge: run the script in an empty folder.
%%# Write it to a .sh. The %%s is printf's: %% is how a template writes a %.
%%record
printf '# %%s\n\nCity: %%s\nPlan: %%s\n' %{name} %{city} %{plan} > customer-%{id}.md
