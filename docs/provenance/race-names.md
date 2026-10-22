# Racer names (`crates/nfsmw/src/scenes/world/race/names.rs`)

- **Specs:** [ai-racers](../specs/ai-racers.md) (names), [racer-names](../specs/racer-names.md) (fixed names),
  [modding content](../modding/api-content.md#racer-names).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: nothing was renamed, retyped or changed in the
  project) and the install's `LANGUAGES/English.bin` through the repository's own reader (**[verified]**).
- **Evidence tags:** [docs/README.md](../README.md#evidence-tags).
- **Implemented:** 2026-10-21.

## What the executable does **[exe]**

- The string `RACERNAME_%03d` (`0x008A3914`) has one user, `0x005DC800`, and `RACERNAME_000` (`0x008A3924`) is looked up
  there first: when the language table has no such label the racer is called `UNKNOWN`.
- Otherwise the function draws a random number below 150 (`0x96`), formats the label, hashes it (`bStringHash`) and reads the
  text. It then compares the text with the name of every racer already in the race status (records of `0x194` bytes, the
  count at `+0x1958`, the name at `+0x08` of a record) and draws again while one has it.
- The caller is the routine that adds a racer to the race status (`0x00601F90`, reached from the script binding table
  `0x0061E750`). A racer record that the first check (`0x00600730`) accepts keeps the name it has; online games take the
  name from the connection; every other racer, which is every computer-driven one, gets a drawn name.
- The career names are fixed before the draw (second reading, 2026-10-22, [specs/racer-names.md](../specs/racer-names.md)):
  `0x00600730` (called first for every racer added) returns early with a name when the play mode `+0x1964` of the race
  status is 2 (career), the current race (`+0x1968`) is a boss race (`BossRace`, `0xFF5EE5D6`, read by `0x005FBD90`) and a
  current bin object exists (`+0x196C`, set by `0x00602BB0` from the profile's stage byte): it formats
  `BLACKLIST_RIVAL_%02d_LEADERBOARD` (`0x008A3BD0`) with the bin's `BinIndex` (`0x6CE23062`, `0x005DCEA0`), hashes it and
  stores the language text pointer (`0x0056BC10`) at `+0x08` of the racer record. When it declines, `0x00601F90`
  reads the driver's `RacerName` (`0xBEAB64C5`, `0x005DD2A0`; a non-empty text is hashed with `bStringHash`) in career
  mode and stores that text; only when both decline does it call the random draw `0x005DC800`.

## Data read for the fixed names **[verified]**

- `GLOBAL/gameplay.bin` (read with a throwaway script over the `VPAK` layout of [formats/attributes.md](../formats/attributes.md),
  and with the repository reader through `nfsmw-data/tests/real_install/racer_names.rs`): 5 of the 314 characters set
  `RacerName` (the prologue's `razor`, `opponent` of 16.2.1, `ronnie_mccrae`, `toru_sato` and `opponent` of 16.1.1); the 37
  boss races with an opponent set `BossRace` and none of the Blacklist bosses sets `RacerName`; the prologue races do
  not set `BossRace`. The original 2005 `gameplay.bak` holds the same five labels.
- `GLOBAL/GlobalB.lzc`: the `PresetRides` chunk (82 records) has `BL2` to `BL15`, `E3_DEMO_BMW`, `RAZORMUSTANG`,
  `DDAYSUPRA` and the `OPM_*` presets the prologue uses; there is no `BL1`.
- `LANGUAGES/English.bin`: `BLACKLIST_RIVAL_01..16_LEADERBOARD` hold the aliases in capitals.

## The list **[verified]**

`RACERNAME_000` to `RACERNAME_149` are the 150 labels `0x2D3F4EDC` to `0x2D3F53AA` (the hashes are consecutive because the
labels differ in their last digits only) of the English table: `BUTLER`, `BLADES`, `VANCE`, `LOLLEY`, ... `ALICE`, `JOEY`,
`K-9`. `RACERNAME_150` does not exist. The standings screen of a quick race shows three of them (`SLICK`, `ROBERTSON`,
`FERSEN`) beside the player's alias.

## The rewrite

A career race first fixes the names its data fixes (a boss race's rival, a character's `RacerName`). The pool is the table's names (read up to the first missing label) plus the names mods bring, each once. A name is picked from
a position the clock chooses and the next free one is taken, so a name is never given twice to one race and never equals the
player's. Mods add names in `mod.toml`:

```toml
[racers]
names = ["MAVERICK", "DEE DEE"]
```
