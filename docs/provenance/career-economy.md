# Provenance: the career economy

Addresses, Ghidra names, method and dead ends for [career-economy.md](../specs/career-economy.md) (and the
continuation files if the spec was split). Image base `0x00400000`; program `/speed.exe`;
`D:\Need For Speed Most Wanted Black Edition\speed.exe`. Ghidra auto names are used here only.

## Addresses in `speed.exe`

### Stable and career record

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| FECareerRecord accessor | `FUN_0056f120` | `0x0056f120` | index > 0x19 or byte0 == 0xFF gives null |
| FECarRecord lookup by handle | `FUN_0056ecc0` | `0x0056ecc0` | |
| Record: add pursuit result | `FUN_0056f7c0` | `0x0056f7c0` | (mask, bounty, evaded flag): adds the mask's kinds to the unserved set, then `+0x14++` and `+0x10 += bounty`, or `+0x16++` |
| Mask to counter vector | `FUN_0056f410` | `0x0056f410` | bit n to index n |
| Counter getter by bit | `FUN_0056f510` | `0x0056f510` | switch on 1,2,4,8,0x10,0x20,0x40,0x80 |
| Undo a pursuit (marker) | none (not a defined function) | `0x0056f840` | decrements unserved kinds in mask and `+0x16` |
| Serve all (pay) | none | `0x0056f890` | served += unserved; unserved = 0 |
| Impound set state | `FUN_0056f630` | `0x0056f630` | called on `record + 2`: state, strikes = limit, countdown = 5 |
| Impound tick helper | `FUN_0056f670` | `0x0056f670` | only reached from a debug screen |
| Strike room test | `FUN_0056f710` | `0x0056f710` | limit < 5 and state 0 |
| Release | `FUN_0056f650`, `FUN_0056f660` | `0x0056f650`, `0x0056f660` | clear strikes, state, countdown (same bytes) |
| Raise strike limit | none | `0x0056f730` | limit + 1, capped at the static `0x008f4314` (= 5) |
| Heat set / get | `FUN_0056f7a0`, `FUN_0056f7b0` | | float at `+0xC` |
| Stable ctor / reset | `FUN_005902e0`, `FUN_00590d40` | | the second creates the starter car `M3GTRCAREERSTART` (handle `0x12345678`) |
| Total bounty | `FUN_00590500` | `0x00590500` | live sum + `+0x8ca0` |
| Total evaded / busted | `FUN_00590550`, `FUN_005905a0` | | |
| Total infractions by set | `FUN_00590430` | `0x00590430` | flag 1 = unserved, 0 = served |
| Infractions by kind | `FUN_005903b0` | `0x005903b0` | |
| Visitor helper | `FUN_00580f40` | `0x00580f40` | |
| Visitors (vtables) | data | `0x008a0720`.. `0x008a0750`, `0x008a1140` | 0x8a0720 infractions sum, 0x8a0728 bounty, 0x8a0730 evaded, 0x8a0738 busted, 0x8a0740 impounded flag, 0x8a0748 fine, 0x8a0750 count, 0x8a1140 per kind |
| Sold-history add | none (data misread as code) | `0x00580fd0` | adds record `+0x10..+0x37` into the stable's `+0x8ca0..` |
| Remove car by handle | none | `0x00581370` | (handle, mask, add-to-history); thunks `0x00590d00` (mask 4, no history), `0x00590d20` (mask 2, flag) |
| Impounded-free car count | `FUN_005811a0` | `0x005811a0` | counts cars whose record has state 0 |
| Fine of a counter vector | `FUN_00581cb0` | `0x00581cb0` | sum count x `infractions` amount; the `infractions` instance is built by `FUN_00580660` |
| Car `Cost` | `FUN_00581730` | `0x00581730` | `frontend` class collection of the car key, layout offset `0x4c` |
| Release fee | `FUN_00581760` | `0x00581760` | `Cost` x float at `0x008f4310` (0.25), rounded |
| Cheapest-soonest impounded car | `FUN_00590710` | `0x00590710` | smallest countdown, ties by lowest `Cost`; no direct caller found |

### Tickers and heat factors

