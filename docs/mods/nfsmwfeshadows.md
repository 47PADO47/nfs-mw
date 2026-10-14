# NFSMWFEShadows (NFSMW FE Shadows)

Repository: https://github.com/AeroWidescreen/NFSMWFEShadows

NFSMW FE Shadows is a native mod for Need for Speed: Most Wanted (2005, PC) that turns on dynamic shadows in the front-end menus (Platform Crib, Customization Shop, Career Safe House, Car Lot). The README says the feature was originally available only in the Xbox 360 version. The mod is a DLL dropped into the game folder. It is installed by patching the game's code in memory at startup, with a few small code caves and jump hooks, and it reads a small INI file for four switches. The repo is GPL-3.0 licensed, which means its code cannot be copied into our MIT OR Apache-2.0 codebase. We can only learn from its described behaviour and our own measurements.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | AeroWidescreen/NFSMWFEShadows |
| Author | Aero_ (named as creator in the README and INI header) |
| Type | Native .dll mod (in-process code patching). Not an .asi plugin in the usual sense, although the repo ships `dinput8.dll` (see section 3). |
| Licence | GNU GPL v3 (GPL-3.0). Verified from the LICENSE file (GPLv3, 29 June 2007). |
| Language | C++ (Visual Studio project, `.vcxproj`, `.sln`, `.h`, `.hpp`, `.cpp`). The GitHub language bar showed nothing. |
| Target game build | Need for Speed: Most Wanted (2005, PC). `DllMain` checks the executable entry point RVA against `0x7C4040` and refuses to run otherwise [source]. Other builds are not supported by the code. |
| Latest visible activity | Not visible. The repo page shows 10 commits and no date (fetched 2026-10-14). |
| Related projects | Widescreen Fix (recommended in the README). ExOpts (credited as the base project). Ultimate-ASI-Loader by ThirteenAG (credited in the README). |

## 2. What it does

Each feature below is user-visible, with the evidence.

- **Front-end environment shadows.** Renders the world shadows in the front-end menus. Config key `RenderWorldShadows`. [README: "Renders environment shadows while in the front-end."] [source: `NFSMWFEShadows/dllmain.cpp` hooks `0x6E5186` to `FEShadowCodeCave`.]
- **Front-end vehicle shadows.** Renders shadows under the cars shown in the front-end. Config key `RenderCarShadows`. [README: "Renders vehicle shadows while in the front-end."] Note: the code reads this key but the hook code shown does not branch on it (see section 6).
- **Disable shadows on cars.** Suppresses shadows on vehicles in the front-end, to match the Xbox 360 look. Config key `DisableShadowsOnCars`. [README] [source: when set, a JMP hook at `0x6C8323` goes to `ShadowsTransformCodeCave`.]
- **Experimental restore of missing shadows.** Config key `RestoreShadows`, marked experimental in the INI file. The README does not mention it. [source: hooks `0x6C6A2D` and `0x6DABB5` when set.] [INI comment: brings back missing front-end shadows on original platform models; turn it off if it causes bugs.]
- **Car self-shadowing fix.** Always applied. Hook at `0x74E37F` to `FESelfShadowingCodeCave`. [source] (Not described in the README.)
- **Particle speed fix, ground shadow fix and shadow bias fix.** Always applied. Hooks at `0x7A22F2`, `0x6E0B6E` and `0x6E551B`. [source] (Not described in the README.)
- **No performance cost (claimed).** The README says the mod should not affect performance. [README] Not measured by us.

## 3. How it does it

The mechanism is verified from the source files unless marked otherwise.

- **Delivery.** The install step copies `dinput8.dll` and a `scripts` folder into the game folder [README]. The `dinput8.dll` name is the usual proxy DLL name that a loader uses to load mods at startup. The README mentions the loader only in the credits (Ultimate-ASI-Loader by ThirteenAG). Whether the shipped DLL is that loader, or a copy of it, is not verified from the files we read. [inferred] The `scripts` folder and `.ini` file are read by the DLL [source: `CIniReader`].
- **Entry point check.** `DllMain` on `DLL_PROCESS_ATTACH` computes the EXE entry point RVA and compares it to `0x7C4040`. On a match it calls `Init()`. Otherwise it shows a `MessageBoxA` error and returns `FALSE` [source: `NFSMWFEShadows/dllmain.cpp`].
- **Patching method.** Code is changed through a small injector library in `includes/injector/` (files `injector.hpp`, `hooking.hpp`, `calling.hpp`, `assembly.hpp`, `utility.hpp`, plus a `gvm` folder). The calls used are `MakeJMP`, `MakeNOP` and `MakeCALL` [source]. The library's internals were not read. [inferred from file names only]
- **Hooks (always applied) [source: dllmain.cpp, addresses from the code]:**

  | Address | Action | Target |
  |---|---|---|
  | `0x6E5186` | JMP | `FEShadowCodeCave` (front-end shadow render) |
  | `0x74E37F` | JMP, plus 2-byte NOP at `0x74E384` | `FESelfShadowingCodeCave` |
  | `0x7A22F2` | JMP | `FEParticleSpeedCodeCave` |
  | `0x6E0B6E` | JMP, plus 3-byte NOP at `0x6E0B73` | `FEShadowRenderFixCodeCave` |
  | `0x6E551B` | JMP, plus 1-byte NOP at `0x6E5520` | `ShadowBiasCodeCave` |

