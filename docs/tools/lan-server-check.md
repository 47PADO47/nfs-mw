# lan-server-check

Checks an installed game's `server.dll` against the patch set used by the MIT-licensed NFSLAN launcher, and
prints the timing and service keys from `server.cfg`. Read-only. Background and the reverse-engineering
notes are in [specs/lan-server.md](../specs/lan-server.md).

```bash
python tools/lan_server_check.py "D:\Need For Speed Most Wanted Black Edition"
```

What it reports:

- whether `server.dll` exports `StartServer`, `IsServerRunning` and `StopServer`;
- for each of the ten patch patterns, the number of matches and the address (must be exactly one match at
  the expected address);
- the `server.cfg` keys `PINGTIME`, `PINGWAIT`, `TIMEIDLE`, `TIMEMAXM`, `LAN_THROTTLE`, `TRUST`, `ADDR`,
  `ACCOUNT`, `MASTER`, `SLAVE` and `REDIR`.

Exit status is `0` when the patch set matches, `2` when it does not, and `1` when `server.dll` is missing.

Standard library only; tested with Python 3.10+ on a Black Edition install (`server.dll` dated 2005-11-15).