| Role | Ghidra name | Address |
|---|---|---|
| Evade tick: strike decay + countdown | `FUN_00580a40` | `0x00580a40` (called from `0x004110f3`) |
| Countdown only, notifies on release (script event `0x78`) | `FUN_00580990` | `0x00580990` (called from `0x00611c1d`) |
| Evade heat factor over all cars | `FUN_00590160` -> `FUN_00581ff0` | `0x00590160` (called from `0x00411211`); factor at layout `+0xa8` of `pursuitlevels/default` |
| Event-win heat factor | `FUN_00590260` -> `FUN_00581f70` | `0x00590260` (called from `0x00611c17`); layout `+0x84` |
| Third factor (milestone) | `FUN_00581fb0` | `0x00581fb0`; layout `+0x60`; no caller found |

### Pursuit end, history and statistics

| Role | Ghidra name | Address |
|---|---|---|
| Pursuit-finished recorder | `FUN_00411240` | `0x00411240`; called once per finished pursuit from `FUN_0043e8d0` at `0x0043eb6e` |
| Pursuit listener (counts in a row, evade time, tick) | not a defined function | `0x00411070`; slot 8 of vtable `0x008918f0` (`0x00891910`); caller not found |
| History add (profile + 0x90e0) | `FUN_00580170`, thunk `FUN_0058fd00` | `0x00580170`, `0x0058fd00` |
| Rank from thresholds | `FUN_005a3c30` | `0x005a3c30` |
| Statistics accessor | `FUN_005999f0` | `0x005999f0` |
| Rankings screen builder | `FUN_0052e750` | `0x0052e750` |
| Vehicle database rows | `FUN_0052d640` | `0x0052d640` |
| Script variable setter | `FUN_005eda20` | `0x005eda20` (named floats on the gameplay manager `0x0091e00c`) |
| Pursuit variables | `FUN_006013f0` | `0x006013f0` (`cost_to_state`, `bounty_in_pursuit`) |
| Infraction manager report | `FUN_005fdda0` | `0x005fdda0` |
| Message struct (kind 0x18) | `FUN_00405960`, post `FUN_00654af0` | channel `0x20d60dbf` |

### Cash and screens

| Role | Ghidra name | Address |
|---|---|---|
| Spend cash (clamps at 0) | `FUN_0056d7a0` | `0x0056d7a0` (on `profile + 0xa8`) |
| Bonus flag, +10000 | `FUN_0056d7c0` | `0x0056d7c0`; caller `0x005185ea` |
| Race finished script hook | not a defined function | `0x006119f0` (`NotifyRaceFinished` registration in `FUN_0061e750`) |
| Event cash value | `FUN_005fb920` | `0x005fb920`; field `CashValue` (`0xd8baa07b`) |
| Reward application | `FUN_005a62d0` | `0x005a62d0`; kind 0x13 = cash |
| Infractions screen constructor | `FUN_0058cef0` | `0x0058cef0` |
| Infractions screen messages | `FUN_005a15b0` | `0x005a15b0` |
| Car select messages | not defined | `0x007ad490` (release, markers); `0x007c1410` region (sell, buy) |
| Cart checkout | `FUN_007bab60` | `0x007bab60` |
| Marker inventory remove / count | `FUN_00576c90`, `FUN_00576f10` | table at `0x0091d928` |
| Script hooks | `FUN_0061e750` | `AwardPlayerBounty` `0x00612220`; `GetPlayerBounty` `0x00612300`; `AwardCash` and `AwardPoints` both end in `0x0060e030`; `StorePursuitRepForFE`, `SetRaceCompleteForFE` are `ret` at `0x00604ca0`, `0x00604c90` |

## Method

- Offsets of the record were read from the rap sheet row builder (`0x0052d640`), which pushes the fields into the
  labels `BOUNTY_%d`, `FINES_%d`, `UNSERVED_%d`, `EVADED_%d`, `BUSTED_%d`, and from the visitor functions.
- Statistic names come from the rankings builder (`0x0052e750`, `0x005a3c30`), whose switch maps index 0..9 to the
  `frontend` collections of the same names.
- Strings were resolved from the label hash with `LANGUAGES/Labels.bin`; collection names from the fine routine's
  literals; fine amounts from `GLOBAL/attributes.bin`.
- Undefined functions: Ghidra did not define several short or LAB-style functions (script handlers, `0x0056f840`,
  `0x0056f890`, `0x00580fd0`, `0x00581370`, `0x00411070`, the car select handler). They were read by disassembling the
  file bytes with a linear sweep. A sweep is wrong when it starts inside data; start only at a `ret`/`int3` boundary.
