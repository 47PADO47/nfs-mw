# Provenance: the career text

How [career-text.md](../specs/career-text.md) and [career-text-2.md](../specs/career-text-2.md) were made: the files read,
the decoder, the label hash scheme, the exe addresses and the dead ends. Ghidra auto names (`FUN_*`, `DAT_*`) are used here
only. Image base `0x00400000`; program `/speed.exe`.

## Files and tools

| File | Use |
|---|---|
| `LANGUAGES/English.bin` | 4,586 strings (the installed table) |
| `LANGUAGES/Labels.bin` | the label names of 4,546 hashes |
| `LANGUAGES/ENGLISH.BIN.bacc` | the 2005 table, 4,546 strings; compared with the installed one: no differences; the 40 extra strings of the installed file are add-on car and part names |
| `GLOBAL/fe_attrib.bak`, `GLOBAL/gameplay.bin` | rap sheet thresholds, rival collections, preset names, milestone labels |
| the three saves | SMS slots, the shop unlock table, the flags |

**Decoder:** a private Python script (kept in the scratchpad, not in the repo) written from the format of
[text.md](../formats/text.md): the bChunk walk to the `0x00039000` payload, the header, the sorted `{hash, offset}`
records, and the packed-string unpack with the histogram table and the two-level prefix. It decodes all 4,586 strings
and all 4,546 label names. Spot checks against `nfsmw strings 0x<hash>` agreed (for example `0x01BD185C` = "Resume Free
Roam"). The repo's `nfsmw strings` command was used only to cross-check single labels.

**Label hash:** `bStringHash`: `h = 0xFFFFFFFF; for each byte c: h = h * 33 + toupper(c)`. The exe function is
`FUN_005af1c0` (`0x005AF1C0`). All 4,546 names of `Labels.bin` hash to the key of their own record with this function. The
`printf` helper `FUN_00573140` (`0x00573140`) formats a name and returns its hash; the index argument is passed by the
caller (not visible in the decompile).

**Preset names:** the preset handles in the stable are `bStringHash` of the preset name; the names were found by hashing
candidates built from the car list (`BONUS_<car>`, `CE_<car>`, the `BL<n>` ranks) and a short brute force for three-letter
suffixes. Two handles stay unnamed.

## Addresses

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Rival bio screen fill | `FUN_0052fa50` | `0x0052FA50` | `NAME`, `AKA`, `BIO`, `CAR`, `STRENGTH`; title from `0x3A64DE21` / `0x96CA2471` |
| Rival challenge screen | `FUN_0052ee50` | `0x0052EE50` | `CHALLENGE` and `AKA` |
| Rankings / race lists | `FUN_0052e750`, `FUN_00600730` | | `AKA`, `CAR` (`%.2d` form) and `LEADERBOARD` |
| Arrival stepper | `FUN_005302c0` | `0x005302C0` | `blacklist_%02d` movie |
| Spawn point and milestone label makers | `FUN_0057fe00`, `FUN_0057fe20`, `FUN_0057fe40`, `FUN_0057fe60` | `0x0057FE00..` | the four `%02d` formats |
| Milestone label users | `FUN_005a2390`, `FUN_005a28e0` | | `_SHORT` labels in the milestone lists |
| Rap sheet summary page | `FUN_0052cbc0` | `0x0052CBC0` | the field map of career-text.md section 6; `rapsheet_summary_strings` thresholds |
| Rap sheet counters | `FUN_005a3c30`, `FUN_005a3f10` | | statistic names and counter offsets |
| SMS list item | `FUN_00549150` | `0x00549150` | `SMS_MESSAGE_%d`; `FUN_0056d750` = story / non-story class by id |
| SMS delivery | `FUN_005f5120`, `FUN_005ddd20`, `FUN_005ec160` | | ids 10 to 14, 95 and 96 at once |
| SMS slot init | `FUN_0057f4d0` | `0x0057F4D0` | writes the id or 0xFF from `SMS_MESSAGE_%d` |
| Car name / maker labels | `FUN_00591160`, `FUN_00591330` | | `CARNAME_%s_%s`, `CARSELECT_MANUFACTURER_%s` |
| Car select bonus messages | `FUN_007a3c00`, `FUN_007a3c70` | | message hash per preset; rival rank per preset |
| Car select tabs | `FUN_007ada50` | `0x007ADA50` | tab labels `CARLIST_*` by mask |
| Car select purchase handler | `0x007C1770` (undefined in Ghidra) | | dialogs; hashes in [career-car-lot.md](../specs/career-car-lot.md) |
| Reward kinds | table at `0x008F547C` | | `{kind, part, id}`; ids 1 to 0x15 |
| Part name table | table at `0x008F5580` | | 18 `{name, id}` pairs for the shop unlock table |
| Shop unlock table | `0x0091CD10` | | 57 x 8 bytes; helpers `FUN_00576410`, `FUN_00576560`, `FUN_00576670`, `FUN_00577680`, `FUN_005768a0`; maxima at `0x0091CCC0`, set by `FUN_00576340` |
| Loading tip table | `0x008F3D78` | | 27 tip entries of 16 bytes |
| Main menu | `FUN_00544e50` | | button labels |
| Safe house menu | `FUN_00532220` | | button labels |

## Method notes

- Every label family in the task list was found as a printf format in the exe's read-only data and traced through
  `get_xrefs_to`; the screen that uses the family then gave the target field.
- The SMS ids were resolved by listing `SMS_MESSAGE_<n>`; the 95 present ids equal the 95 valid slots of the three saves.
  The rival tables' phone/message ids of [career-blacklist.md](../specs/career-blacklist.md) were checked by reading the
  sender and subject of each: they match the rival (so the "EntryCellCallID" and "SMSRivalChallenge" fields are SMS ids).
- The 57-entry table was first thought to be hint counters ([career-save.md](../formats/career-save.md) in its first
  version). Tracing the three functions that read it (`FUN_0058a020`, `FUN_00576560`, `FUN_00576670`) and the reward
  function that writes it showed that it is indexed by part category, so it is the shop unlock table. The save page was
  corrected.

## Dead ends and caveats

- The `MARKER_NAME_*` labels were matched to the popup types by the reward table's part names; categories 15 (decals) and
  16 (paint) have no `MARKER_NAME_*` label, only `CB_MARKERS_*` counters.
- The loading tip mask words (second field of each tip entry) were not decoded.
- `RAP_SUMMARY_%.2d`: the printed index is hidden in the variadic call; the array was read but the +1 is a guess.
- Only English was read. The other language files are not in this install (only `English.bin`).
- Which events send SMS ids outside the bin table was not traced (they come from the Lua handlers).
