# Provenance: the busted sequence

Addresses, files read, method lessons and dead ends for [busted-sequence.md](../specs/busted-sequence.md). The spec
holds the rules; this file says where each one came from. Ghidra auto names (`FUN_*`, `LAB_*`) are used here only.
Image base `0x00400000`; program `/speed.exe`; `D:\Need For Speed Most Wanted Black Edition\speed.exe` (6,029,312 bytes).

## Addresses in `speed.exe`

### Arrest path

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Cutscene-channel listener, the arrest handler | `FUN_0044dc70` (created by `create_function`, 427 bytes) | `0x0044dc70` | calls the chooser, the spot finder, `NISActivity`, `Infractions.fng` |
| Scene chooser | `FUN_0044d770` | `0x0044d770` | 547 bytes; one caller (`0x0044dcc0`, inside the handler) |
| Arrest spot finder | `FUN_0044da10` | `0x0044da10` | 600 bytes; caller the handler |
| Listener constructor (arrest) | `FUN_0044f160` | `0x0044f160` | vtable `0x008933cc`; caller `FUN_00450570` at `0x00450570` |
| Subscribe helper | `FUN_0044c380` | `0x0044c380` | args: listener, callback `0x0044dc70`, `0`, subject `0xfea34c0a`, `0` |
| Scene-set into NIS manager | `FUN_0046e710` | `0x0046e710` | 59 bytes; global `0x008ee320` (the camera-pack owner in `animation.md`) |
| String copy helper | `FUN_00460ca0` | `0x00460ca0` | 40 bytes, 75 callers; (buffer, name, size) |
| Scene table init (names, flag bits) | `FUN_004dfe30` | `0x004dfe30` | 2349 bytes; caller `FUN_004e0790` (`0x004e0790`, no callers) |
| NIS activity constructor | `FUN_00640f00` | `0x00640f00` | 1506 bytes; callers `FUN_00641520`, `FUN_00641590`, `FUN_00643d50` |
| NIS listener | `FUN_006f4ea0` | `0x006f4ea0` | 644 bytes; string `NISListenerActivity` at `0x00893b50` used at `0x006f5098`; no callers |
| Race-bust request | `FUN_00623a90` | `0x00623a90` | 39 bytes; vtable `0x008a7110`; params (0, 1, 0); consumer not found |
| Front-end screen request | `FUN_0062fe40` | `0x0062fe40` | 119 bytes; vtable `0x008a723c`; mode 2 with `Infractions.fng` from the handler |
| Random helper (variant) | `FUN_0045da70` | `0x0045da70` | float helper, called with `0.99` (chooser); 32 callers incl. rain code |
| Rounding helper | `FUN_007c4b80` | `0x007c4b80` | converts the x87 result to an integer |
| Distance helper | `FUN_005cb5d0` | `0x005cb5d0` | used in the spot finder |
| Spot query | `FUN_007862d0` | `0x007862d0` | called in the spot finder with a query object `FUN_0076e2d0` / `FUN_007658a0` / `FUN_0077b9e0` setup; not decoded |
| Perp handle | globals `DAT_0092cd84`, `DAT_0092cd7c` | | null check, then the perp object |
| Career/race object | global `DAT_0091e000` (`GRaceStatus` per the script registry) | | `+0x1968` null in free roam (name ours) |
| Free-roam flag | global `DAT_009b9e6c` | | set to 1 by the handler when `+0x1968` is null |
| Handler gate | global `DAT_009885c8` | | the handler does nothing when non-zero (meaning not known) |

