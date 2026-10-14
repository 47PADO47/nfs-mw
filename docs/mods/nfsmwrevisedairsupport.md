# NFSMWRevisedAirSupport

A community ASI plugin for Need for Speed: Most Wanted (2005, PC) that restores the police helicopter's unused radio speech and fixes parts of the police radio. It is a runtime patcher: it hooks game functions inside the running process and adds new pursuit logic on top of the original speech system.

Evidence tags used below: [source] verified by reading the code, [README] stated only in the README, [inferred] our reading, not stated anywhere. Note that the source files were read through a summarising fetch, not a full verbatim download, so exact figures should be re-checked against a clone before any port.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/eatincrispies/NFSMWRevisedAirSupport |
| Author | eatincrispies (copyright line in LICENSE: "Copyright (c) 2026 eatincrispies") |
| Type | ASI plugin (DLL with `.asi` extension), C++ |
| Licence | MIT (LICENSE file in repo root, "MIT License") [source] |
| Language | C++17, MSVC toolset v145, dynamic library, ConformanceMode on [source: vcxproj] |
| Target game build | NFS:MW PC v1.3 [README] |
| Loader | Ultimate ASI Loader [README] |
| Latest visible activity | Commit on 2026-10-11 (message "SoundAI"); page shows 22 commits [source: GitHub API commits list] |
| Repo size | Small: one DLL project, two source files plus one header in the speech subsystem |

## What it does

Each feature is a restored or changed line of police/helicopter radio behaviour. All are listed in the README [README], and the config keys that gate them are in the source [source].

1. Helicopter ram lines and intent-to-ram calls. Config `[Pilot] RamLines`. [README][source]
2. Search patterns: the pilot calls a search pattern when the player is lost or the chase cools down. Config `SearchPattern`. [README][source]
3. "Spotted you again" lines when the helicopter reacquires the player. Config `SpottedAgain`. [README][source]
4. A vehicle description (car colour, type, speed) on first sighting. Config `VehicleReport`. [README][source]
5. Escaped-car reporting: the spotter asks for the car that got away. Config `EscapedCar`. [README][source]
6. Reacquire lines after the helicopter loses visual. Config `Reacquire`. [README][source]
7. Lines when the player's car is hit. Config `BeenHit`. [source]
8. Kill-shot and dead-helicopter silence lines. Config `KillShot`, `DeadHeliSilence`. [source]
9. Roadblock call: the pilot calls for a roadblock ahead. Config `RoadblockCall`. [README][source]
10. Weather commentary when it rains. Config `WeatherReport` (pilot) and `RainFlag` (radio). [README][source]
11. Answering dispatch: the pilot answers 911 or backup calls, reports nearby units or says it is on its way. Config `Answer911`. [README][source]
12. Hold-on call when the player has stopped for a short time. Config `HoldOn`. [source]
13. Attack-missed line when a ram attempt misses. Config `AttackMissed`. [source]
14. Air-support requests: the player is asked for a fast air-support call (10-67 and spike strips in the README). Config `FastAirRequest`, `FastAirReminder`. [README][source]
15. Pursuit-music reminder at the 10-minute mark naming the player's current vehicle. [README]
16. Dispatch reads the suspect's history at heat 5 and above (README). Heat threshold in code is `kLowestHeliHeat = 4` for the pilot to take a 911 request. [README][source] (The README says heat 5+; the code constant is 4. Unresolved, see section 6.)
17. Helicopter fuel-out and damage bailout lines. [README][source: `kHeliLowFuelTime = 8.0`, health check `>= 1.0`]
18. Fixes: stops stray "low fuel" and "tunnels" lines after the helicopter is shot down; stops the cover-unit line when the helicopter is destroyed; keeps the speech system working through long pursuits. [README]
19. "No speech dropoff" fix: stops the speech system from silently dropping events. Config `NoSpeechDropoff`. [source]

## How it does it

Overall pattern [source]: a DLL with a `DllMain` that, on attach, disables thread library calls, starts a thread that sleeps one second and then calls `SoundAI::Init`. On detach it calls `SoundAI::Restore` only when the unload is a `FreeLibrary` (not process exit). Config is read at init.

### Loader
- Loaded by Ultimate ASI Loader from the game's `scripts` folder [README]. Ultimate ASI Loader is not in the repo, so no loader code is present [source: tree listing].

### Memory and patch layer [source]
- A small "Memory" namespace: SEH-guarded reads, a pattern check (reads up to 64 bytes and compares), and a write that sets page protection to read/write/execute, copies, flushes the instruction cache, and restores protection.
- A "Patch" namespace with these primitives, each recorded so it can be undone in reverse order (max 32 patches):
  - `Replace`: verify expected bytes, then overwrite.
  - `ReplaceVirtual`: overwrite one 4-byte vtable slot.
  - `RedirectCall`: rewrite a 5-byte `E8` near call to a new target.
  - `RedirectVirtualCall`: write a call plus NOP padding over a 5 to 16 byte sequence.
  - `Detour`: allocate a trampoline (copied prologue plus a jump back), then overwrite the function entry with a jump.
