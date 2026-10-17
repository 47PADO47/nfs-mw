# Provenance: the rap sheet and the reputation sheets

Addresses, Ghidra names, method and dead ends for [career-rapsheet.md](../specs/career-rapsheet.md) and
[career-rapsheet-2.md](../specs/career-rapsheet-2.md). Ghidra auto names (`FUN_*`, `DAT_*`, `LAB_*`, `PTR_*`) are used
here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3).

## Method

1. The full decompile (every function) was searched by regex for strings, hashes, offsets and callers. The screen
   family is registered in a table of `{name pointer, constructor stub}` pairs at `0x008F43F0` to `0x008F4510`; each stub
   allocates the page object and calls the class constructor, which fills the page from the profile.
2. Message handlers (vtable slot 1 of the page classes), the per-category fill routines and most item handlers are **not
   defined as functions** in the project (they show up as `LAB_*` or sit inside other functions). They were read by
   disassembling the bytes with Capstone from the file (PE section map). Vtables were read from the data. Callers of an
   undefined routine were found by scanning `.text` for `call rel32`.
3. Decompiled calls to the formatter lose their variadic arguments (the values pushed above the format). Every fill routine was
   therefore re-read in disassembly to see which statistic or total feeds each object.
4. FEng objects are known only as name hashes. The engine hash (`h = h*33 + byte`, seed `0xFFFFFFFF`, upper-case names)
   was applied to every ASCII string of `speed.exe` and to the `%d` / `%.2d` expansions 0 to 40 of every format string
   (`CAR_%d`, `RANK_%d`, `BUTTON_%d`, ...); that resolved the row and button objects of every page. Single objects
   (`0x1232703A` case file, `0xE3DA78E7` name) were matched through the constants in the fill routines.
5. The package contents (objects, scripts, events, package responses) came from `nfsmw dump-screen NAME.fng` (the name
   needs the `.fng` suffix; `list-screens` lists them) on the install. The tool must always be given a sub-command:
   started without one it opens the game window. The animation timelines were obtained by following the script
   events (message to target at a time offset) with a small event simulator; objects are started by the package
   `INIT` script and message chains. Timelines match the packages, not a run.
6. Language labels were resolved with the install's `English.bin` rows (name, hash, text); attribute collections
   (`rapsheet_summary_strings`, the 20 ranking tables) were dumped with `libs/blackbox-attrib` from `FE_ATTRIB.bin`.
7. No run-time tool was available; every claim is a static reading. Unclosed items are tagged **[unconfirmed]** in the specs.

## Entry, flow and shared helpers

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Screen registration table | data | `0x008F43F0` | `RapSheetLogin`/`Login2` share stub `0x00570F60`; `Main` `0x00570FC0`; `RS` `0x00571020`; `US` `0x00571080`; `VD` `0x005710E0`; `CTS` `0x00571140`; `TEP` `0x005711A0`; `PD` `0x00571200`; `Rankings` `0x00571260`; `RankingsDetail` `0x005712C0`; `SafehouseReputationOverview` `0x00571320`; rival/bounty/milestone stubs `0x00571380` to `0x005715C0`; `GameWon` and the three `_ENDGAME` pages share `0x005715C0` |
| Safe house menu builder | `FUN_00532220` | `0x00532220` | adds "Rap Sheet" only if settings flag bit 4; fills `CAREER_DATA`, `BOUNTY_DATA`, `CASH_DATA` |
| Rap Sheet item select | undefined | `0x005320E0` | sets context bit `0x200`, pushes `RapSheetLogin.fng` (string `0x0089D9A8`) |
| `SetHasRapSheet` | `LAB_00605670` | `0x00605670` | `flags |= 0x10`; registered at `0x0061E750` |
| Push a screen | `FUN_00525940` | `0x00525940` | (manager `0x0091CADC`, name, arg, 0, 0) |
| Set/get remembered selection | `FUN_00571eb0` / `FUN_00571e80` / `FUN_00571e00` | `0x00571EB0` / `0x00571E80` / `0x00571E00` | table `0x0091CFC0` .. `0x0091D218`, 12 bytes per entry |
| Format a name from a pattern | `FUN_00573140` | `0x00573140` | `printf` into a 256 byte buffer, then the engine hash (`FUN_00460BF0`) |
| Select a button by name hash | `FUN_00515e90` | `0x00515E90` | |
| Set a string object | `FUN_005252d0` / `FUN_00525220` | `0x005252D0` / `0x00525220` | (package, object hash, format, args...) / (package, object hash, label hash) |
| Language lookup | `FUN_0056bc10` (`FUN_0056bb80`) | `0x0056BC10` | binary search over `0x0091CF80`; fallback label `0x9BB9CCC3` (`ERROR_DEFAULT_STRING`) |
| Profile name accessor | `FUN_0056db00` | `0x0056DB00` | returns the profile pointer (the name is the first member) |
| Scroller list base | `FUN_005938a0` | `0x005938A0` | (this, package, columns, rows, flag); base message handler at `0x005A58B0` (scroll); clear `0x00593880`, add `0x0059BD30`, refresh `0x00593560` / `0x00593910` |