### Message dispatch

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| `MPerpBusted` type getter | `FUN_004080d0` | `0x004080d0` | 116 bytes; first call sets the cache `DAT_0090d960` (flag `DAT_0090d964`) from `FUN_005cc240("MPerpBusted")` |
| Name hash | `FUN_005cc240` | `0x005cc240` | used for `MPerpBusted`, `NISActivity`, `car1`, `AIRacerBusted`, `Speech`; the hash function is not identified |
| Broadcast registration table | `FUN_00648590` | `0x00648590` | 2075 bytes; about 65 `FUN_006xxxxx` registrations, all on subject `0x20d60dbf` |
| Registration helper A | `FUN_00635d20` | `0x00635d20` | 124 bytes; (callback, subject, data); type from the getter |
| Registration helper B | `FUN_00706570` | `0x00706570` | 140 bytes; same shape; caller `FUN_007200c0` |
| Registration helper C | `FUN_004eb210` | `0x004eb210` | 132 bytes; caller `FUN_004f8830` |
| Queue forwarder | `FUN_00642910` | `0x00642910` | 115 bytes; forwards the message into the event queue `DAT_0091e0d0` through `FUN_0061fc10` |
| Speech constructor | `FUN_007200c0` | `0x007200c0` | 1788 bytes; strings `Speech`, `Comment`, `Infraction`, `SoundAI`, `TireBlo`; registers callback `0x00700f70` for the getter's type on `0x20d60dbf` |
| Constructor (registers `Pathfinder5`, `EATraxInit`, `Event`, `Control`, ...) | `FUN_004f8830` | `0x004f8830` | 641 bytes; registers `0x004dfc20` on `0x20d60dbf` |
| Message dispatch | `FUN_00665390` | `0x00665390` | called by all the registration helpers; not decoded (posts or subscribes) |
| Cutscene subject | | `0xfea34c0a` | arrest listener only |
| Broadcast subject | | `0x20d60dbf` | all other listeners found |

### Pursuit overlay and screens

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| In-game overlay router | `FUN_004c2340` | `0x004c2340` | 5029 bytes; 39 names (hash plus string); callers `FUN_00572360` (203 callers), `FUN_00586290` (76), `FUN_0060dbd0` (`StartRace`) |
| Script function registry | `FUN_0061e750` | `0x0061e750` | 2466 bytes; registers `GRaceStatus` on `DAT_0091e000`, `PreventPlayerBeingBusted` at `0x0060ae00`, `ShowWinningPostRaceScreen`, `SetHasRapSheet`, `AwardPlayerBounty`, `Play`, `PlayHackE3FMV` |
| Infractions builder (cars) | `FUN_00546a50` | `0x00546a50` | 1296 bytes; factory `FUN_005710e0` (`0x005710e0`, 94 bytes, no callers) |
| Infractions per-car fill | `FUN_0052d640` | `0x0052d640` | 678 bytes; caller `FUN_00546a50` |
| Infraction-type rows | `FUN_005466d0` | `0x005466d0` | 675 bytes; factory `FUN_00571080` (`0x00571080`, no callers); eight rows (`FUN_005938a0(this, param, 1, 8, 0)`) |
| Recent pursuits (rap sheet rows) | `FUN_0052e0c0` | `0x0052e0c0` | 847 bytes; caller `FUN_00546fc0` (`0x00546fc0`, 96 bytes) via factory `FUN_005711a0` |
| Speech-related in pursuit | `FUN_00700f70` | `0x00700f70` | callback named as the speech listener; not read |

### Strings (addresses in `speed.exe`)

