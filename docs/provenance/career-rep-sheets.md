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

## Recorded category and bounty artwork **[verified]**

The existing 2026-10-27 PC reference recording (modded Black Edition, executable SHA-256
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`) was reviewed without starting the game again.
At session time 210.4 s the overview shows crossed checkered flags for Race Events. At 213.4 s the bounty grid shows
the college/tree crest for the first four entries, coastal/sailing crest for the next four, and Rockport/city crest
for the ninth visible entry. The selected fifth entry's detail repeats the coastal crest. This is visual evidence
from the recorded modded reference; it does not independently establish every stock PC runtime path.

Matching texture records were read from the original-style Black Edition install with the existing JDLZ/TPK readers.
The decoded images match the recorded shapes. No extracted image is part of the implementation.

| Texture | Hash | Source package | Size / format |
|---|---|---|---|
| `RACE_EVENTS` | `0xEFC9662E` | `FRONTEND/FrontB.lzc` | 64×64 DXT3 |
| `BOUNTY_ICON_COLLEGE` | `0x8A21B882` | `GLOBAL/GlobalB.lzc` | 64×64 DXT3 |
| `BOUNTY_ICON_COASTAL` | `0x895EC0AE` | `GLOBAL/GlobalB.lzc` | 64×64 DXT3 |
| `BOUNTY_ICON_ROCKPORT` | `0x9129E7FB` | `GLOBAL/GlobalB.lzc` | 64×64 DXT3 |

Spawn numbers 9–12 share the Rockport region. The first visible Rockport entry and installed region-specific texture
support that regional mapping; the recording did not individually select all twelve locations. Installed regressions
check texture decoding and that grid/detail copies track the selected spawn across the scroll window.

## Dead ends

- The pending-jump consumer (`+0x11C` / `+0x111`): not looked for again.
- The value of the race sheet's "Cop Probability" row: `run_script_inline` is disabled (`GHIDRA_MCP_ALLOW_SCRIPTS`), so the
  users of the label constants `0xDF690965` / `0xCE2B53D4` were not scanned for. The labels are the package's own strings.
- The in-game twins differ from the safe house packages in many object names (a diff of the name sets this pass), so they are
  not bound; the pause menu uses the safe house packages.
