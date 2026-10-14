# NFSMWHPC (NFSMW Hot Pursuit Challenges companion script)

## Summary

NFSMWHPC is a C++ Windows DLL, built as a Visual Studio project, that is the companion script for elaymm4's "NFSMW Hot Pursuit Challenges" mod for Need for Speed: Most Wanted (2005, PC). It loads into the game process at startup, checks that the executable is the expected build, and then applies a set of fixed-address memory patches and code caves. The patches add or change Hot Pursuit Challenge menu behaviour, hide some options and menu entries, change a few default values, and tweak shadow and car-skin settings. The README says users do not need to compile or download it separately because the mod already bundles it.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/nlgxzef/NFSMWHPC |
| Author | nlgxzef (repo owner, per GitHub URL). No author byline in the README. |
| Type | Native DLL (C++ / Visual Studio project), companion to a mod. Not an `.asi` file by name; see section 2. |
| Licence | None stated. No LICENSE file was found at the repo root (HTTP 404 on the LICENSE path). Treat as all rights reserved until the author says otherwise. |
| Language | C++ (`.cpp`/`.h`, `NFSMWHPC.vcxproj`). The GitHub language breakdown was not readable. |
| Target game build | NFS:MW v1.3 "speed.exe" (5.75 MB, 6,029,312 bytes), per the error message in `dllmain.cpp`. |
| Latest visible activity | Not verified. The repo page shows 9 commits on main; no date was readable. |

## 1. What it does

Each feature below is stated in the code or README. Confidence tags are defined in section 3.

- **Refuses to run on other builds.** If the entry-point check fails, the DLL shows an error box naming v1.3 speed.exe and returns FALSE. Evidence: `dllmain.cpp` [source].
- **Refuses to run with the Russian release.** If `Languages\Russian.bin` or `..\Languages\Russian.bin` exists, it shows an error. Evidence: `Init()` in `HPCStuff.h` [source, via summary].
- **Hides the Tutorial button on the Challenge Series screen.** The button is moved off-screen and made invisible. Evidence: `CodeCaves.h`, `HideTutorialButtonCodeCave` [source, via summary].
- **Replaces the main menu constructor.** `UIMain_Setup` is called in place of the original `UIMain` constructor. Evidence: `HPCStuff.h` [source, via summary].
- **Disables the Game Stats main menu entry.** Evidence: `HPCStuff.h` [source, via summary].
- **Removes Over Bright and Visual Treatment from the video options.** Evidence: `HPCStuff.h` [source, via summary].
- **Changes the audio default.** Interactive Music default is set to 0 (off). Evidence: `WriteMemory<int>(0x56D61F, 0, true)` in `HPCStuff.h` [source, via summary].
- **Changes Quick Race pursuit content.** Roadblocks, ground support and helicopter spawns are patched in the Quick Race pursuit code. Evidence: `HPCStuff.h` [source, via summary].
- **Changes the shadow look.** Shadow pop-in is fixed with a set value, shadow FOV uses an aspect-ratio-based calculation, and Full Shadow LOD is forced. Evidence: `CodeCaves.h` and `HPCStuff.h` [source, via summary].
- **Changes time-of-day and rain code.** Two addresses are patched. Evidence: `HPCStuff.h` [source, via summary]. The intent is not stated.
- **Skips Career mode in car category switching.** Evidence: `HPCStuff.h`, `Game_IsCareerMode` / address `0x7BF970` [source, via summary].
- **Hides the "Defaults" option and its controls** on the options screens (PC Defaults group, Console Defaults text, Console Circle button). Evidence: `CodeCaves.h` [source, via summary].
- **Moves the PC Clear button** to where the hidden Defaults button was on the controller options screen. Evidence: `HideDefaultsCodeCave_Controller` in `CodeCaves.h` [source, via summary].
- **Hides Quick Play from the Quick Race menu.** Evidence: `HPCStuff.h` [source, via summary].
- **Raises the car skin count from 4 to 20** and changes ride height, unless the NFSMW Unlimiter or NFSMW Limit Adjuster ASI is loaded. Evidence: `HPCStuff.h`, `WriteMemory<unsigned char>(0x75D2B6, 20, true)` [source, via summary]. The comment in the code says 20 dummy skin and wheel textures are needed in `CARS\TEXTURES.BIN`.

## 2. How it does it