- A "Game" namespace that calls original game functions by address with `__thiscall` or `__cdecl`, and a `Global<T>` helper that reinterprets a fixed address.

Each patch first checks byte patterns (for example a function prologue) before writing, so a wrong game build should fail the check rather than corrupt memory [source].

### Game functions and globals (addresses from source, valid only for the one game build the code targets) [source]
- Speech and pursuit functions include `SoundAI::SyncPursuit` (0x00720850), `SoundAI::TerminatePursuit` (0x0071F6E0), `SoundAI::ResetPursuit` (0x0071CB10), `SoundAI::AddNewHeli` (0x0070DA60), `Manager::IndirectSpeechEvent` (0x006FEF90), `Manager::PostValidate` (0x0070C600), `Manager::CalcProbPlayback` (0x00704570), `EAXAirSupport::Update` (0x00709F50), `EAXCop::IntentToRam` (0x00718A30), `EAXCop::VehicleReport` (0x00718150), `EAXDispatch::BackupReply` (0x00717090).
- Globals include the `SoundAI` instance (0x00993CC8), `Manager::mGlobalHistory` (0x00992718), `Precipitation` (0x009196B8), `gHeliVehicle` (0x0090D61C), `WorldTimer` (0x00925AE8).
- The full list (about 40 functions, 10 globals, vtables and slots, struct offsets) is in the source headers. It is too long to copy here; re-derive it from a clone if needed.

### Hooks and patches applied [source]
- Vtable slot replacements on the helicopter class (`EAXAirSupport`) for intent-to-ram, set-handle, set-line-of-sight, update and bailout. Vtable slots on `EAXCop` for backup call and backup reminder.
- Detours at function entries: skid-hit pursuit (AI action), reset pursuit, collision handler, indirect speech event, post-validate, backup reply.
- Call-site redirects (5-byte `E8` calls) at specific game call sites: sync pursuit, reinitiate pursuit, add new helicopter, roadblock setup, speech dropoff, 911 answer, rain check.
- Byte-pattern checks guard each patch (for example `FF 92 E8 00 00 00` at the reinitiate site; `8B 44 24 04 83 EC 08 48` at terminate pursuit).

### Speech events [source]
- The plugin adds calls by event name, interface ID, CRC, handle, function and scheduling offset. Nine events are declared, including `AnytimeEvents_IntentToRam`, `AnytimeEvents_RegainVisual`, `AnytimeEvents_LostSuspect`, `AnytimeEvents_WeatherReport`, `Setup_VehicleReport`, `Setup_SpotterWanted`, `HeliSpecific_HeliBailout`, `StaticRoadblock_CallForRB`, `Interrupts_InterruptRamHigh`.
- Speech event IDs are an enum (for example `VehicleReport` = 155, `IntentToRam` = 113, `WeatherReport` = 219) [source].
- Scheduling uses the game's own speech manager; the plugin adds events to the queue and sets the playback probability to 1.0 when forced [source].

### Pursuit state [source]
- The plugin keeps its own state beside the game's: a list of recently escaped cars (up to 8), fuel tracking for the helicopter, a rain flag, the current search intensity (starts High), and per-event history.
- Helicopter fuel time is read from a fixed offset (0x7D8) and compared with the 8-second threshold [source].
- Rain intensity is read from a fixed offset (0x28C), threshold 0.25 [source].

### Configuration [source]
- A TOML-like file named after the module: `NFSMWRevisedAirSupport.toml`, located next to the module (the code builds the name by replacing the extension). Parsed line by line: `#` comments, UTF-8 BOM skipped, only `true` and `false` accepted.
- Every key defaults to `true` [source].

## Configuration

Config file: `NFSMWRevisedAirSupport.toml` (name inferred from code; the repo tree has no `.toml` file, so the file is not shipped) [source: tree listing].

`[Pilot]` section, all default `true`:
`RamLines`, `SearchPattern`, `SpottedAgain`, `VehicleReport`, `EscapedCar`, `Reacquire`, `BeenHit`, `KillShot`, `DeadHeliSilence`, `RoadblockCall`, `WeatherReport`, `Answer911`, `HoldOn`, `AttackMissed`.

`[Radio]` section, all default `true`:
`RainFlag`, `FastAirRequest`, `FastAirReminder`, `NoSpeechDropoff`.

Install per README: copy the `scripts` folder into the game directory and launch [README]. To uninstall, delete `NFSMWRevisedAirSupport.asi` from `scripts` [README]. The README does not mention the `.toml` file.

## How we could implement it in nfs-mw

The rewrite does not run the original binary, so the DLL patch approach does not apply. Almost everything here is a clean-room design problem. Licence: MIT, so the code could be read, but per our rule we describe behaviour and facts only and write our own code from the spec.