| String | Address | Refs (code or data) |
|---|---|---|
| `MPerpBusted` | `0x00890d98` | `FUN_004080d0` at `0x004080fa` |
| `AIRacerBusted` | `0x00892fc7` (with a leading `>`) | `FUN_007200c0` (the hash goes to a stack temp; the temp is not passed to the registration) |
| `BustedMomentMarker` | `0x00893200` | none |
| `eNIS_Busted` | `0x008a3e20` | none |
| `Infractions.fng` | `0x00893ab8` | data refs `0x0044ddc0` (the handler), `0x006f76d7`, `0x008f46e8` |
| `NISActivity` | `0x00893ad0` | `FUN_00640f00` (`0x006412db`), `0x0044dd0b`, `0x006fd1f0`, `0x008891b0` |
| `NISListenerActivity` | `0x00893b50` | `FUN_006f4ea0` (`0x006f5098`), `0x008848f0` |
| `ArrestM01` .. `ArrestF23b` (the chooser's names) | `0x00893a7c` to `0x00893aac` (the packed names); `ArrestM23` at `0x008988f0`, `ArrestM23b` at `0x008988e4` | table in `FUN_004dfe30`; refs in `FUN_0044d770` |
| `BUSTED_OVERLAY.fng` | `0x008948f8` | `FUN_004c2340` at `0x004c2489`, `0x004c24a8`, `0x004c24cc`; and data `0x008f46d8` |
| `PostBusted.fng` | `0x0089f7dc` | data `0x008f46e0` only |
| `PostRace_Pursuit.fng` | `0x00897210` | `FUN_004c2340` (3 refs), `0x00563e54`, `0x006421b7`, `0x008f46f8` |
| `InGameBounty.fng` | `0x008971d4` | `FUN_004c2340`, `0x0055a717`, `0x008f44b0` |
| `BUSTED_%d` | `0x0089d464` | `FUN_00546a50` (`0x00546e39`), `FUN_0052d640` (`0x0052d895`) |
| `FINES_%d` | `0x0089d494` | `FUN_00546a50`, `FUN_0052d640` |
| `BOUNTY_%d` | `0x0089d4a0` | `FUN_00546a50`, `FUN_0052e0c0`, `FUN_0052d640` |
| `INFRACTION_TYPE_%d` | `0x0089e610` | `FUN_005466d0` |
| `INFRACTION_GROUP_%d` | `0x008a2648` | not read |
| `Hud_BustedMeter` | `0x008a2878` | `Unwind@008738ee` (as before) |
| `PreventPlayerBeingBusted` | `0x008a4994` | `FUN_0061e750` (`0x0061f0d1`) |
| `RapSheetPD.fng` | `0x0089d510` | not traced |
| `NISMemory` | `0x008938a0` | `FUN_0044cc90` |
| `NisCameras` | `0x008950b4` | `Unwind@0086a4a0` only |

### Hashes checked

| Value | Meaning | Check |
|---|---|---|
| `0x0c407210` | `BUTTON_PRESSED` (FEng message) | bstring hash of the upper-cased name (`libs/blackbox-hash`, same as `fe_hash_upper`): matches. Used by `InGameBusted.fng` and `Infractions.fng` |
| `0xfea34c0a` | cutscene subject | not the bstring or lookup2 hash of `MPerpBusted` (raw, upper, initval `0xABCDEF00`, initval 0). Not identified |
| `0x20d60dbf` | broadcast subject | not identified |
| `0x35e4e01f` | `< VEHICLE IMPOUNDED >` (label) | `nfsmw strings 0x35e4e01f` |
| `0xd3efe2e5` | `< SUSPECTED TO DRIVE >` (label) | `nfsmw strings`; code computes this value as the default |
| `0x2089554c` | `< CONFIRMED TO DRIVE >` (label) | `0xd3efe2e5 + 0x4c997267` (mod 2^32); the code adds `0x4c997267` when a ushort at car record `+0x16` is non-zero |
| FEng names (`InGameBusted.fng`, `Infractions.fng`, `BUSTED_OVERLAY.fng`, `PostBusted.fng`, `PostRace_Pursuit.fng`) | | bstring of the name and of the upper-cased name, and of the stem: no hit in `speed.exe` (see dead ends) |

## Files read

### On `D:` (read-only)

- `D:\Need For Speed Most Wanted Black Edition\speed.exe`: strings, xrefs, function decompiles (Ghidra), and its bytes
  for the hash search.
- `FRONTEND/`, `GLOBAL/`, `NIS/`, `MOVIES/` folder listings (names and sizes).
- `NIS/Scene_ArrestF02_BundleB.bun`: `tools/chunkdump.py` (tree, `-s` for the scene header and entity records).
- Screen dumps through the repo's `target/release/nfsmw.exe` (an existing build, not built again):
  `dump-screen` for `Infractions.fng`, `BUSTED_OVERLAY.fng`, `InGameBusted.fng`, `PostRace_Pursuit.fng`,
  `RapSheetPD.fng` (and `PostBusted.fng`, which gives "no screen"); `list-screens` (197 packages from 6 files);
  `strings` for the labels and the bust texts.
- The install was the Black Edition only. The Desktop copy and the Redux copy (see `docs/install-layout.md`) were not
  checked.
- No game was launched and no window was opened.

### In the repo (read)

- `AGENTS.md` (in context), `docs/reversing.md` (method and tags), `docs/specs/ai-pursuit-heat.md` (all),
  `docs/specs/ai-pursuit.md` (sections 1, 4.4, 5, 6 and the constants), `docs/provenance/ai-pursuit-busted.md` (all),
  `docs/provenance/ai-pursuit-escape.md` (the Ghidra pass), `docs/specs/camera-views.md` (the arrest camera section),
  `docs/specs/loading-screen.md` (the top; script ids), `docs/specs/feng-runtime.md` and `docs/specs/frontend-menus.md`
  and `docs/specs/eagl-animation.md` and `docs/formats/video.md` (headings only), `docs/formats/animation.md` (the
  arrest and ICE sections), `docs/formats/frontend.md` (headings and the package list), `docs/models.md` (grep only).
- Code, for what is loadable: `libs/blackbox-hash/src/lib.rs` (bstring hash), `libs/blackbox-feng/src/hash.rs`
  (`fe_hash_upper`), `libs/blackbox-attrib/src/hash.rs` (lookup2 and `vlt_hash`), `libs/blackbox-movie` (directory
  listing only). `libs/blackbox-feng` runtime and `libs/blackbox-movie` decoder were not read.
- Not re-read in this pass: `CONTRIBUTING.md`, `docs/licensing.md`. The clean-room rule was followed: specs hold
  behaviour in prose; no decompiled code is in the repo.

### Scratch (not in the repo)

- `scratchpad/busted/hashchk.py`, `lk2.py` (lookup2), `hsearch.py` (bytes search for name hashes in `speed.exe`),
  `guess.py` (candidate FEng message names), plus the screen dumps and the chunk tree.

## Lessons

- **A string xref into a getter is not the message's sender.** `MPerpBusted` is referenced only by `FUN_004080d0`.
  That getter caches the name's hash, so the five functions that call it are subscriptions of that one type, not five
  senders. The earlier note ("the sender is not located") still holds for the sender.
- **Count and read the registrations, not just the callers.** `FUN_00648590` registers about 65 types on one broadcast
  subject. Reading its callees gave the listener set faster than following callers.
- **A label with no function can still be code.** `0x0044dc70` had no function. `create_function` at the entry, then
  `get_functions` with `decompiled_code`, worked. The earlier `disassemble_function` at `0x0044dc80` failed because
  that address was inside the new function, not its entry.
- **Name the globals from the script registry.** `FUN_0061e750` registers script names with their functions and
  takes the object as a global (`DAT_0091e000`, `GRaceStatus`). Those names help read the callers.
- **Last write wins with a copy helper.** The chooser writes the scene name into one buffer several times. The
  helper's arguments (buffer, name, size) say it copies, so only the last write is the final scene. This is an
  assumption about the helper; check the helper if the copy semantics matter.
- **FEng message names use the bstring hash of the upper-cased name.** `BUTTON_PRESSED` gives `0x0c407210`, which is the
  message on every button press in both bust screens. This is a quick way to name messages in `dump-screen` output.
- **Labels are checkable.** `nfsmw strings 0xHASH` returns the text for a label. The status labels of the impound list
  (`0x35e4e01f`, `0xd3efe2e5`, `0x2089554c`) were named this way, and the code's computed default matched a known label.
- **Screen names in `speed.exe` are plain strings.** A search for the name hashes of the screen files (bstring and
  upper-cased, stem and full name) found nothing, for names that are plainly used (`Infractions.fng`,
  `BUSTED_OVERLAY.fng`). So the absence of a name string is the test for a screen that the code never names.
- **A mask router can skip an entry without an error.** `FUN_004c2340` has the overlay name but no action for it.
  Read every branch that the entry's index can reach before saying a screen is shown or hidden there.
- **`chunkdump.py` needs its own folder on the path.** Run it as `python tools/chunkdump.py` (not with `-I`, which
  drops the script's folder and fails on `bchunk_names`).
- **Search strings with distinct queries.** The tool hook warned about a repeated `search_strings`. Use one
  alternation pattern per question.

## Dead ends (don't repeat)

- **`PostBusted.fng`.** The string is at `0x0089f7dc` and has one data reference (`0x008f46e0`) and no code reference.
  The install has no such package (`dump-screen` says "no screen", `list-screens` has no match). The Black Edition
  does not ship it. Do not look for it in other install folders.
- **`eNIS_Busted`.** Named at `0x008a3e20`; no references at all. Probably a debug enum name in the source. Not a
  scene or a bundle.
- **`BustedMomentMarker`.** Named at `0x00893200`; no references (same as the earlier pass). The arrest spot is found
  by the spot query, not by this name.
- **`InGameBusted.fng`.** On disk (`GLOBAL/InGameB.bun`, 3 objects), but not named in `speed.exe` (no string, no name
  hash match). Its use is unknown; do not treat its position as proof of a shown screen.
- **Name hash of `MPerpBusted` vs the subject `0xfea34c0a`.** Neither the bstring hash nor lookup2 (initval `0xABCDEF00`
  or 0), raw or upper-cased, gives `0xfea34c0a`. The cutscene subject is a different key. `FUN_005cc240` produces the
  type hash in the binary; its function was not identified.
- **Overlay show path via `FUN_004c2340`.** The router has `BUSTED_OVERLAY.fng` at index 7, but no branch acts on that
  index (the mask path tests index 10 and above, and the switch has `break` for 4 to 7 and 13). Do not expect the
  router to show it.
- **Busted FMV.** None in `MOVIES/` (32 files) or in `NIS/` `Scene_FMV*` names. The arrest is an NIS.
- **Camera in the arrest bundle.** `Scene_ArrestF02_BundleB.bun` has no camera chunk (only texture, geometry,
  skeleton, animation, `CarpEventSequence` and one `AnimScene` with three entities).
- **Arrest scenes `ArrestM23` / `ArrestM23b`.** Named in the table but no bundle exists and the chooser never picks
  them.
- **Random helper `0x0045da70`.** Its return value and the rule are not decoded (32 callers, including rain code).
  The variant rule stays unconfirmed.
- **Race-bust request `0x00623a90`.** Its constructor is read, but the consumer that gives the pause was not found.
- **The text "TOTAL COST 80,000" and "Cash 60,000" on `Infractions.fng`.** Placeholders in the package. Not a fine total.
- **`NISListenerActivity` (`0x006f4ea0`).** Read as a name only: no callers as a function, so it is reached through a
  table. Its end path was not read.

## Open leads (carried to the spec's Next leads)

- Sender of `MPerpBusted`, the 3 s delay, the post-NIS screen, the spot query, the variant rule, the request consumers,
  the camera track of the arrest, the per-car record fields (`VehicleDB.cpp`, not read), `InGameBusted.fng`'s user.
- The function containing the `Infractions.fng` reference at `0x006f76d7` (not yet decompiled; outside the NIS listener).

## Ghidra notes for the next pass

- Program `/speed.exe` only. Pass `program` on every call.
- `get_functions` with `fields=callers,signature` is the cheap way to see a function's shape before decompiling it.
- `get_xrefs_to` takes many addresses at once (`addresses=`); use it for a set of strings.
- `read_memory` and `disassemble_function` need an entry address; `create_function` first if the label is inside a
  bigger region.
