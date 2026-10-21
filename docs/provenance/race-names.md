# Racer names (`crates/nfsmw/src/scenes/world/race/names.rs`)

- **Specs:** [ai-racers](../specs/ai-racers.md) (names), [modding content](../modding/api-content.md#racer-names).
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
- Whether a career rival is renamed after the add (the Blacklist names) was not read **[unconfirmed]**. The rewrite names
  every computer-driven racer from the list.

## The list **[verified]**

`RACERNAME_000` to `RACERNAME_149` are the 150 labels `0x2D3F4EDC` to `0x2D3F53AA` (the hashes are consecutive because the
labels differ in their last digits only) of the English table: `BUTLER`, `BLADES`, `VANCE`, `LOLLEY`, ... `ALICE`, `JOEY`,
`K-9`. `RACERNAME_150` does not exist. The standings screen of a quick race shows three of them (`SLICK`, `ROBERTSON`,
`FERSEN`) beside the player's alias.

## The rewrite

The pool is the table's names (read up to the first missing label) plus the names mods bring, each once. A name is picked from
a position the clock chooses and the next free one is taken, so a name is never given twice to one race and never equals the
player's. Mods add names in `mod.toml`:

```toml
[racers]
names = ["MAVERICK", "DEE DEE"]
```