### Does the mechanism apply?
- Not applicable: ASI loader, DLL entry, thread sleep, memory patching, byte-pattern checks, vtable and call-site redirects, Detour trampolines. These exist only because the plugin patches a closed binary. Our rewrite has its own speech and AI code, so there is nothing to hook.
- Not applicable: all hard-coded addresses and struct offsets. These are game-build facts about v1.3 and must not appear in `libs/`.
- Applicable in concept: the behaviour list (section 2), the probability and threshold rules (section 4 config defaults), and the state the plugin tracks.

### Behaviours to specify (spec-first)
1. Pursuit-driven helicopter events: ram intent, miss, search pattern, reacquire, lost visual, bailout on fuel or damage, weather report on rain.
2. A dispatch layer: 911 requests answered with a probability (the source uses 1/3 when heat is at or above a minimum), backup calls, reminders, and "on the way" replies.
3. Escaped-car memory of up to 8 cars, with the spotter asking for them.
4. Radio rules: fast-air request and reminder, roadblock call, and a "no dropoff" rule so queued speech is not silently discarded.
5. A forced-playback rule for the speech queue, so the helicopter lines are not suppressed by normal probability.
6. Config: each line group can be turned off by a boolean.

### Data and formats needed
- Speech sample bank: we need our own sample list, its event IDs, and a mapping from event name to our speech bank. Do not copy the game's IDs or the plugin's event table; derive from our own extraction (the existing `game-install` and format docs are the right place to start).
- Pursuit state inputs: heat level, player speed, line of sight to the suspect, helicopter fuel and health, rain intensity, time since the player stopped, escaped-car list. Each needs a documented source in our simulation.
- A config file format: we can use the same boolean keys with a clear spec, or TOML if we adopt it for the project. Decide in a spec, not by copying this file.

### Natural home
- Engine-generic pieces (a speech queue with priority, probability gating, event scheduling, a bounded recent-events memory, a TOML/boolean config reader) belong in `libs/`. No NFS:MW names or addresses in `libs/`.
- NFS:MW-specific pieces (the helicopter pursuit logic, the cop and dispatch lines, the escaped-car rule tied to our pursuit model, the heat threshold of 4 or 5) belong in `crates/`. `crates/` depends on `libs/`, never the reverse.
- Format parsing for the speech bank belongs in a format crate under `libs/` that takes `&[u8]`, per the project rules; only `game-install` touches the install path.

### Risks
- Behaviour mismatch: the plugin's exact timing and probability values come from one game build; they are a starting point, not a spec. Verify against the original game's data before adopting any number.
- Heat threshold conflict: README says heat 5 and above for suspect history; the code constant is 4. Resolve in our own spec.
- Licence hygiene: MIT permits reuse, but our rule is spec-first. Do not paste code or addresses into our tree; record facts in a spec with a link to this page.
- Scope: this is a content and AI-flavour feature, low priority compared with physics and format work. Note it for the roadmap, do not start it early.

## Open questions and what could not be verified

- The README says a `.toml` config is read, but the repo has no `.toml` file. Whether the default file ships in the release is not visible here.
- Release binaries and the `.asi` file were not downloaded (per instructions), so the built behaviour is unverified; only source reading was done.
- Source was read through a summarising fetch, not a full raw download, so the address and offset tables may contain transcription errors. Re-check against a clone before any port.
- Latest commit date is 2026-10-11 and the repo has 22 commits; the repo is small and early (possibly one author). No issues, release list or CI config was read.
- Whether the game-version target (v1.3) is the only one supported is unknown: the code uses fixed addresses, so only one build is likely supported [inferred].
- No tests found in the tree [source: tree listing].
- The README credits testers Roskler, greedymanual and MrYudhaP14 [README]. Not verified.

## Sources

- https://github.com/eatincrispies/NFSMWRevisedAirSupport (repo page, summarised)
- https://raw.githubusercontent.com/eatincrispies/NFSMWRevisedAirSupport/HEAD/README.md
- https://raw.githubusercontent.com/eatincrispies/NFSMWRevisedAirSupport/HEAD/LICENSE
- https://api.github.com/repos/eatincrispies/NFSMWRevisedAirSupport/git/trees/HEAD?recursive=1
- https://api.github.com/repos/eatincrispies/NFSMWRevisedAirSupport/commits?per_page=3
- https://raw.githubusercontent.com/eatincrispies/NFSMWRevisedAirSupport/HEAD/NFSMWRevisedAirSupport/dllmain.cpp
- https://raw.githubusercontent.com/eatincrispies/NFSMWRevisedAirSupport/HEAD/NFSMWRevisedAirSupport/src/NFSMW/SoundAI.cpp
- https://raw.githubusercontent.com/eatincrispies/NFSMWRevisedAirSupport/HEAD/NFSMWRevisedAirSupport/src/NFSMW/SoundAI.hpp
- https://raw.githubusercontent.com/eatincrispies/NFSMWRevisedAirSupport/HEAD/NFSMWRevisedAirSupport/NFSMWRevisedAirSupport.vcxproj
