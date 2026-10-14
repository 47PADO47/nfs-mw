# NFSMWMotionBlurController

NFSMWMotionBlurController is a small Windows DLL mod for the 2005 PC release of Need for Speed: Most Wanted. Per its README, it tweaks the vanilla motion blur effect: it can switch between the widescreen-fix and vanilla blur paths, scale blur strength, set the minimum vehicle speed at which blur applies, and re-read its INI file while the game runs. The source is small (a DllMain, an INI reader, a file-watch thread and a memory-write helper), and it writes fixed game addresses directly. The repository has no licence file, so its code must not be copied into nfs-mw. Only the behaviour and the game facts it exposes are useful to us.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/nlgxzef/NFSMWMotionBlurController |
| Author | nlgxzef (GitHub account; no further identity stated) |
| Type | Mod: C++ DLL (`dllmain.cpp`, Visual Studio project), loaded into the game process |
| Licence | None stated. No LICENSE file in the root listing; raw LICENSE fetch returned HTTP 404. Treat as all rights reserved. |
| Language | C++ (Visual Studio `.sln` / `.vcxproj`); language percentages not captured |
| Target game build | NFS:MW v1.3 (`speed.exe`), per the unsupported-executable message in `dllmain.cpp` [source, as summarised by WebFetch] |
| Latest visible activity | Last commit 13 March 2023; 7 commits total (GitHub commits page, fetched 2026-10-14) |
| Credits (README) | Requested by P A I N; Aero; ThirteenAG; ThomasMonkman |

## 1. Summary

The mod is a single DLL that the game process loads at start. On attach it checks that the host executable is the expected v1.3 build. If so, it reads a settings INI, writes the motion blur values into game memory, and starts a background thread that polls the INI and re-applies changes. The README lists four user-facing features. No function hooks are installed; all changes are direct memory writes.

## 2. What it does

Each feature below is taken from the README feature list unless marked otherwise.

- **Toggle motion blur between Widescreen Fix and Vanilla implementation.** [README] The INI key `MotionBlur` controls this. [README, INI key names as summarised from source]
- **Modify motion blur strength.** [README] Key `MotionBlurAmount`, default 0.25, vanilla value 1.0. [INI, as summarised]
- **Modify effective speed.** [README] Key `MotionBlurMinEffectiveSpeed`, default 1.0, vanilla value 25.0, unit m/s, the minimum player vehicle speed before blur applies. [INI, as summarised]
- **Reload configuration automatically.** [README] A watcher thread polls the INI; `ThreadDelay` sets the poll interval. [source, as summarised]
- **Refuse to load on the wrong executable.** Shows an error box and fails the DLL load if the entry point is not the expected value. [source, as summarised]
- **Enable the in-game motion blur variable.** [INI] Key `g_MotionBlurEnable`, default 1. Not listed in the README feature list. [INI, as summarised]
- **Note for users.** Motion blur must already be enabled in the game's own config or registry for the mod to have any effect. [README]

## 3. How it does it

Claims are tagged as follows: [source] verified in code (here: read through a WebFetch summary, not a full checkout, see section 6); [README] stated only in the README; [inferred] our reading.

### Loading

- The DLL entry point `DllMain` acts only on `DLL_PROCESS_ATTACH`. [source]
- It gets the host image base with `GetModuleHandleA(NULL)`, walks the PE headers (`IMAGE_DOS_HEADER`, `e_lfanew`, `IMAGE_NT_HEADERS`), and compares the entry-point RVA against a constant. [source]
- On mismatch it calls `MessageBoxA` with `MB_ICONERROR` and returns `FALSE`. On match it calls `Init()`. [source]
- Loading as an `.asi` plugin through an ASI loader is [inferred] from the `.asi`-style INI, DLL-entry and file layout. The README does not name a loader. Not verified.

### Initialisation and config

- `Init()` runs an initial reload (`ReadConfig` then `SetValues`) and starts `FileWatchThread` with `CreateThread`. [source]
- `ReadConfig` reads integers and floats via a small `IniReader` helper. [source]
- `FileWatchThread` uses a `filewatch::FileWatch<std::string>` on the INI and calls `ReloadConfig` on a `modified` event, sleeping `ThreadDelay` seconds between checks. [source]

