# mwfixes (GrimMaple)

A small Windows DLL that patches the NFS:MW 1.3 executable at load time to fix a few crashes and a race-timer bug. Read here for behaviour and facts only; the code is GPL-3.0 and is not to be copied (see section 5).

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/GrimMaple/mwfixes (public, branch `master`) |
| Author | GrimMaple (GitHub account; no further identity stated) |
| Type | Native DLL patch (plain DLL that writes into the game's memory; not an `.asi` loader plugin in the source) [source] |
| Licence | GNU GPL v3 (`LICENSE` file, "Version 3, 29 June 2007") [source] |
| Language | C++ (Visual Studio solution `MWFixes.sln`); exact share not shown on the page |
| Target game build | NFS:MW 1.3 only. README: "Fixes only work with 1.3 version of the game." DllMain checks the entry point against `0x7C4040` [README] [source] |
| Latest visible activity | 14 Feb 2018, "Some commenting and release preps" (549e781). 8 commits in total, all Feb 2018 [source: commits page] |
| Releases | Releases section empty on the repo page; README points to GitHub Releases [README] |
| Description | None set on the repo |

## 1. Summary

mwfixes is a DLL that is copied into the game's install directory next to the executable. On load it reads an INI file, then writes machine code into the running game: it replaces a few instructions, redirects some calls and jumps to its own code, and runs a background thread that resets the race timer. Its goal is stability: it targets known null-pointer dereferences, a pure virtual call crash, and a "timebug" in the race timer. The project is small (one source file in `src/`, one header with the patch helpers, one vendored injector header).

## 2. What it does

Each feature is switchable in `MWFixes.ini` (section 4).

- **Timebug fix** (`TimebugFix`, default 0 in the INI). Resets the race timer value to 0.0 when it decreases, so a timer going backwards no longer carries over. README lists it as "Timebug". Evidence: `src/dllmain.cpp` `FixTimebug()` [source]; README [README].
- **Memory fitness check fix** (part of `StabilityFixes`). Forces the game's "use_best_fit" check to fail so that the game's memory-fitness path is skipped. README: "Memory fittnes checks" [README]; code: `FixMemory()` [source].
- **Null-pointer fix in `Attrib::StringToLowerCaseKey`** (part of `StabilityFixes`). Returns 0 when the input string pointer is null instead of dereferencing it. README [README]; code: `FixStringToLower()` [source].
- **Null-pointer fix near 0x0057D105** (part of `StabilityFixes`). Inserts a null check before a `mov edx, [ecx]` by jumping into a small code cave. README: "Null-pointer dereference @ 0057D105" [README]; code: `FixSub_0057D0F0()` [source].
- **Roadblock spawn fix, "pure virtual function call"** (`PreventPurecall`, default 1 in the INI). Intercepts one virtual call; when the vtable pointer matches the expected value at `0x00890970`, the call returns 0 instead of reaching a pure virtual slot. The INI warns this can stop some roadblocks spawning. README: "Roadblock spawn problem (pure virtual function call)" [README]; code: `FixPurecall()` [source].
- **Purecall trap and dump** (`AddPurecallHandler`, default 0). Replaces the game's `_purecall` handler with one that crashes on purpose so a dump file is produced. The INI says "Crashes anyway, but generates dumps". README: "Purecall handler trap to generate dump files" [README]; code: `AddPurecallHandler()` [source].

## 3. How it does it

Mechanism, as read in the source:

- **Loading.** `DllMain` runs on process attach. It compares the executable's entry point with `0x7C4040`. If it does not match, it shows an error box and stops. If it matches, it starts `Init()` and a background thread [source]. The loader that brings the DLL in is not part of the repo; the README says to copy the files into the install directory [README]. Whether this is an `.asi`-style loader or a DLL proxy is not stated [open question].
- **Config.** `ReadConfig()` reads `MWFixes.ini` through `IniReader.h`, section `[Fixes]`, four integer keys [source].
- **Init.** Calls `ReadConfig`, then `AddPurecallHandler` and `FixPurecall` if enabled, then `FixMemory`, `FixStringToLower` and `FixSub_0057D0F0` if `StabilityFixes` is set [source].
- **Background thread.** Loops: `FixTimebug()` then `Sleep(1)` [source].
- **Timer addresses.** `TimerAddress = 0x009142DC`, `GlobalTimerAddress = 0x009885D8` (globals inside the game image, read and written by `FixTimebug`) [source].
- **Patch primitives** (`include/CPatch.h`, class `CPatch`, all static) [source]:
  - `Patch`, `Patch2`: `VirtualProtect` to read-write-execute, `memcpy`, restore protection.
  - `Nop`, `FillWithZeroes`, typed setters (`SetInt`, `SetFloat`, `SetPointer`, etc.), `SetUIntWithCheck` (writes only if the current value matches an expected value).
  - `RedirectCall` writes an `E8` (CALL rel32) at an address; `RedirectJump` writes an `E9` (JMP rel32). Each overwrites 5 bytes with no trampoline, so the original code at that site is lost.
  - `AdjustPointer` rebases one DWORD that falls in a given range; patches at most one location per call.
- **Patch sites** (game image addresses, from the source) [source]:
  - `FixMemory`: `0x00464EE6`, `0x00464F47`. A `mov` is replaced by `xor eax, eax` plus NOPs.
  - `FixSub_0057D0F0`: `0x0057D105`, `0x0057D121`.
  - `FixPurecall`: `0x0043E005`, `0x0043DD15`, `0x0043DCF5`, `0x0043DCE5`. Vtable check against `0x00890970`.
  - `AddPurecallHandler`: `0x007C56B0`, `0x007C56B5`, `0x00663D35`.
  - `FixStringToLower`: `0x004549A1`, `0x004549A5`, `0x004549AC`, `0x004549BF`, `0x00454962`.
- **DirectX.** No DirectX code in the repo [source].
- **Exports.** None; `DllMain` is the only entry point [source].
- **Injector.** `include/injector/` is a vendored folder for memory patching (the source includes `injector/injector.hpp`). Its contents were not read in this pass [open question].

Tag key: [source] verified in the code read here; [README] only stated in the README; [inferred] not stated directly.

## 4. Configuration

File: `MWFixes/MWFixes.ini` (copied next to the game). Section `[Fixes]`, integer values, 1 = on, 0 = off [source].

| Key | Default in INI | Code fallback | Meaning |
|---|---|---|---|
| `TimebugFix` | 0 | 0 [source] | Race timer reset when it decreases |
| `StabilityFixes` | 1 | 0 [source] | Memory fitness, string-to-lower, and 0057D105 fixes |
| `AddPurecallHandler` | 0 | 0 [source] | Replace `_purecall` handler with dump-generating crash |
| `PreventPurecall` | 1 | 0 [source] | Return 0 from the guarded virtual call (roadblock risk) |

Note: the code's fallback for two keys (`StabilityFixes`, `PreventPurecall`) is 0, but the shipped INI sets them to 1. Which applies when the INI is missing is not confirmed [inferred from the source reading].

## 5. How we could implement it in nfs-mw

Licence and process. The repo is GPL-3.0. nfs-mw is MIT OR Apache-2.0 (see project memory). Copying or closely paraphrasing the code would bring GPL terms into our tree, so the rule is: write specs from the observed behaviour and the game's data, then implement clean-room. Use the addresses below only as hypotheses to verify against the install, not as copied code.

What it would look like in the Rust rewrite:

- The whole approach (writing machine code into a running game process) does not apply to a clean-room rewrite. The rewrite reads the game's data and runs its own physics and AI, so it has no need to patch the original executable. Loader-specific hacks (DLL entry-point check, `.asi` loading, `VirtualProtect` patching, `E8`/`E9` redirection) do not apply and should not be ported.
- The behaviours are still useful as bug notes. Each one describes a defect the rewrite must avoid rather than reproduce:
  - Race timer should never go backwards on a checkpoint or reset. Spec the timer rules in `docs/` from observed behaviour first.
  - Lookups by string key must handle an empty or absent key (no null dereference). Applies to any attribute table lookup in `libs/` or `crates/`.
  - Virtual or trait dispatch on an object not yet fully constructed must not reach an empty slot. Rust has no virtual-call-to-pure failure, but the same logic gap (spawning a roadblock before its data exists) must be checked in game logic.
  - Memory-fitness checks are a loader/allocator concern in the original; in the rewrite, out-of-bounds data reads should be caught by the parser, not by a runtime check.
- Data or formats needed: the race-timer representation and its update rule; the roadblock spawn data (format reading belongs to `crates/` or a format crate in `libs/`). Pending the spec for races and zones already in the repo (`docs/formats`).
- Natural home:
  - `libs/` for engine-generic behaviour (a timer type that is monotonic, string-key lookup that returns `Option`).
  - `crates/` for NFS:MW-specific behaviour (roadblock spawn rules, race-timer rules for this game).
  - Dependency direction: `crates/` may use `libs/`, never the reverse.
- Risks:
  - Behaviour may differ between the 1.3 build targeted here and other builds; the repo is 1.3 only.
  - The fixes are "fixes for the original binary's crashes". A clean rewrite may not have these crashes at all, so each item should be re-tested rather than assumed.
  - The purecall fix trades a crash for a missing roadblock; the rewrite must not silently skip spawns.
  - Repo activity stopped in February 2018, so there are no later bug reports to check against.

## 6. Open questions and what could not be verified

- Whether the DLL is loaded by an ASI loader, a proxy DLL, or by hand: not stated; the loader is not in the repo.
- Full contents of `include/injector/` and `include/IniReader.h`, `include/ini_parser.hpp`: listed but not read in full.
- The Visual Studio project files (`MWFixes.vcxproj`, filters, `.sln`) were not read; the language share and build flags are unverified.
- Whether the address table is correct for any 1.3 build other than the one the author used. The entry-point check only covers one value.
- Whether the timebug fix is effective: no evidence in the README or code beyond the code itself.
- Latest release contents: the Releases section is empty on the repo page, so no binary was checked (and none was to be downloaded).
- Issue tracker contents: not read; the README directs bug reports to issues, but the page did not show any.
- Whether the "Readme.txt" (install note) and the README agree on every detail: both say copy the files to the install directory; the 1.3 limitation is in both.

## 7. Sources

- https://github.com/GrimMaple/mwfixes
- https://raw.githubusercontent.com/GrimMaple/mwfixes/master/README.md
- https://raw.githubusercontent.com/GrimMaple/mwfixes/master/Readme.txt
- https://raw.githubusercontent.com/GrimMaple/mwfixes/master/LICENSE
- https://github.com/GrimMaple/mwfixes/tree/master/src
- https://raw.githubusercontent.com/GrimMaple/mwfixes/master/src/dllmain.cpp
- https://github.com/GrimMaple/mwfixes/tree/master/include
- https://raw.githubusercontent.com/GrimMaple/mwfixes/master/include/CPatch.h
- https://github.com/GrimMaple/mwfixes/tree/master/MWFixes
- https://raw.githubusercontent.com/GrimMaple/mwfixes/master/MWFixes/MWFixes.ini
- https://github.com/GrimMaple/mwfixes/commits/master
