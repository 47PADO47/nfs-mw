# Parts database capacity: how many mod cars the game can carry

Spec for `libs/blackbox-carparts` (`parts/push.rs`, `parts/mod.rs`) and `crates/nfsmw-data/src/car/mods/parts.rs`. Part of the
[specs](README.md). It is the author's own analysis of the repository's reader and of the stored layout in
[cardata.md](../formats/cardata.md); nothing here comes from the game binary beyond that layout. The mod-facing summary is in
[api-content.md](../modding/api-content.md#parts-database-capacity).

## 1. What is stored and what is in memory

The parts database of the install (`CarPartPack` in `GLOBAL/GlobalB.lzc`) is a set of packed tables. The records are read once
into the structures of `PartsDb` and the repository never writes the pack back: a mod car's parts are added to the decoded
copy in memory ([`PartsDb::push_part`]). The 14-byte stored part record has fields too narrow for hundreds of mod cars:

| Field of the stored record | Width | What it indexes | Stock install | Stored limit |
|---|---|---|---|---|
| type index (offset 0x7) | 8 bits | the type-name table: one entry per car type and shared group (`PAINT`, `WHEELS`, ...) | 106 | 256 |
| name offset (0x8) | 16 bits, units of 4 bytes | the string table | 26,480 bytes | 262,140 bytes |
| model table (0xC) | 16 bits | the model tables (the solid hashes of a part by level of detail) | 543 | 65,535 |
| attribute list (0xA) | 16 bits, units of 2 bytes | the attribute lists | 15,460 bytes | 131,070 bytes |
| attribute index in a list | signed 16 bits | the attribute pairs | 1,870 | 32,767 |

(The "stock install" column is `PartsDb::sizes()` on the v1.3 install's `GlobalB.lzc`: 13,080 parts.)

## 2. What a mod car costs

A mod car adds one type name (its folder, so that `find` can tell its parts from every other car's) and these parts, each with
one model table, one name of the form `<FOLDER>_<KIND>` and no attributes (`car/mods/parts.rs`):

| Car | Parts | Model tables | Strings |
|---|---|---|---|
| a car | body, front wheel, driver | 3 | 3 names |
| a bike | the three above, rear wheel, fork, bars | 6 | 6 names |

Measured on the stock database with 410 mod cars of six parts each: 516 type names, 3,003 model tables, 80,600 bytes of strings
(about 132 bytes per bike, names of at most 27 + 12 characters padded to 4 bytes), 15,540 parts.

## 3. Which limit binds first

With the stored widths the **type-name table** is the first to fill: 256 - 106 = 150 mod cars. A car after the 150th gets
`Error::Full("type-name")`, its parts are not added, and the game logs that the car has no parts (it then has no body part and
is not drawn). The other tables are far from full at that point (strings 26,480 of 262,140 bytes; model tables 543 of 65,535),
and would allow about 1,700 bikes (strings: 235,000 bytes / 132) before the next limit.

The need: the 2012 mod's 93 cars and the 317 bikes of RIDE 6 are 410 mod cars, 2.7 times the limit.

## 4. The fix: wide in-memory fields

The stored widths are properties of the file, not of the engine's decoded copy. `Part` now keeps, in memory:

| Field | Was | Is |
|---|---|---|
| `type_index` | `u8` | `u16` (the stored byte is widened when the pack is read) |
| `name_offset` | `u16` | `u32` |
| `model_table` | `u16` | `u32` |

and `push_part` checks the wide limits (`u16` type names: 65,535; `u32` strings and model tables) instead of the stored ones. The
attribute tables keep their stored widths (mod cars add no attributes; the attribute lists, shared pairs and `i16` indices are
the same as before and `Error::Full("attribute"|"attribute list")` still applies).

Properties kept (all covered by the tests of `libs/blackbox-carparts/src/tests/push.rs`):

- the stock parts keep their order and values, and a search that takes the first match still finds them first;
- a type name already in the table is reused; a part group goes in whole or not at all (`push_parts` rewinds on a failure);
- reading a stock pack gives the same parts as before (the widened fields hold the same numbers).

Why not the alternatives: registering a car's parts when it is selected (lazily) would add a second life cycle (a car must be
registered before anything asks for it: the garage, the shop, traffic, AI, the network catalogue all look parts up by type) and
reusing one part per (kind, slot) with the solid name looked up per car would change what a part is (the model names come from
the part's own table). Both hide a limit that is only a width, at the price of code on every lookup. A wide index costs 2 bytes
per part.

## 5. Tests

- `blackbox-carparts`: 600 new car types; a string table and model tables beyond the 16-bit range still resolve; the attribute
  limits are still refused; a group that does not fit leaves the database as it was.
- `nfsmw-data` (`car/mods/parts.rs`): 410 mod cars through `extend`, each with its body and wheel parts findable.
- `nfsmw-data` real install (`tests/real_install/carparts.rs`, ignored without a game): 410 cars of six parts pushed onto the real
  `GlobalB.lzc` database; the stock parts unchanged, the 150th and the 410th car found.

## 6. Not covered

Other per-car tables of the game outside the parts database have not been checked at 410 cars: the garage lists, the shop's
catalogues, the career's car slots and the network catalogue are tables of their own, with their own limits, and a game with
this many mod cars has not been run.