- **Hooks (conditional) [source]:**
  - `DisableShadowsOnCars` set: JMP at `0x6C8323` to `ShadowsTransformCodeCave`, plus 1-byte NOP at `0x6C8328`.
  - `RestoreShadows` set: JMP at `0x6C6A2D` to `RestoreShadowsCodeCave` (experimental); CALL at `0x6DABB5` to `ShadowShaderCodeCave`, plus 1-byte NOP at `0x6DABBA`.
- **Code caves.** Naked assembly functions (`FEShadowCodeCave`, `FESelfShadowingCodeCave`, `FEParticleSpeedCodeCave`, `FEShadowRenderFixCodeCave`, `ShadowsTransformCodeCave`, `ShadowBiasCodeCave`, `RestoreShadowsCodeCave`, `ShadowShaderCodeCave`) are defined in `settings.h` and `assembly.h`. Each jumps back into the game at a fixed return address (for example `ShadowBiasCodeCaveExit` at `0x6E5521`, `ShadowsTransformCodeCaveExit` at `0x6C8329`) [source].
- **Game state checks.** The caves read game memory directly. Examples [source, `assembly.h`]:
  - `0x925E90` is compared with `0x03` to detect the front-end state.
  - `0x901830` is a shadows-enabled flag.
  - Other data addresses used: `0x982A20`, `0x982C80`, `0x982BDC`, `0x9195E4`, `0x919C70`, `0x93DB3C`, `0x93DE78`, `0x93DE98`.
  - Object identification uses immediate values `0x2EE02501` and `0x1BF93EF9`, compared against `[esi+0x24]`.
  - Shadow bias data: `ShadowResDivider` = 2048 (int), `ShadowBiasMultiplier` = 2.0f, plus two float results.
- **Direct3D.** `d3d9.h` is included in `dllmain.cpp`, but no Direct3D 9 calls appear in the parts we read. The caves may call D3D or game functions; the functions named in `assembly.h` (`sub_7A1FA0`, `sub_7A2200`, `sub_750980`, `sub_723FA0`, `sub_50D320`) were not decompiled or identified. [inferred: the shadow render path is D3D9 because the game is D3D9 and the header is included; not verified in code.]
- **Config loading.** `CIniReader` `ReadInteger` reads the four keys from section `[MAIN]` of `NFSMWFEShadows.ini` [source].
- **Other calls.** `GetModuleHandleA(NULL)` is used in `DllMain` [source].

## 4. Configuration

File: `NFSMWFEShadows.ini`, section `[MAIN]`. Values are integers (0 or 1).

| Key | INI value in repo | Default in code (`dllmain.cpp`) | Effect |
|---|---|---|---|
| `RenderWorldShadows` | 1 | 1 | Environment shadows in the front-end (read but not branched on in the code shown). |
| `RenderCarShadows` | 0 | 1 | Vehicle shadows in the front-end (read but not branched on in the code shown). |
| `DisableShadowsOnCars` | 1 | 1 | When 1, adds the `ShadowsTransformCodeCave` hook. |
| `RestoreShadows` | 0 | 0 | When 1, adds the experimental restore hooks. |

Note the disagreement: the shipped INI sets `RenderCarShadows` to 0, but the code default is 1. The README gives no defaults. The `settings.h` file does not state defaults. [source]

## 5. How we could implement it in nfs-mw

Ground rules for us:

- The repo is GPL-3.0. Our project is MIT OR Apache-2.0 (see the project rules). Copying or closely porting any of its code would bring GPL obligations into our code, so we must not copy it. We work from behaviour, facts we measure from the game, and our own specification.
- Spec-first: write down the behaviour in a spec under `docs/specs/`, then implement from that spec.
- Our rewrite does not need a loader or in-memory patching. Most of this repo (DLL entry check, JMP/NOP patches, code caves, fixed addresses) does not apply to a clean-room rewrite. Loader-specific hacks do not apply. The feature itself (front-end shadows) does apply, as a rendering feature in our own engine.

Behaviours to specify (from the README and the INI, not from the code):

1. Front-end shadow pass: when the game is in a front-end screen (Platform Crib, Customization Shop, Career Safe House, Car Lot), render environment shadows. This is a rendering spec, not an address spec.
2. Vehicle shadow pass in the same screens, with a toggle.
3. Option to suppress vehicle shadows in the front-end to match the Xbox 360 look.
4. Shadow bias / depth-bias correction so shadows do not swim or self-shadow, and a ground-shadow fix. These are described only by the code names (`ShadowBias`, `FEShadowRenderFix`, `FESelfShadowing`), so we must measure the artefacts ourselves before writing a spec.
5. Particle speed correction in the front-end (the name `FEParticleSpeed` only; the actual behaviour is not described).
6. Settings: four toggles, same names as the INI, so users coming from the mod see the same options.

