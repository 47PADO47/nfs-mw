# Menu navigation and selection sounds: findings

Status: **partially implemented (2026-10-23)**. Menu messages now reach authored bank playback;
see [the implementation spec](specs/frontend-audio.md) and
[provenance](provenance/frontend-audio.md) for the verified mapping and remaining gaps.
The historical investigation below describes the state before that change.

The Rust front end did not play menu navigation or selection sounds. The original
does. This note records what was established about the original's sound path in `speed.exe` (the
NFS: Most Wanted PC build), what was ruled out, and where to continue. No code change was made for
this; the front end still drops the sound messages.

Addresses are `speed.exe` image addresses (image base `0x00400000`). Function names are Ghidra's
`FUN_` names, not symbols.

## The symptom

Moving the cursor in a front-end menu and pressing accept or back is silent in the port. The original
plays a short sound for each.

## What the Rust port does

- `libs/blackbox-feng/src/runtime/messages.rs` routes a script's post to sound (response id 3,
  target `0xFFFFFFFB`) into `Outgoing::Sound { message, package, from }`.
- `libs/blackbox-feng/src/runtime/input.rs` also sends directional and accept/back pad messages to the
  sound target (`docs/specs/feng-input.md`, section on accept and directions).
- `crates/nfsmw/src/frontend/screens.rs:160` receives every outgoing message and drops sound ones:
  `Outgoing::Sound { .. } => {}`.
- No code in the port plays a front-end cue. `crates/nfsmw/src/audio/common_effects.rs` covers only the
  speedbreaker and visual pulse events, through the in-game `SFX_Common` class.

So the missing piece is the mapping from a front-end sound message to an actual sound, and the call that
plays it.

## What the original's sound path looks like

Established by reading `speed.exe` in Ghidra (MCP server, `speed.exe` loaded) and the install files.

**Front-end sound banks.** `SOUND/FE/FE_MB.abk` (3.1 MB) and `SOUND/GLOBAL/FE_COMMON_MB.abk` hold the
front-end sounds (`docs/formats/audio.md`, folder map). `FE_MB` is registered as `AEMS_FEBanks`
(`docs/specs/engine-sound.md`, section 2).

**Event descriptor.** `SOUND/EVT_SYS/FE_AEMS.csi` is an `MOIR` file. Its readable strings are
`FEDriveOn`, `PlayFrontEndSample`, `PlayCommonSample`, `PlayFrontEndSample_RS`. There is one event,
`FEDriveOn`, and the rest are script functions it calls.

**Script functions.** `FUN_004AC3F0` registers four named functions with the sound system through
`FUN_0083198B`: `PlayCommonSample` (`PTR_s_PlayCommonSample_008f02b8`), `PlayFrontEndSample`
(`PTR_s_PlayFrontEndSample_008f02c0`), `PlayFrontEndSample_RS` (`008f02c8`) and `FEDriveOn`
(`008f02d0`). `FUN_004D02E0` calls `FUN_004AC3F0` during sound setup.

**Sample player.** `FUN_004B9470` (a virtual method; its vtable is `PTR_FUN_00896F80`, class constructor
`FUN_004B92C0`) takes one `undefined4*` argument. Its branch test is `param + 0x17`. Values `0x67` to
`0x76` (except `0x6E` and `0x6F`) take the `PlayFrontEndSample` path with a 0x14-byte voice. Other values
take the `_RS` path, with duration capped at 500 ms (`0x1F4`).

**Sound queue dispatcher.** `FUN_004D02E0` runs every frame. It walks a linked queue at `param_1+0xBC`.
For each record it matches the pair of fields `[4]` and `[5]` against a table at `DAT_00912860` (0x68-byte
stride), then calls a virtual method on the matched object (`(**(code**)(*local_8 + 0x1C))()`) and
passes the record to `FUN_004C91D0`.

**Cue table.** `DAT_00912860` is empty in the file and filled at run time. `FUN_004CA7C0` appends one
entry; its key pair comes from the record's `[2]` and `[3]`. The only caller is `FUN_004CE960`, the
general "post a cue" path. `FUN_004CE960` has three callers: `FUN_004CEFB0`, `FUN_004CF010` and
`FUN_004E52C0`.