### Memory writes

All effect changes are done by `injector::WriteMemory` calls (from a bundled `injector.hpp` helper), run once at load and again on each reload. [source, as summarised]

- A one-byte toggle at **0x6DF1D2** is patched to `0x74` (JE) when `MotionBlur` is on, or `0xEB` (JMP) when off. This switches between two code paths. [source]
- The in-game enable variable `g_MotionBlurEnable` is written with the INI value. The symbol is defined in `GlobalVariables.h`, which was not read. [source]
- Four pointer or immediate sites at **0x6DBD46, 0x6DBD56, 0x6DBD66, 0x6DBD74** receive the address of the `MotionBlurAmount` float. They are referenced from a game function named `sub_6DBB20` in the source comments. [source]
- The float `MotionBlurEffectiveSpeed` (named `_MotionBlurEffectiveSpeed`) is written and read by `sub_6DBB20`. [source]
- No function hooks are installed. [source]
- No pattern scans are used. [source, as summarised]

### Files

- `includes/` holds `CPatch.h`, `FileWatch.hpp`, `IniReader.h`, `ini_parser.hpp`, `stdafx.h`, and an `injector/` subfolder whose contents were not listed. [source]
- Third-party headers (FileWatch, injector, an INI parser) are present in `includes/`. Their licences were not checked. They must not be reused without checking.

## 4. Configuration

