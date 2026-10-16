# Provenance: mod world content (scenery packs and interiors)

Record for [specs/mod-world-content.md](../specs/mod-world-content.md) (task B14, 2026-10-16). Program: `/speed.exe`
(the Black Edition v1.3 install, image base `0x00400000`, about 26,000 functions). Ghidra auto-names appear here only;
the spec uses addresses. Evidence tags as in [docs/README.md](../README.md#evidence-tags).

Sources read for the design (behaviour only, no code copied): `docs/formats/maps.md`, `world.md`, `gameplay-races.md`,
`install-layout.md`, `docs/specs/scenery-visibility.md`, `scenery-lod.md`, `visible-sections.md`, `world-markers.md`,
`docs/provenance/world-entry-zones.md`, `docs/modding/api*.md`, ADR 0005. Rust sources read: `libs/blackbox-streaming`,
`libs/blackbox-scenery`, `libs/blackbox-solid`, `libs/blackbox-collision` (`world.rs`, `grid.rs`), `libs/game-install`
(`vfs.rs`), `crates/nfsmw-data/src/world/` and `gameplay/markers.rs`, `crates/nfsmw/src/scenes/world/` (`residency.rs`,
`resident.rs`, `zone.rs`, `entry_zones.rs`, `entry.rs`, `ground.rs`). No cargo was run.

## How the addresses were found

| Address | Role (behaviour) | Found by | Tag |
|---|---|---|---|
| `0x00666aa0` | Region loader: order of the global, metadata and stream loads | string `TRACKS\%s.BUN` (`0x008a916c`) and its xref; the handler's debug name string `RegionLoader::LoadHandler` | [decomp] |
| `0x00659530` | Builds `TRACKS\STREAM%s.BUN`, starts the streamer, sets `BeginGameFlowLoadTrack` | string `TRACKS\STREAM%s.BUN` (`0x008a8cf4`), its xref | [decomp] |
| `0x0072c070` | Binds each index record to a per-number pool (pointer at `+0x58`); the `% 100` remap | callee of `0x00659530` | [decomp] |
| `0x0072bd60` | Chunk loader: `0x34110` index (92-byte records), `0x34111`, `0x34112`, `0x34113` | byte search `10 41 03 00` (hits `0x0072bd6b`, `0x0072bf07`) | [decomp] |
| `0x0072bf00` | Reset of the same chunk ids | the second hit above | [decomp] |
| `0x00727de0` | Sets the record `Status` to 4 after a section load (`+0x0C`) | callee of `0x0072bd60` | [decomp] |
| `0x00658dd0` | Section read: `Size` bytes at `FileOffset`, fix-up, dispatch | callers `0x00658f40`, `0x00727de0` | [decomp] |
| `0x00658a00` | Dispatcher: 64-chunk windows, last to first; loader table at `0x008f7ef8`, fallback `0x008f7e40` | callee of `0x00658dd0` | [decomp] |
| `0x0072b130` | `EmTriggerPack` loader (`0x80036000`; children `0x36001`, `0x36002`, `0x36003`) | byte search `00 60 03 80` (hit `0x0072b136`) | [decomp] |
| `0x0072b220` | `EmTriggerPack` unload | byte search (hit `0x0072b226`) | [decomp] |
| `0x007307e0` | Candidate `ScenerySection` (`0x80034100`) parser | byte search `00 41 03 80` (hit `0x0073080d`) | [decomp, not read] |
| `0x00731e70`, `0x00729a80` | Candidate `VisibleSectionManager` (`0x80034150`) parsers | byte search `50 41 03 80` (hits `0x00731e7b`, `0x00729a87`) | [decomp, not read] |
| `0x0073124b` | A hit for `0x80034100` inside no defined function | byte search | [unresolved] |
| `0x00724360` | Section number to key (the `% 100` remap uses its result) | callee of `0x0072c070` | [decomp, not read] |
| `0x00516e40` | Returns the file name of a platform scene by index (five names at `0x0089b4d8`–`0x0089b580`) | xrefs of the platform strings | [decomp] |
| `0x0061e750` | Table that references `JumpToSafeHouse`, `JumpToCarLot`, `DoSafeHouseIntro`, `AllowEngageSafehouse` | xrefs of those strings (`0x008a4b18`, `0x008a4b34`, `0x008a4cc8`, `0x008a4a5c`) | [xref] |
| `0x00751860` | Car part name builder (`%s_SKIN1`, `%s_INTERIOR`, `%s_DRIVER`, `%s_LICENSE_PLATE`, ...) with an 87-entry part cap (`0x57`) | xref of `%s_INTERIOR` (`0x008b4e8c`) | [decomp] |
| `0x0058c640` | Zone enter handler (opens front-end screens) | from `world-entry-zones.md` (earlier pass) | [exe] |
| `0x0056e100` | Zone type to hash | from `world-entry-zones.md` | [exe] |
| `0x00625d20` | Reads the `ZoneType` attribute name (`0x008a7784`) | xref | [xref, not read] |
| `0x009020f4`, `0x00902100` | Data references to `TUNNEL` (`0x008b29d4`) and `STREAMER_PREDICTION` (`0x008b29a4`) | xrefs (data only) | [xref] |
| `0x008b4e7c`, `0x008b4e8c` | `%s_INTERIOR_N`, `%s_INTERIOR` strings | xrefs to `0x00751860` | [decomp] |
| `0x0064b770` | Uses `TrackStreamerLoadingBar` (`0x008a89a0`) | xref | [xref, not read] |
| `0x008f7ef8`, `0x009a3b50`, `0x009aa490`, `0x009aa4dc`, `0x00903164` | Dispatch table, trigger pool, ready and pending queues of trigger packs, the remap constant | read as globals in the decompile | [decomp] |

## Read-only check of the install

- Method: a read-only parse of `D:\Need For Speed Most Wanted Black Edition\TRACKS\L2RA.BUN` (no write, no game launch).
  The first `10 41 03 00` (chunk id `0x00034110`) gives the index at file offset `0x38DD0`, payload 66,240 bytes, which is
  720 records of 92 bytes.
- Result [verified]: all 720 numbers unique; minimum 126 (`A26`), maximum 2600 (`Z0`). Letters present `A B C D E F G H I J
  L M O P Q R S T V X Y Z`; no `K`, `N`, `U` or `W`. The name and number agree for every record (`A41` is 141, and so on).
  The first record is `X0` at number 2400 with offset 0 and size 18,707,408; the last is `T99` (number 2099).
- Use: the free bands in spec section 1.3. The check covers one install only; see the open question on the other installs.

## Lessons

- A byte search for a chunk id finds more than loaders. Each hit has to be placed inside a function (the function's
  start and size). Two hits inside one loader are the id test and the payload read; a hit inside no function is
  an unanalysed site (`0x0073124b`). Create the function at the start address before decompiling it.
- A string such as `%s_INTERIOR` in a front-end or model codebase can name a part, not a place. Read the function that
  builds the name before naming a feature from it. Here the function built all car parts (`0x00751860`).
- The order in which a dispatcher runs chunk loaders is a fact to record. Here the windows run last to first, so a loader
  cannot assume an earlier chunk. Check the order with the decompile of the dispatcher, not from the file order.
- Big functions (4 KB) are costly to decompile. Ask for `signature,callers,callees` first, then decompile only the
  functions the question needs.
- A constant read from a global (`0x00903164`) is a lead until its value is read. The remap test uses it. Reading its value
  at run time (a read-only debug build or memory dump) is the next step; it is not guessed.
- Read the installed data directly for claims about sets and ranges (letters, numbers). A short read-only parse settled the
  section-number question in one call, where prose from the format doc only implied it.
- Data references (a string used in a table) have no code xref. Find the table's base address before reading its consumer.

## Dead ends

- `%s_INTERIOR` is not a building interior. It is the car cockpit part (`0x00751860`). Do not search for building interiors
  through it.
- `0x00751860` is 4 KB of car part logic. It holds nothing about entry zones or streaming.
- `TUNNEL` (`0x008b29d4`) has data references only, from the table at `0x009020f4`. No function reads the table by code
  xref. The tunnel type is known from the format docs (track zones, type 3). The reader was not found.
- `AMEnterSafeHouse` (`0x008a0537`) has no xref. It may be used by a name table, not by code.
- The safe-house entry icon search from the earlier pass is not repeated here (see `world-entry-zones.md`).
- Searching for a section loader by its data word alone (for example the number `0x1974ff09`, used by the region
  loader as a state key) gives state tests, not loads. Not followed further.

## Could not verify

- The remap constant at `0x00903164` (presumed 40, the `LODOffset` of the format doc).
- That the trigger pool key at payload `+0x0C` is a section number.
- What the trigger records (`0x36003`) drive: scripted events, entry zones, or both.
- Whether the record pointer list at streamer `+0x6C4` / `+0x6C8` has any writer at run time.
- Whether `U` and `W` have no sections in the other installs (`D:\NFSTools\NFS Most Wanted Redux 2.3` and the Desktop copy).
  The Black Edition install was the only one parsed.
- Whether the streamer caps its queue. No cap was found in the index loader; the queue code was not read.

## Next leads (copied from the spec, section 8)

1. `0x00724360` and the value at `0x00903164`.
2. The writer of streamer `+0x6C4`.
3. The scenery and visible-section parsers at `0x007307e0`, `0x00731e70`, `0x00729a80`, and the hit at `0x0073124b`.
4. `0x0064a9c0` (second read) and `0x00465a70` (buffer step) in the section read.
5. The trigger tree (`0x36002`) and records (`0x36003`), and the consumer of the pool at `0x009a3b50`.
6. `0x0061e750` (the script command table) and the platform loads.
7. The `TUNNEL` table at `0x009020f4`: find its base and its reader.
8. The overlay reader for `VisibleSectionOverlays` (`0x00034158`).
9. The queue limits of the streamer.