Data and formats needed:

- Front-end state (which screen is active). We need this from our own UI state, not from memory address `0x925E90`.
- The list of front-end screens and their scene objects. We need our own data from the install, read through the existing install-discovery path.
- Per-object shadow flags. We need to find out what our engine's equivalent of the object tags `0x2EE02501` and `0x1BF93EF9` is, by our own inspection of the game data, and document the result in a `docs/formats/` note. Do not copy the mod's constants without checking them.
- Settings file format: our config format, not INI unless that is already our choice.

Natural home:

- Engine-generic parts (shadow map pass, depth bias, toggle plumbing, config reader) go in `libs/`. They must not carry NFS:MW names, paths or addresses. The mod's names (`FE`, `Xbox`) must not appear in `libs/`.
- NFS:MW-specific parts (which screens count as front-end, which objects are cars, the front-end state source) go in `crates/`. `crates/` depends on `libs/`, never the reverse.
- If the renderer backend is Bevy-based, it lives in `libs/` as an optional leaf crate (ADR 0004). The shadow logic itself stays Bevy-free.

Risks:

- Licence risk: a copy or near-copy of the caves would be a GPL derivation. Mitigation: no reading of the cave code while writing our version; write the spec from observed behaviour first.
- Version risk: the mod is tied to one executable build (the entry RVA check). Our data-driven approach should not need addresses, but any game-state detection we add must be checked against the install we target.
- Visual correctness: the mod uses fixes (bias, particle speed) that exist because of artefacts of the original renderer. We must reproduce the look from our own measurements, with screenshots (`--screenshot`, offscreen, per project rules).
- Performance claim: the README says no performance cost. We have not verified this and should measure in our renderer.
- Scope: this is a cosmetic feature. It should be a later milestone, after the menu and renderer work it depends on, and not block the core game path.

Feature verdict: the front-end shadow feature applies to a clean-room rewrite as a rendering behaviour. The DLL loader, in-memory patching, JMP/NOP hooks, code caves and fixed addresses do not apply and must not be ported.

## 6. Open questions and what could not be verified

- Whether the shipped `dinput8.dll` is Ultimate-ASI-Loader or a custom loader. Not verified; the injector folder is the only other clue. [unverified]
- What the injector library does internally (`hooking.hpp`, `injector.hpp`, `gvm` folder). The files were not read. [unverified]
- What the code caves actually compute. The addresses and names are known, but the body of each cave, and the functions `sub_7A1FA0`, `sub_7A2200`, `sub_750980`, `sub_723FA0`, `sub_50D320`, were not read or identified. [unverified]
- Whether `RenderWorldShadows` and `RenderCarShadows` have any effect. The code reads them but the hook code shown does not branch on them. The header `settings.h` also declares them without defaults. [unverified; possibly dead keys]
- The default value disagreement for `RenderCarShadows` (INI 0, code 1). Not resolved.
- The purpose of `RestoreShadows`. The INI comment and the name are the only sources. [README does not mention it]
- Which game build the addresses belong to beyond the entry RVA check `0x7C4040`. [unverified]
- Latest activity date and commit history. Not visible on the fetched pages. The repo shows 10 commits.
- Full `dllmain.cpp`, `assembly.h` bodies, and `settings.h` were not read in full; the WebFetch summaries were used. The summaries may miss detail. Downloading the repo was not allowed for this task, so these findings should be re-checked by reading the raw files before any design decision.
- The README's install step names a `scripts` folder. The repo does not show a `scripts` folder in its root listing; the INI may be read from the install folder instead. [unverified]

## 7. Sources

- https://github.com/AeroWidescreen/NFSMWFEShadows (repository page: description, top-level listing, licence, commit count)
- https://raw.githubusercontent.com/AeroWidescreen/NFSMWFEShadows/main/README.md (README, via summary)
- https://raw.githubusercontent.com/AeroWidescreen/NFSMWFEShadows/main/NFSMWFEShadows.ini (INI keys and values)
- https://raw.githubusercontent.com/AeroWidescreen/NFSMWFEShadows/main/LICENSE (licence)
- https://github.com/AeroWidescreen/NFSMWFEShadows/tree/main/includes (listing)
- https://github.com/AeroWidescreen/NFSMWFEShadows/tree/main/includes/injector (listing)
- https://github.com/AeroWidescreen/NFSMWFEShadows/tree/main/NFSMWFEShadows (listing)
- https://raw.githubusercontent.com/AeroWidescreen/NFSMWFEShadows/main/NFSMWFEShadows/dllmain.cpp (hooks, entry check, INI reads)
- https://raw.githubusercontent.com/AeroWidescreen/NFSMWFEShadows/main/NFSMWFEShadows/settings.h (settings declarations)
- https://raw.githubusercontent.com/AeroWidescreen/NFSMWFEShadows/main/NFSMWFEShadows/assembly.h (code cave names, addresses, data)
