# nushell-history-skim

Prototype of enhanced history search widget for nushell sqlite history backend using skim. Since it uses skim it probably won't work on Windows (for now). Shows juicy details of ran commands, allows filtering by current working directory.

Heavily inspired by [zsh-histdb-skim](https://github.com/m42e/zsh-histdb-skim).

## Importing from zsh-histdb

```sql
attach '.histdb/zsh-history.db' as zsh;
attach '.config/nushell/history.sqlite3' as nu;

update nu.history set session_id = (select max(session) + 1 from zsh.history);
update nu.history set id = id + (select max(id) + 1 from zsh.history);

insert into nu.history (id, command_line, start_timestamp, session_id, hostname, cwd, duration_ms, exit_status, more_info)
select
	history.id as id,
	argv as command_line,
	start_time * 1000 as start_timestamp,
	session as session_id,
	host as hostname,
	dir as cwd,
	duration as duration_ms,
	exit_status,
	json_object('src', 'zsh-histdb') as more_info
from zsh.history
join zsh.places on places.id = history.place_id
join zsh.commands on commands.id = history.command_id;
```