- **Loading.** The DLL is a standard Windows DLL with a `DllMain` that acts on `DLL_PROCESS_ATTACH` [source, `dllmain.cpp`]. It is loaded into the game process by a loader. The loader is not named in the repo. Whether it is an ASI loader is [inferred]: the code checks for `.asi` files by name, which suggests the mod is loaded as an ASI, but the repo does not say how it is loaded.
- **Build check.** `DllMain` gets the base address of the main module, reads the PE header, and computes the entry-point RVA. It compares the result with `0x7C4040`. If they match, it calls `Init()` [source, `dllmain.cpp`]. The expression reduces to a check that the entry-point RVA equals `0x3C4040`, which identifies the v1.3 build [source, derived from `dllmain.cpp`].
- **Patching.** `Init()` in `HPCStuff.h` applies the patches. The mechanisms are:
  - `injector::MakeCALL` and `injector::MakeJMP` to redirect calls and jumps to replacement code [source, via summary].
  - `injector::WriteMemory<T>` to write fixed values [source, via summary].
  - NOP-ing of instructions [source, via summary].
  - Replacement of a vtable entry (for the main menu) [source, via summary].
  - Naked-function "code caves" that run at a fixed return address. They save and restore registers with `pushad`/`popad` and return with `retn` [source, via summary; not verified line by line].
- **Helper library.** The project vendors the `injector` header set (`CPatch.h`, `injector.hpp`), `IniReader.h` and `ini_parser.hpp` under `includes/`, and the usual MSVC `stdafx`/`targetver` files [source, repo listing].
- **Other mods.** `GetModuleHandleA` is used to detect `NFSMWUnlimiter.asi` and `NFSMWLimitAdjuster.asi`. If either is loaded, the car-skin patch is skipped [source, via summary].
- **Addresses used.** Patch and hook addresses are fixed for the v1.3 binary. The full list is in `HPCStuff.h` and `CodeCaves.h`. The main ones are listed in the summary above (for example `0x54FD11`, `0x89EAE4`, `0x6E4668`, `0x75D2B6`). They are not re-listed here so this page does not reproduce game addresses at length; they can be read from the source.
- **DirectX hooks.** None found in the files read [source, via summary of `HPCStuff.h`].
- **Config files.** None read or written by the code excerpts seen. `IniReader.h` is included in `dllmain.cpp` and `HPCStuff.h`, which suggests an INI reader is present, but no INI file name or key was found [inferred, unverified].

Notes on confidence:
- [source] means the claim was seen in code. For `dllmain.cpp` the full text was read. For `HPCStuff.h` and `CodeCaves.h` the WebFetch tool returned a summary, not the raw text, so those claims are tagged [source, via summary] and should be checked against the raw file before relying on them.
- [README] means the claim comes only from the README. The README is short and only says what the companion script is.
- [inferred] means a reasonable reading that the code does not state.

## 3. Confidence tags

- [source]: verified in code. Where a claim relies on a fetch summary rather than the raw file, it is marked "via summary".
- [README]: stated only in the README. Used for the purpose statement and the "no need to compile" note.
- [inferred]: reasoned from code names or structure, not stated.

## 4. Configuration

No user-facing configuration was found.

- No INI file name, key or default was found in the code read.
- The defaults that the code sets (as opposed to reads) are:
  - Car skin count: 20 (was 4), unless an Unlimiter or Limit Adjuster ASI is present [source, via summary].
  - Interactive Music: off (set to 0) [source, via summary].
  - Full Shadow LOD: forced to 0 [source, via summary].
  - Shadow pop-in value: 360.0 (`0x43B40000`) [source, via summary].
  - Shadow multiplier: 0.85 (`0x3F59999A`) [source, via summary].
- Whether any of these can be changed by the user is not known. [inferred]: they are compiled-in constants.

## 5. How we could implement it in nfs-mw

Ground rules. The repo has no licence, so its code must not be copied. Everything below is written as behaviour to specify, not code to port. The spec-first process in `docs/licensing.md` applies.

Does this feature apply to a clean-room rewrite? Mostly no.
- The DLL, the entry-point check, the fixed-address patches, the code caves and the `WriteMemory` calls are all loader-specific hacks against the original `speed.exe`. A Rust rewrite does not patch the original binary, so none of that mechanism applies.
- The behaviours the mod changes are game-design and UI choices. They can be expressed as settings and rules in a rewrite, and that part does apply.