## Pages

| Page | Class ctor | Vtable | Handler | Fill | Notes |
|---|---|---|---|---|---|
| `RapSheetLogin`/`Login2` | `FUN_00546510` | `0x0089E5B4` | `0x0052C670` | ctor | mode at `+0x2C`, Back flag `+0x30`; sounds via `0x004AE940` (ids `0x67`, `0x68`); format `> %s` at `0x0089D398` |
| `RapSheetMain` | `FUN_005465C0` | `0x0089E5C0` | `0x0052C790` | `FUN_0052ca00` | jump table `0x0052C9E0`; `BUTTON_%d` string `0x0089D408` |
| `RapSheetRS` | `FUN_00546650` | `0x0089E5D0` | `0x0052CB80` | `FUN_0052cbc0` | summary index loop at `0x0052CE02` to `0x0052CEF0`; `RAP_SUMMARY_%.2d` at `0x0089D414` |
| `RapSheetUS` | `FUN_005466D0` | `0x0089E630` | `0x005469D0` | `FUN_0052cf40` (rows), `0x0052D420` (header) | rows are 0x24-byte data nodes (vtable `0x0089C2C0`), slot vtable `0x0089C2C8`, show `0x0051E2F0`; toggle `0x0052D5C0`; view byte `+0xE0` |
| `RapSheetVD` | `FUN_00546A50` | `0x0089E65C` | `0x0052D5E0` | `FUN_0052d640` | data nodes vtable `0x0089C2D4`, slot vtable `0x0089C2DC`, show `0x0051E480` |
| `RapSheetCTS` | `FUN_0052da20` | `0x0089D504` | `0x0052DD20` | `FUN_0051e6e0` (rows), `0x0052DD80` (header) | slot vtable `0x0089C2EC`, show `0x0051E660`; unit prices in `FUN_005a3f10` |
| `RapSheetTEP` | `FUN_00546fc0` | `0x0089E668` | `0x0052DE40` | `FUN_0052e0c0` | 5 buttons, jump table `0x0052E0A0` |
| `RapSheetPD` | `FUN_00547050` | `0x0089E678` | `0x0052E410` | `FUN_0052e450` | entry index u16 at `+0x2C` |
| `RapSheetRankings` | `FUN_00550dc0` | `0x0089EAF8` | `0x00550E50` | `FUN_005470e0` + `FUN_0052e6c0` (cell) + `FUN_0052e5f0` (header) | jump table `0x0055102C`; view byte `0x0091CA34` |
| `RapSheetRankingsDetail` | `FUN_005471b0` | `0x0089E6F0` | `0x005475A0` | `FUN_0052e750` | timer slot vtable `0x0089C318` (show `0x0051ED90`), plain slot `0x0089C310` (show `0x0051ECA0`); highlight `0x0051EE90`; view byte `0x0091CA35` |
| End-game pages and `GameWon` | `FUN_00532b90` | `0x0089B214` | `0x00549380` | ctor | step routine `0x00532DD0` (jump table `0x00532E5C`); counter `0x0091CA54`; reset `FUN_00511fe0` called from `0x0062412F` |
| Credits end hook | undefined | `0x005332BB` | | | calls the step routine when bit 18 is set |