**Startup cues.** `FUN_004CF010` registers the siren and effect cues with `FUN_004CEFB0`. The cue
names are name hashes such as `0xE48DF448` (indices 0 to 7), `0xEB1FD1BF`, `0x916AA05D`, `0x0663AAF3`,
`0x9849F8DF` and `0x7E6BA5C8`. Its caller is `FUN_004D163A`, the sound setup.

**Car and world cues.** `FUN_004E52C0` posts vehicle and world cues (for example gearbox and
impact events in `FUN_004E8620`, `FUN_004E8770`, `FUN_004E8810`, `FUN_004E8880`, `FUN_004E8930`,
`FUN_004E8A40`). These are not the front end.

## Hashes

The engine's name hash is `h = h * 33 + byte`, seeded with `0xFFFFFFFF` (`libs/blackbox-hash`). With it:

| Name | Hash |
|---|---|
| `PAD_UP` | `0x72619778` |
| `PAD_DOWN` | `0x911C0A4B` |
| `PAD_LEFT` | `0x9120409E` |
| `PAD_RIGHT` | `0xB5971BF1` |
| `PAD_ACCEPT` | `0x406415E3` |
| `PAD_BACK` | `0x911AB364` |
| `PAD_START` | `0xB5AF2461` |
| `FE_Navigate`, `FE_Select`, `FE_Back` | `0xD9340F78`, `0xDD940C49`, `0xC7FBADFA` |

`PAD_ACCEPT` matches `docs/specs/feng-input.md`, so the hash function is right for pad messages.

The attribute hash in `libs/blackbox-attrib` is a different function (`vlt_hash`, Jenkins lookup2). The
attribute files were searched with both kinds of hash.

## Negative results

- **Attribute database.** `GLOBAL/attributes.bin` and `GLOBAL/FE_ATTRIB.bin` were decoded with
  `blackbox-attrib`. The probe checked every `UInt32`, `Key`, `Text` and `Raw` value for the pad hashes
  and the cue hashes above. No match was found. The attribute values are mostly `Float`, `Bool`, `Int32`
  and `Array` values. The `Raw` values are reference records.
- **Raw files.** `SOUND/EVT_SYS/FE_AEMS.csi`, `SOUND/EVT_SYS/MAIN_AEMS.csi`, `SOUND/FE/FE_MB.abk`,
  `SOUND/GLOBAL/FE_COMMON_MB.abk` and `GLOBAL/FE_ATTRIB.bin` were searched for the same hashes
  (little-endian `u32`). No match was found. `FEDriveOn` is stored as text in the `.csi`, not as a hash.
- **Runtime breakpoints.** Hardware execute breakpoints were set in Cheat Engine on the cue post
  (`0x004CE960`), the cue registry (`0x004CA7C0`) and the sample player (`0x004B9470`). The menu was
  navigated: a move, then an accept, then back. The cue post and the registry logged nothing. The sample
  player logged only hits from an earlier run. The queue dispatcher (`0x004D02E0`) runs every frame and
  was removed to reduce noise.
- **Byte pattern search.** A search for the immediate `0xFFFFFFFB` (the sound target) found many
  unrelated hits. The ones checked are not the front-end path.

## Conclusion so far

The menu sound path does not go through `FUN_004CE960` or `FUN_004B9470` during ordinary menu
navigation. The front-end pad messages reach the sound system somewhere else, and the cue they produce
is not one of the hashed cues found by the static searches.

## Where to continue

1. Find the front-end pad input handler. `docs/specs/feng-input.md` describes it: accept and
   directions are sent to the sound target (`0xFFFFFFFB`). In Ghidra, the handler is the function that
   calls the FEng message-queue post with a pad message id. Start from the pad message strings
   (`FEPad_Up` at `0x0089AD5C`) and their xrefs.
2. Set a Cheat Engine breakpoint on that handler, then read the sound target and message id it sends.
   That gives the real mapping without guessing.
3. Once the mapping is known, decide between two fixes: route `Outgoing::Sound` into a front-end playback
   path that loads `FE/FE_MB.abk` and plays the mapped index, or keep the cue path and register the
   cues.

## Tools and method

- Ghidra MCP: `get_xrefs_to` (group `xref`), `force_decompile`, `read_memory`, `search_strings`,
  `search_byte_patterns` (group `analysis`).
- Cheat Engine MCP: `open_process` (by PID or name), `set_breakpoint` (hardware debug registers, four
  slots), `get_breakpoint_hits`, `remove_breakpoint`, `read_integer`.
- Probe tests were temporary and removed. The repo tree was left clean.
