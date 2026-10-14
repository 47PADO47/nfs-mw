# NFSMWFairBusts

A small ASI plugin for Need for Speed: Most Wanted (2005, PC) that removes the instant bust applied when the player resets the car during a police pursuit.

Source: https://github.com/eatincrispies/NFSMWFairBusts

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/eatincrispies/NFSMWFairBusts |
| Author | eatincrispies (copyright line in LICENSE: "Copyright (c) 2026 eatincrispies") |
| Type | .asi plugin (game mod, C++ DLL loaded by an ASI loader) |
| Licence | MIT (LICENSE file and GitHub API report MIT License) [source] |
| Language | C++ (GitHub API; build via Visual Studio or MinGW-w64 g++ with C++20) |
| Target game build | Most Wanted (2005) PC v1.3, `speed.exe` 6,029,312 bytes [README] |
| Stars / forks | 2 / 0 (GitHub API) |
| Created | 2026-09-06 (GitHub API) |
| Last push | 2026-09-19 (GitHub API). Last repo update 2026-09-27 (GitHub API). |

## Summary

The mod targets the stock behaviour where resetting the car during a pursuit fills the busted meter about four times faster and ignores the usual escape rules. With default settings the reset costs the player only momentum and the pursuit continues under normal rules. The mod works by patching machine code in memory at startup, located by a byte pattern rather than fixed addresses. Its settings live in an INI file next to the plugin.

## What it does

- Removes the instant bust on reset. Default `ResetAutoBust = 0` applies normal pursuit range and removes the speed gate that caused the instant bust. [source: `src/dllmain.cpp`, Initialize step 5; `NFSMWFairBusts.ini`]
- Restores stock behaviour on request with `ResetAutoBust = 1`. [source: `dllmain.cpp` step 5 is skipped; `NFSMWFairBusts.ini`]
- Adjusts the post-reset busted meter fill rate with `ResetBustMultiplier`. Stock is 4.0; the default in the INI is 1.0. [source: `dllmain.cpp` step 6, constant `kResetRate = 4.0f`; `NFSMWFairBusts.ini`]
- Can be disabled without uninstalling via `Enabled = 0`. [source: `dllmain.cpp` Initialize step 1; `NFSMWFairBusts.ini`]
- Refuses to patch an unknown build: if the byte pattern is not found exactly once, it logs and does nothing. [source: `dllmain.cpp` FindUnique and Initialize step 3] (README says it targets v1.3 only.)
- Compatibility claimed with Unlimiter, Bartender, ExtraOptions, Takedown N2O, and the Widescreen Fix, and with patched executables (large address aware, widescreen, no-CD). [README]

## How it does it

All of the following is in `src/dllmain.cpp` (one file, no comments in the source) unless noted.

- **Loader.** Packaged as `NFSMWFairBusts.asi`, loaded by an ASI loader (`dinput8.dll` from Ultimate ASI Loader is named in the README). [README]
- **Entry point.** `DllMain` on attach calls `DisableThreadLibraryCalls`, stores the module handle and starts `InitThread` on a new thread; on detach it calls `Shutdown`, which reverts all patches. [source]
- **Crash guard.** `InitThread` runs `Initialize` inside SEH and calls `ReportCrash` on exception. [source]
- **Locating code.** No fixed addresses. It scans the executable sections of the host module (after validating DOS and NT headers) for a byte pattern named `kResetPenaltyBlock`, which begins `83 F8 01 75 02 8A D8 84 DB` and contains wildcards (`??`) for address operands. The match must be unique. [source]
- **Offsets within the match.** Fixed offsets from the pattern start: normal-range load at +0x09, reset-range load at +0x13, speed-gate skip at +0x19 (two bytes), reset-rate scale at +0x54; operand at +0x02 within each instruction. [source]
- **Constants.** Normal pursuit range 15.0f, reset range 90.0f, reset rate 4.0f. [source]
- **Patches.** Three optional patches: `resetRange` (repoint the reset-range operand at the normal-range float), `speedGate` (overwrite two bytes with NOPs `90 90`), and `resetRate` (repoint the rate operand at a global `g_resetRate` float when the multiplier differs from 4.0). [source]
- **Patch write.** `VirtualProtect` to `PAGE_EXECUTE_READWRITE`, `memcpy`, restore protection, `FlushInstructionCache`. A `ScopedPatch` saves up to 8 original bytes and restores them on destruction. [source]
- **Safe memory reads.** `IsReadable` walks `VirtualQuery` regions; `SafeCopy` uses MSVC SEH around `memcpy` (non-MSVC checks readability first). [source]
- **Consistency check.** Before patching, it checks that operands at the load and scale sites point to the expected float values; if not, it logs that another plugin has changed the code and stops. [source]
- **Identification.** `HostMd5` computes the MD5 of the host executable with CryptoAPI and writes it to the log. It is logged, not used as a gate. [source]
- **Logging.** `WriteLog` overwrites `NFSMWFairBusts.log` in the module directory with a single status line. [source]
- **Config reading.** `GetPrivateProfileStringA`, section `[General]`. [source]

