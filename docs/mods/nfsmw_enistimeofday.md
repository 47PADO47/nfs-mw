# NFSMW_ENISTimeOfDay (xan1242)

## Summary

NFSMW_ENISTimeOfDay is a small C++ DLL fix for Need for Speed: Most Wanted (2005, PC). The author states that the game's `ENISTimeOfDay` event handler was broken on the PC version, so cutscenes (NISes) that depend on it showed the wrong time of day. The fix replaces the PC handler with a callback that behaves like the Xbox 360 version. The fix is hard-wired to the v1.3 US executable. The repository is a single-purpose patch, not a framework or loader.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/xan1242/NFSMW_ENISTimeOfDay |
| Author | Lovro "xan1242" Pleše (copyright holder in LICENSE.txt) [source] |
| Type | .dll patch (C++ DLL, Visual Studio project). Intended as a plugin for the game. Whether it ships as an `.asi` is [inferred], not stated. |
| Licence | MIT License, "Copyright (c) 2025 Lovro (xan1242) Pleše" [source: LICENSE.txt] |
| Language | C++ (`dllmain.cpp`, `.vcxproj`, `.sln`) [source for file names; the GitHub language bar was not captured] |
| Target game build | Need for Speed: Most Wanted, v1.3 US executable only [README, dllmain.cpp header comment] |
| Latest visible activity | Not captured. The repo page shows 3 commits and no releases or tags. Last commit date and message: unknown. The LICENSE copyright year is 2025. |
| Dependencies | `includes/injector/` (header-only helpers) [source: folder listing] |

## What it does

Each feature below is a user-visible effect, with its evidence.

1. **Correct time of day in the ENISTimeOfDay NIS event.** The PC handler is replaced so the event sets the lighting time of day, update rate, sun azimuth and overcast value as the Xbox 360 version does. [README; dllmain.cpp]
2. **Correct time of day in specific NISes.** The README names the Blacklist intro NISes and the D-Day intro as affected by the bug. [README]
3. **Overcast value reset is incomplete.** The source comment says not all NISes reset the overcast value to -1.0, a "known bug" that matches the 360 behaviour. The overcast can stay set until the game is restarted after some Blacklist intro NISes. [dllmain.cpp comment]

No other features are present in the source. There is no UI, no config, and no extra gameplay change.

## How it does it

Claims are tagged: [source] = seen in code, [README] = stated only in README, [inferred] = my reasoning.

- **Load-time hook.** `DllMain` calls `Init()` on `DLL_PROCESS_ATTACH`. [source]
- **Callback table overwrite.** `Init()` sets `idx_ENISTimeOfDay = 61` and writes the address of `ENISTimeOfDay_MakeEventCallback` into `gMakeEventCallbacks[61]`. This is a single function-pointer table write, not an inline patch. [source]
- **Hard-coded addresses (v1.3 only).**
  - `gMakeEventCallbacks = 0x008F5F40`: the game's MakeEvent callback table. [source]
  - `ppTimeOfDayLighting = 0x9B392C`: pointer to the live time-of-day lighting object. [source]
  - `p_twkTimeOfDayOvercast = 0x905CE8`: address of the overcast float the callback writes. [source]
  - The header comment says "the fixed addresses are for v1.3 ONLY!". [source]
- **Lighting structure.** `eTimeOfDayLighting` has the fields `UpdateRate` (float), `UpdateDirection` (int), `CurrentTimeOfDay` (float), `SunAzimuth` (float) and `Latitude` (float). `Get()` dereferences `ppTimeOfDayLighting`. `UpdateDirection` and `Latitude` are declared but never used. [source]
- **Callback body.** `ENISTimeOfDay_MakeEventCallback(float* staticdata)` reads the event's static data array:
  - `staticdata[1]` into `CurrentTimeOfDay`
  - `staticdata[2]` into `UpdateRate`
  - `staticdata[3]` into `SunAzimuth`
  - `staticdata[4]` written to the overcast float at `p_twkTimeOfDayOvercast` [source]