File: `NFSMWMotionBlurControllerSettings.ini`. Values as summarised from the file (the raw file fetch was summarised, not returned verbatim, so treat exact formatting as unverified).

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[Main]` | `MotionBlur` | `1` | 1 = Widescreen Fix path, 0 = Vanilla path |
| `[Main]` | `g_MotionBlurEnable` | `1` | Enables the vanilla in-game motion blur variable |
| `[Main]` | `MotionBlurAmount` | `0.25` | Blur strength; vanilla value 1.0 |
| `[Main]` | `MotionBlurMinEffectiveSpeed` | `1.0` | Minimum speed in m/s before blur applies; vanilla value 25.0 |
| `[Misc]` | `ThreadDelay` | `5` | Seconds between INI change checks |

Note: the source summary gives `ThreadDelay` in seconds in one place and in milliseconds in another. Unresolved, see section 6.

## 5. How we could implement it in nfs-mw

This mod is a tool for a binary that we do not run. A clean-room rewrite does not need its loader, its memory patches or its file watcher. What carries over is the behaviour.

### Does the mechanism apply?

- **Loader-specific hacks: do not apply.** The DLL entry, PE header walk, `MessageBoxA` failure path, `GetModuleHandleA`, `injector::WriteMemory` and the fixed addresses are the mod's method for patching the original exe. The rewrite replaces the game's runtime, so it needs no equivalent. Say so in any design note.
- **The fixed addresses (0x6DF1D2 and the rest): do not port.** They are the original binary's layout. Our effect would be implemented in the rewrite's own renderer code, with its own parameters.
- **The behaviours and their parameter ranges: apply.** Blur on or off, a strength value, a minimum speed threshold and live reload are game-facing features of the effect. Describe them as specs.

### Behaviours to specify (spec-first, from our own evidence)

1. Motion blur has two selectable modes: a widescreen-fixed mode and a vanilla mode. Specify what each mode changes in the rendered frame, from our own captures.
2. Blur strength is a float with a vanilla reference value. Specify the range and the reference we measure ourselves, not the mod's default.
3. Blur is gated by a minimum vehicle speed in m/s. Specify the threshold, its unit and where speed is read from.
4. Settings can change at runtime without restarting. Specify the reload trigger and what state is rebuilt.
5. The vanilla in-game enable flag is a separate switch from the mod's mode switch. Specify their precedence.

### Data and formats needed

- Our own measured vanilla values for strength and speed threshold, from the v1.3 build we already read. Do not cite this mod's numbers as authority.
- A settings file format. An INI reader fits, since the mod already uses one. The parser must be a clean implementation or a permissively licensed crate, chosen under `docs/licensing.md`.

### Natural home

- **Generic pieces go in `libs/`:** a typed settings-file reader with change detection and debounced reload. These are engine-generic and must not contain NFS:MW names, paths or defaults. Per AGENTS.md, `libs/` stays Bevy-free unless it is a renderer backend leaf.
- **NFS:MW pieces go in `crates/`:** the motion blur effect itself, its default values, the speed threshold source and the mode names. `crates/` depends on `libs/`, never the reverse.
- Any renderer work that touches the blur pass belongs in a backend crate per ADR 0004 (`docs/decisions/0004-swappable-renderers.md`), not in the game crate.

### Risks

- **No licence.** Do not copy any code, identifiers, comments or the INI file layout beyond what the game needs. Write specs from our own evidence.
- **Build drift.** The mod targets v1.3 and was last touched in March 2023. Its addresses cannot be assumed valid for any other build. Our reference is the install we read, not this repo.
- **Unverified mechanism.** The two-path toggle and the exact behaviour of `sub_6DBB20` are known only from a summary. Confirm them in our own analysis before relying on them.
- **Credit and attribution.** The README credits people for their work. If we rely on any of their findings, we need a licence or our own derivation.

## 6. Open questions and what could not be verified

- **Full source not read.** The raw `.h`/`.cpp` files were fetched through WebFetch, which returned summaries, not verbatim text. Claims tagged [source] are accurate to that summary only. A checkout was not allowed, so nothing was compiled or run.
- **Raw INI and README not returned verbatim.** The INI and README fetches came back as paraphrase. Exact key spelling, whitespace, comments and any extra keys are unverified.
- **Entry-point constant.** The summary gives the check as `AddressOfEntryPoint == 0x3C4040` in one place and `0x7C4040` in another, and its arithmetic about the `0x400000` rebase term looks inconsistent. The correct constant is unresolved.
- **ThreadDelay units.** Seconds in the INI summary, milliseconds in the table. Unresolved.
- **`injector/` and `GlobalVariables.h`** were not listed or read in full. Symbol names and the `sub_6DBB20` identification come from the summary only.
- **Loader.** No ASI loader is named in the README. The `.asi`-style layout is [inferred].
- **Licence.** Only the root listing and a 404 on LICENSE were checked. Other licence files (for example `COPYING`, `NOTICE`) and per-file headers were not checked. The statement "no licence" rests on the root listing.
- **Activity.** Only the commit listing page was read. Issues, releases and forks were not checked.
- **Game version.** The v1.3 target comes from one error string in a summary. Not confirmed against the binary.

## 7. Sources

- https://github.com/nlgxzef/NFSMWMotionBlurController (repository root listing, fetched 2026-10-14)
- https://raw.githubusercontent.com/nlgxzef/NFSMWMotionBlurController/main/README.md (returned as summary)
- https://raw.githubusercontent.com/nlgxzef/NFSMWMotionBlurController/main/NFSMWMotionBlurControllerSettings.ini (returned as summary)
- https://raw.githubusercontent.com/nlgxzef/NFSMWMotionBlurController/main/dllmain.cpp (returned as summary)
- https://raw.githubusercontent.com/nlgxzef/NFSMWMotionBlurController/main/MotionBlurControllerStuff.h (returned as summary)
- https://github.com/nlgxzef/NFSMWMotionBlurController/tree/main/includes (folder listing)
- https://raw.githubusercontent.com/nlgxzef/NFSMWMotionBlurController/main/LICENSE (HTTP 404)
- https://github.com/nlgxzef/NFSMWMotionBlurController/commits/main (commit date and count)
- Referenced in the page but not fetched: https://www.github.com/nlgxzef/NFSMWMotionBlurController/releases (README download link; not read)
- Repository references in the README credits: https://www.github.com/AeroWidescreen, https://www.github.com/ThirteenAG, https://www.github.com/ThomasMonkman (not read)