No DirectX hooks, no function hooks or detours, and no game-function calls were seen in the source. The mod is a byte patcher.

## Configuration

File: `NFSMWFairBusts.ini`, section `[General]`. Changes apply on next launch. [README, source]

| Key | Default | Meaning |
|---|---|---|
| `Enabled` | `1` (source default true) | `0` leaves the game unchanged. |
| `ResetAutoBust` | `0` (source default false) | `0` removes the instant bust on reset (normal rules). `1` keeps stock behaviour. |
| `ResetBustMultiplier` | `1.0` (source default 1.0) | Post-reset meter fill rate. Clamped to 0 to 64, parsed with `strtof`. Stock game is 4.0. |

Boolean values accept `1/0`, `true/false`, `yes/no`, `on/off` (case-insensitive). [source]

Note: the README summary says the multiplier and `ResetAutoBust` interact only as described above; the source applies the multiplier patch regardless of `ResetAutoBust`. [source, inferred for the interaction]

## How we could implement it in nfs-mw

Licence: the repo is MIT. The rule for this project still applies: this page records behaviour and facts only. No code from the repo is to be copied or ported. Specify from the behaviours below and implement from our own spec.

Important: this mod is a loader-specific, in-process patcher of the original executable. A clean-room Rust rewrite does not need it. Our rewrite reads the install data and implements the physics and pursuit rules itself, so there is no `speed.exe` to patch and no ASI loader to run under. The only parts that carry over are the design facts about the behaviour.

Behaviours to specify (as rules for our own pursuit model, not patch logic):

- After a player reset during pursuit, the busted meter does not jump. Pursuit range and escape rules stay normal.
- Post-reset busted meter fill rate is a tunable factor; stock is 4.0, default for the fair variant is 1.0.
- A settings switch restores the stock reset behaviour for comparison.
- A global enable switch.
- Clamp ranges on the tunable factor (0 to 64 in this repo; to be decided by our spec).

Data or formats needed:

- The pursuit/heat state model and the meter fill rule, which must come from our own spec-first research notes (see `docs/` physics and AI specs), not from this repo.
- A small config format for the switches. The repo uses INI; if we keep INI, the parser is generic and belongs in `libs/`.

Natural home:

- Pursuit reset and meter rules: `crates/` (NFS:MW-specific) on top of an engine-generic simulation in `libs/`.
- Config parsing for generic key/value files: `libs/`.
- Constants such as the 15 and 90 range values, and the 4.0 stock multiplier, live in NFS:MW-specific data in `crates/`, not in `libs/`, per the rule against NFS:MW names or defaults in `libs/`.
- Crates depend on libs, never the reverse.

Risks:

- Using this repo as a reference for values would blur the clean-room boundary; any value we use must be verified against the game data or our own spec.
- The stock 4.0 multiplier and the 15/90 range constants come from this repo's code and have not been verified against the game data by us.
- The repo's summaries disagree about the meaning of `ResetAutoBust = 0`; the source and INI comments agree that `0` removes the instant bust. We follow the source reading here.

## Open questions and what could not be verified

- The pattern bytes are only partly known (the first nine bytes in the source). The full pattern and wildcard layout were not read, so no absolute addresses are given. The repo does not state the exact game function that holds the reset penalty code; that is [inferred] from the pattern name `kResetPenaltyBlock`.
- The fetched dllmain.cpp was summarised by the fetch tool, not read verbatim (the tool refused verbatim output). The offsets, constants and function list above come from that summary and should be treated as [source] only as far as the summary is accurate. Reading the file directly would confirm them.
- The README says the mod targets v1.3 only; the source checks only that the pattern is unique, not the build version or the MD5 value.
- No release binaries were checked; they were not downloaded, per the task rules.
- Last commit date and message were not visible through the directory page. The API gives push and update dates only.
- Not verified: whether the stated compatibility with other mods has been tested by the author beyond the README claim.

## Sources

- https://github.com/eatincrispies/NFSMWFairBusts (repo page, via WebFetch)
- https://raw.githubusercontent.com/eatincrispies/NFSMWFairBusts/main/README.md (via WebFetch)
- https://raw.githubusercontent.com/eatincrispies/NFSMWFairBusts/main/NFSMWFairBusts.ini (via WebFetch)
- https://github.com/eatincrispies/NFSMWFairBusts/tree/main/src (via WebFetch)
- https://raw.githubusercontent.com/eatincrispies/NFSMWFairBusts/main/LICENSE (via WebFetch)
- https://raw.githubusercontent.com/eatincrispies/NFSMWFairBusts/main/src/dllmain.cpp (via WebFetch, summarised)
- https://api.github.com/repos/eatincrispies/NFSMWFairBusts (via WebFetch)