- Callers of a function that is called from undefined code do not show up in the decompile index; scan the code bytes
  for `E8`/`E9` calls to the address instead.

## Dead ends and open points

Open questions are listed in [career-economy-flows.md](../specs/career-economy-flows.md) section 10.

- **The divisor of `AwardPlayerBounty`.** Handler `0x00612220` loads the pointer at `0x008f5a50` (`PTR_DAT_008f5a50`),
  which points at `0x008f5a4c` (value 5 in the file; `.data` section, writable). Only two references exist: that
  handler and `FUN_005fa430` (`0x005fa430`, constructor of the gameplay manager, called with `GLOBAL\GAMEPLAY.BIN` from
  `0x00665fc0`), which stores `&DAT_008f59f0` and `&PTR_DAT_008f5a50` in two stack slots and feeds their addresses to
  helper functions (`FUN_005f8e30`, `FUN_005f8dc0`, `FUN_005f8ce0`, `FUN_005f8ea0`, `FUN_005f8d50`) built from masked
  multiply-and-xor arithmetic with a trace hook (`0x008fbf98`). It also compares a result with a constant before
  creating the manager. Looks like an obfuscated self-check, not an Attrib default. A set/add/subtract patch routine
  (`FUN_005abfe0`, modes 0/1/2 on a pointed-to int) exists nearby and may be what changes the value. Tried and failed:
  hashing about 47000 strings (exe, strings table, doc names) with lookup2 (two initvals) and the label hash for the keys
  `0x73872468` and `0x69241641` found in the block; neither is in `attributes.bin`, `gameplay.bin` or `FE_ATTRIB.bin`.
  Cheat Engine was not reachable, so no run-time read.
- **Slot `+0x74` of the pursuit interface** (`0x00433890`: `mov al,[ecx+0xe8]`). The only writer found is in an
  undefined function around `0x00443bf0` (`mov [esi+0xe8],al`), computed from the race context; `0x004198e5` sets the
  neighbour `+0xe9`. See spec flows 2.2.
- **Race status object** `0x0091e000`: `+0x1960` race context (0 none, 1 in a race event), `+0x1964` race mode (2 =
  career event in `NotifyRaceFinished`), `+0x1968` current event parameters. Names are ours. The event flag getters are
  `FUN_005fbe70` (`PursuitRace`, hash `0x2b1f54f6`), `FUN_005fc180` (`ChallengeSeriesRace`, `0x1c650104`),
  `FUN_005fc110` (`0xa4e6fcfd`), `FUN_005fc560` (`0x4393f69b`), `FUN_005fc1f0` (`0x637584fe`); names found by a word
  search against `lookup2`.
- **`FUN_00410320` / `FUN_0051e1a0`** are Attrib instance constructors for the class whose key is the first argument of
  `FUN_00455fd0` (`0x551e22b3` `pursuitlevels`, `0x85885722` `frontend`, `0x2870de90` `infractions`).
- Scratch tools used (not in the repo): a Rust program over `libs/blackbox-attrib` and `libs/blackbox-text` to dump
  collections, fields and Lua bytecode blobs; a Python linear-sweep disassembler (capstone) over the file bytes; a
  scanner that finds `E8`/`E9` calls to an address.

## Car Select stat bars (2026-10-18)

Question: what fills the TOP SPEED / ACCELERATION / HANDLING bars in `Car_Select.fng` (spec
[career-car-lot.md](../specs/career-car-lot.md) open question 1). Partial answer only.

- **Sources**: `speed.exe` 1.3 read-only Ghidra (strings, `disassemble_function`), a byte scan of the install's
  `speed.exe` for immediates and E8 call sites, `nfsmw dump-screen Car_Select.fng` output (read only).
- **Verified**: bar groups `0xF1737` / `0xF172F` / `0xF173F` as above; messages `0x26C9925F`, `0x0B9A1B40`,
  `0x06FE3A05` and the start message `0x97BDB465`; label strings at `0x89B2F8`, `0x89B304`, `0x89B310`; setup function
  at `0x007B4D70` with constants only (1.0, 5.0, 160.0).
- **Second pass (orchestrator request, same day)**: xrefs now callable. Per-car refresh `0x007B8F10` (caller
  `0x007BE3F0`) reads a car record through getter `0x007AF220`, which returns one of three floats by mode index
  (0, 1, 2) and a constant otherwise. The per-car loop uses the modes 0 to 6, which fits the "seven floats" in the spec.
  Segment setter `0x007AFF10` clamps a value to min and max and builds 10 child elements per bar. Callers listed in the
  spec.