- **Compiler setting.** The callback is wrapped in `#pragma runtime_checks("", off)` and then restored. [source]
- **Injector helpers.** `includes/injector/` contains `assembly.hpp`, `calling.hpp`, `hooking.hpp`, `injector.hpp` and `utility.hpp`. `injector.hpp` is included by `dllmain.cpp`, but the code shown does not call any of its functions. The helpers are a generic memory-patching library, probably derived from the Silent/ThirteenAG style injector family. [source for file names; the origin is [inferred]]
- **No byte patterns, no INI reading, no DirectX hooks.** The source has no byte-pattern scans and no config file access. [source]

What is not shown: how the DLL is loaded into the game process (ASI loader or other). The README does not say. [inferred: the design is consistent with an ASI-loaded DLL, but that is unverified.]

## Configuration

None. The source reads no config file, environment variable or command-line option. The only "settings" are the hard-coded addresses and the v1.3 US executable requirement. [source]

## How we could implement it in nfs-mw

Rules applied: the repo is MIT licensed, so its code may be read but the approach should be specified from behaviour and facts first (spec-first, per `docs/licensing.md`). Do not port the source text. The feature is also tied to the game's runtime memory, which a clean-room Rust rewrite does not have.

**Does the feature apply to a clean-room rewrite?** Mostly no, as written. The repo is a binary-patch fix to the original executable: it writes into the game's memory at fixed addresses and replaces a function-pointer table entry. The nfs-mw project reads the install from data (see `docs/architecture.md`) and does not run the original game code. The underlying behaviour does apply: the rewrite should make the ENISTimeOfDay event produce the same lighting state as the 360 version.

**Behaviours to specify (spec-first):**
1. The ENISTimeOfDay event takes a static data array with four values (index 1 to 4 as above) and maps them to current time of day, update rate, sun azimuth and overcast.
2. Overcast handling, including the known case where it is not reset to -1.0 by some NISes. Decide whether to reproduce this quirk or fix it, and record the choice.
3. Which NISes call the event (Blacklist intro, D-Day intro). This needs to come from the game data, not from hard-coded addresses.
4. The lighting object's fields and their update semantics (`UpdateRate`, `CurrentTimeOfDay`, `SunAzimuth`).

**Data or formats needed:**
- The event's static data layout in the NFS:MW event/NIS data. Check the existing format docs in `docs/` and the gameplay reader (`crates/` or `libs/`) before adding anything. Do not assume the layout from this repo.
- The 360 behaviour as the reference. Confirm it from a legitimate source, not from this repo's source text.

**Natural home:**
- Event and lighting logic that is engine-generic (a time-of-day lighting state, a sun model) goes in `libs/`, with no NFS:MW names, paths or defaults.
- NFS:MW-specific event IDs, the NIS list and the version-specific data go in `crates/`. `crates/` depends on `libs/`, never the reverse.

**Risks:**
- Any address-based approach would be tied to one executable build (v1.3 US). A data-driven approach avoids that.
- The overcast quirk means state can leak between cutscenes. A rewrite must define the reset rules explicitly.
- Licence: this repo is MIT, so copying its code would require keeping the MIT notice. The spec-first rule still applies: do not copy the source text.

## Open questions and what could not be verified

- The loader model (ASI or other) is not stated in the README or the source shown. Not verified.
- The GitHub repo page did not show the language breakdown, last commit date or message. The commit count (3) comes from the repo page. Not verified beyond that.
- The full `dllmain.cpp` and the injector helper contents were not read in full. The WebFetch summary of `dllmain.cpp` was condensed by the fetch tool; quote-level details (for example the exact `staticdata` indexing) should be checked against the raw file before any spec is written.
- The `injector.hpp` helper library: origin and licence are not verified. Its licence must be checked before any reuse.
- The README at the repo root was only partly returned by the fetch tool. The statement that the fix works only with the v1.3 US exe is confirmed by two fetches. Installation steps, credits and loader requirements were not found.
- No test was run and nothing was executed, per the task rules.

## Sources

- https://github.com/xan1242/NFSMW_ENISTimeOfDay
- https://github.com/xan1242/NFSMW_ENISTimeOfDay/tree/master/includes/injector
- https://raw.githubusercontent.com/xan1242/NFSMW_ENISTimeOfDay/master/dllmain.cpp
- https://raw.githubusercontent.com/xan1242/NFSMW_ENISTimeOfDay/master/README.md
- https://raw.githubusercontent.com/xan1242/NFSMW_ENISTimeOfDay/master/LICENSE.txt
