# Provenance: the reputation sheets and the saved Blacklist progress

Method and addresses for [career-rep-sheets.md](../specs/career-rep-sheets.md). Image base `0x00400000`; program
`/speed.exe` through the Ghidra MCP (read only); the packages with `nfsmw dump-screen`; the data with a throwaway real-install
test of `nfsmw-data`.

## Race sheet fill `FUN_00547E30` **[exe]**

Decompiled this pass. After a reset of the page (`FUN_00593880`) it fills the row list in one of two modes, chosen by the byte
`+0x1B5` of the page:

- clear: for every bin from the career's stage byte (`*(DAT_0091CF90 + 0x10) + 0xB0`) up to the highest bin (`FUN_005DC950`),
  `FUN_005E3980` gets the bin, then for each event index `FUN_00601060(i)` gives the event key, `FUN_005FB710` the race, and
  when `FUN_0051F120` accepts it a row is added (`FUN_0045D3A0`, `FUN_00588250`) with its byte `+0x16` (done) set to 0;
- set: the same for the viewed bin `DAT_0091CA3C` only, and the byte `+0x17` is set to 1 when `FUN_005DCA70(key, 2)` says the
  record has flag 2.

It then stores the name hash `0x9BA78BA2` (the track name string) at `+0x54` and, with a map object present, calls
`FUN_00546400`, which stores the map settings and loads `TRACKS\L2RA\TrackMaps.bin` (once) through `FUN_00508B10`. That is
where the city map pictures `TRACK_MAP_<event>` come from; the rewrite reads them with the track select's loader.
The handler `0x00551530` has no function in Ghidra (`get_functions` fails there), as with the other handlers of the
registry.

## Packages **[verified]**

`nfsmw dump-screen` of the three packages: the same counter, map, scroll arrow and icon grid objects; the race sheet's detail
panel is the group `0xCAA242D8` (kind `0x9BA78BA2`, track `0x0F2CD475`, best time `0x8FD41BB4`, cash `0xFFBBBF24`/`0x03A80495`,
distance `0x080C9DAA`, laps `0x0018B36F`, kind icon `0xF97EC5D5`); the milestone and bounty packages have the header
`0xCFA23A2D` (shadow `0xB5117FDE` or `0x2D907846`) and the description `0x028049D6`; only the milestone package has the bounty
reward `0xB21D69BD`. The race package has no "locked" panel (that is the track select's `0x6B67D70B`). The icon grid lies at
y = +593, out of the screen until a message moves it. A real-install test (`rep_sheets/install_tests.rs`) keeps the object ids
honest.

## Data **[verified]**

- `Milestone::spawn` and `SpeedTrap::spawn`: the `SpawnPoint` key of every one of the 55 milestones and 39 traps names a
  collection with a `Position`.
- `GameplayData::bounty_jumps`: 12 collections `bounty_jump/<place>`; the field `0xDB89AB5C` gives 1 to 12, in the order of the
  labels `BLACKLIST_BOUNTY_SPAWN_POINT_01` to `_12`; every one has a position.

## Findings about the rewrite

- The rival challenge button started its boss race with `RaceOptions::default()` (`career: false`), so the result did not
  reach the Blacklist. It and the sheets now start a career race (`start_career_race`).
- `Career::from_profile_bytes` rebuilt no bin state and no race count: a reloaded career had every bin unentered. The races won
  are written to the race records (flag 2) and read back (`Blacklist::restore_progress`).

## Dead ends

- The pending-jump consumer (`+0x11C` / `+0x111`): not looked for again.
- The value of the race sheet's "Cop Probability" row and the source of the average and top speed.