## Numbers

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Statistic accessor | `FUN_005999f0` | `0x005999F0` | 8 = `FUN_00590430(1)` + `FUN_00590430(0)`, 9 = `FUN_00590500` |
| Stable visitor | `FUN_00580f40` | `0x00580F40` | live-car test; visitor vtables `0x008A0720` (infractions), `0x008A0728` (bounty), `0x008A0730` (evaded), `0x008A0738` (busted), `0x008A0740` (impounded), `0x008A0748` (fines), `0x008A0750` (monitored), `0x008A1140` (per kind) |
| Infractions total / by kind | `FUN_00590430` / `FUN_005903b0` | `0x00590430` / `0x005903B0` | sold tail `+0x8CA8` / `+0x8CB8`; `FUN_0056f510` maps a mask to the u16 index |
| Fine of eight counters | `FUN_00581cb0` | `0x00581CB0` | amounts looked up by name (`assault`, `damage`, `hit_and_run`, `off_road`, `racing`, `reckless_driving`, `resisting_arrest`, `speeding`) |
| Per-car counts | `FUN_00580ea0` | `0x00580EA0` | sum of the eight unserved / served counters of one car |
| Total bounty, evaded, busted, impounded, fines, monitored | `FUN_00590500`, `FUN_00590550`, `FUN_005905a0`, `FUN_005905f0`, `FUN_00590630`, `FUN_005906b0` | `0x00590500` ... `0x005906B0` | |
| Car name label | `FUN_00591160` / `FUN_005913b0` | `0x00591160` / `0x005913B0` | `CARNAME_%s_%s`, maker from `FUN_00581790` |
| History recorder | `FUN_00580170` | `0x00580170` | via `FUN_0058fd00` from the pursuit-end recorder `FUN_00411240` |
| Cost to state lines | `FUN_005a3f10` | `0x005A3F10` | switch 0 to 9, unit prices 2000, 450, 250, 5000, 250, 500, 250, 225, 500, direct pair |
| Rank | `FUN_005a3c30` | `0x005A3C30` | collection names by statistic and mode; the `*4000` for statistic 0 is a hidden x87 operand in the decompile |
| Time formatter | `FUN_006547a0`, `FUN_00654710`, `FUN_006541b0` | `0x006547A0`, `0x00654710`, `0x006541B0` | 4000 ticks per second; language switch `FUN_0056bb20` |
| Float to int | `FUN_007c4b80` | `0x007C4B80` | truncating conversion |
| Constants | data | `0x00890984` (0.00025), `0x00893718` (4000.0), `0x008933D4` (0.5), `0x008970EC` (1/140), `0x00890FC8` (100.0), `0x00898DC8` (0.2), `0x008942D8` (0.1), `0x008A2A40` (0.7) | |
| Completion percentages | `FUN_005a8780` | `0x005A8780` | loops bins 1 to 16 (`0x0091E004`), all race records, milestones (`0x0091E00C` + `0xA4`) |
| Main menu card | `FUN_00528bc0` | `0x00528BC0` | caller `FUN_00544e50`; percent string `0x0089CEEC` / `0x0089CEF0` chosen by language id |

## Reputation sheets

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Overview class | `FUN_00561130`, fill `FUN_00551070`, handler `LAB_0055a4d0` | `0x00561130`, `0x00551070`, `0x0055A4D0` | entries built with vtable `0x0089C334`; panel `FUN_0052ee50` |
| Race sheet class | `FUN_0055a760`, fill `FUN_00547e30`, handler `LAB_00551530` | `0x0055A760`, `0x00547E30`, `0x00551530` | mode bytes `+0x1B4` (in game), `+0x1B5` (viewed bin only); selected event global `0x0091CA40` |
| Milestones class | `FUN_0055a9a0`, fill `FUN_00552440`, handler `LAB_00551f80` | `0x0055A9A0`, `0x00552440`, `0x00551F80` | selected row global `0x0091CA44`; datum builders `FUN_0051f310` / `FUN_0051f3c0` |
| Bounty class | `FUN_0055aca0`, fill `FUN_005529f0`, handler `LAB_00552570` | `0x0055ACA0`, `0x005529F0`, `0x00552570` | tier labels `FUN_0056de70`, row text `FUN_0057fe00` |
| Pending jump | manager fields | `0x0091E00C` + `0x111`, `0x114`, `0x11C`; `+0x2B4`, `+0x2B5`, `+0x2B8` | written at `0x00552882` (and the milestone twin); jump routine `FUN_005ebc80`; `+0x111` read at `0x0041EED0`, `0x0042EA90` |
| Viewed bin | data | `0x0091CA3C` | set by the safe house item "Blacklist N" |

## Dead ends

- The consumer of manager `+0x11C` was not found by regex on the decompile; the field is probably read through an
  undefined routine of the world load.
- The pursuit-side implementation of the interface slots `+0xA8` ... `+0xC8` (a vtable with at least 54 slots) was not located:
  a scan for vtables whose slot `+0x74` reads a byte at `+0xE8` returned nothing, and no decompiled function reads the
  eight slots together.
- The end of `Main_ENDGAME`: the package scripts were followed only up to the point where input is awaited.
- The race sheet launch chain (`FUN_005fdcb0`, `FUN_00603440`, `FUN_00600ab0`, `FUN_0057eca0`) was only read as calls.
- The reader of the context bit 9 beyond the audio state chooser `FUN_004df6d0` (state 13).
- The meaning of the front-end database byte `+3` that hides the main menu card.