- **Third pass**: `0x004E4EA0` opens the `pvehicle` class (`0x4A97EC8F`, `docs/formats/attributes.md`) by car key.
  `0x005814E0` searches a 200-entry key table and returns a 408-byte per-car block (index < 76). `0x0067C570` blends
  seven parts (weights table `0x008F8930`, part values from `0x00677F80`, flags hash `0xCDC136E8`) into three floats
  clamped to [0, 1]. `0x007AF220` picks one by mode 0/1/2. `0x007AFF10` takes min and max and builds ten
  `HEAT_BASE_LED_%d` segments. Format strings: `DETAIL_TEXT_LINE%d`, `PERFORMANCE_DETAILS_ICON%d`.
- **Fourth pass (2026-10-18, final)**: weights are seven (x, y, z) rows at `0x008F8928 + 12 i` (the first read at
  `0x008F8930` was off by two floats, corrected). `p_i = installed_i / max_i` via `0x00672D30` / `0x00672E80`
  (`0x00677F80`). `0x0067C570` computes `raw_j = sum_i w_ij p_i / sum_i w_ij` for x, y, z, plus a mask term from
  flag hash `0xCDC136E8` (`0x00454810`), with clamp and smoothing as written in the spec. `0x007AFF10` is the bar
  constructor (lo, hi, current, target clamped into [lo, hi]; ten `HEAT_BASE_LED_%d` children), not the lit-count
  rule. The constructor call in `0x007B4D70` uses lo 1.0, hi 5.0 on the object at `+0x88`.
- **Still open**: the lit-segment rule, the bar-object to label and axis map, the name of hash `0xCDC136E8`, the two
  getters `0x00679660` / `0x006796C0`, and the category index map. No Rust crate is touched; the repo's category
  order is used only as a guess.
- **Status**: UNRESOLVED (partial). Next: `0x00672D30`, `0x00672E80` (what installed and max count are), the bar
  update function that reads the current value, and the hash name.
- **Not found**: the per-car value source, the seven-float layout, the formula and the range. Message hashes are not
  immediates in the exe (no direct `push` of them), so their handlers are reached through FEng, not by code search.
- **Tools gap**: Ghidra xref tools (`get_xrefs_to`) were not callable in this session.
- **Status**: UNRESOLVED. Next: find the car-select object writer and the handler scripts.

## Markers and the reward inventory (2026-10-18)

Question: where the markers (0x11, 0x14, 0x15) live. Answer and evidence are in
[career-economy-flows.md](../specs/career-economy-flows.md) 6.5 and [career-save.md](../formats/career-save.md) 3.2.

- Inventory: `0x0091d928`, 63 x 12 bytes `{type, aux, flag}`, saved at file `0x5739` (756 bytes). Writers and readers:
  `FUN_00576c50` (add), `FUN_00576c90` (remove), `FUN_00576f10` (count). Save `FUN_0057f7b0`, load `FUN_005ac950`.
- Popup queue: separate 6 x 12 bytes at `0x0091dc1c`, count `0x0091dc64`, not saved. Producer `FUN_00576b80`. Consumer not
  found.
- Reward kinds: name table at `0x008f5480`, lookup `FUN_00577590`, applied by `FUN_005a62d0` (caller `0x0062aa90`, undefined
  in Ghidra; flag from `FUN_005fbe00`, meaning not checked).
- Undefined in Ghidra (read by linear disassembly, not defined): `0x005a15b0` (infractions messages), `0x007ad490` (car select
  messages), `0x0062aa90` (reward caller), `0x008866b0` (reset), `0x007b3dd3` (generic add from a list). Start addresses checked
  by the `8B 44 24 04 3D ...` prologue scan and the `ret`/`int3` boundary.
- Scratch tools (not in the repo, `python -I`, read-only): a PE section mapper, a byte-pattern and `E8`/`E9` call scanner,
  and a parser for the 0x5739 block of the three saves. Save files were read only; no save bytes are copied.
- Dead end: the Ghidra decompile index does not list `0x005a15b0`, `0x007ad490`, `0x0062aa90`, `0x008866b0`; `force_decompile`
  fails on them and `disassemble_function` only works where a function boundary is known.