Behaviours to specify (candidates for `docs/specs/` if the project wants them):
1. Quick Race pursuit content toggles: whether roadblocks, ground support and helicopters appear in pursuits.
2. Audio default: Interactive Music default off.
3. Shadow settings: a shadow LOD setting, and a shadow FOV rule based on aspect ratio and a multiplier.
4. Menu visibility: which options screen entries (Defaults, Quick Play, Game Stats, video options) are shown.
5. Car skins: number of skin slots per car, and how an external skin-count tool would interact with it.
6. Challenge Series screen: whether the Tutorial entry is shown.
7. Career-mode skipping in car category switching.

Data and formats needed:
- A settings file format (the rewrite can choose; the repo's INI usage is unverified).
- The menu and screen layout for the options and Challenge Series screens. In the rewrite these come from our own FEng/UI data, not from game addresses.
- For the skin count, the `CARS\TEXTURES.BIN` layout would need a format spec from `docs/research.md` or the format docs. This is not yet documented in this repo's `docs/formats` as far as this page can tell.

Natural home:
- Engine-generic parts (a settings-file reader, UI visibility rules, a generic "option set" model): `libs/`.
- NFS:MW-specific parts (the Challenge Series screen, the Quick Race pursuit rules, car skin counts, shadow values): `crates/`. `crates/` may depend on `libs/`, never the reverse.
- No NFS:MW names, paths or defaults in `libs/`. Game differences go in per-version layout tables or are passed in by the caller (AGENTS.md).

Risks:
- Behaviour may be tied to the v1.3 build. The rewrite should not assume the same content or screen flow as the original.
- Several tweaks change game balance (pursuit content, shadow look). The rewrite should expose them as settings, not as hard-coded defaults, and state the default choice explicitly.
- The shadow FOV and pop-in values are magic numbers in the source. The spec should explain what each one does before any value is copied into a spec. Values are facts, but the rewrite must re-derive or test them.
- The repo has no licence. Do not copy its code or comments. Behaviour observed by playing the game or from our own format research is the basis.

## 6. Open questions and what could not be verified

- Licence: none stated; LICENSE returned HTTP 404. Confirm with the author before any reuse of text or code.
- Latest activity date and the full commit history were not readable. The repo page reports 9 commits on main.
- `Init()` in `HPCStuff.h` was read only as a summary. Exact addresses, byte patches, and the full list of hooks need the raw file.
- `CodeCaves.h` was read only as a summary. Return addresses and the exact patched instructions were not checked against the raw text.
- The Controller cave may have a bug: its null check tests `DefaultsButton` but then calls `FEngSetCenter` on `ClearButton` (WebFetch summary). Verify in the raw file before relying on it.
- The loader is not named. Whether it is an ASI loader is inferred from the `.asi` checks, not stated.
- `IniReader.h` is included, but no config key or file name was found. It is unknown whether any user-facing config exists.
- The purpose of the time-of-day and rain patches (`0x600AEA`, `0x605610`) is not stated.
- `includes/` listing shows `injector/`, `CPatch.h`, `IniReader.h`, `ini_parser.hpp` and `stdafx.h`. Only the file names were read, not their licences. The vendored `injector` library probably carries its own licence, which would be a separate question.
- Whether the mod is still maintained, and whether it works with later game patches, is not known.

## 7. Sources

- https://github.com/nlgxzef/NFSMWHPC (repo page; description, file listing, commit count)
- https://raw.githubusercontent.com/nlgxzef/NFSMWHPC/main/README.md (README; WebFetch returned a summary, no build or config section)
- https://raw.githubusercontent.com/nlgxzef/NFSMWHPC/main/dllmain.cpp (full text returned)
- https://raw.githubusercontent.com/nlgxzef/NFSMWHPC/main/CodeCaves.h (summary returned)
- https://raw.githubusercontent.com/nlgxzef/NFSMWHPC/main/HPCStuff.h (summary returned)
- https://github.com/nlgxzef/NFSMWHPC/blob/main/LICENSE (HTTP 404; no licence file)
- https://github.com/nlgxzef/NFSMWHPC/tree/main/includes (directory listing; reported `injector/`, `CPatch.h`, `IniReader.h`, `ini_parser.hpp`, `stdafx.h`)
