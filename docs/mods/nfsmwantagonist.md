# NFSMWAntagoNISt (AntagoNISt)

AntagoNISt is a Windows DLL mod for the 32-bit PC version of Need for Speed: Most Wanted (2005). It replaces the background traffic and police cars that appear in selected NIS (non-interactive sequence) cutscenes, such as the Blacklist rival race entrances and the final pursuit ending. It works by hooking the game's initialisation and one vehicle-selection path, and it reads a user-editable INI file that maps each cutscene to eight vehicle names.

| Field | Value |
|---|---|
| Repository | https://github.com/rng-guy/NFSMWAntagoNISt |
| Owner / author | rng-guy (repository owner). No separate author name found in the files read. |
| Type | .asi-style plugin (DLL with an ASI-loader dependency, per README). Source is C++. |
| Licence | CC0 1.0 Universal (from LICENSE.txt). |
| Language | C++20, MSVC, Visual Studio 2022 or newer (checked by a compile-time guard in dllmain.cpp). GitHub's language bar was not visible in the fetch. |
| Target game build | Need for Speed: Most Wanted (2005) PC, 32-bit. The entry point must be 0x3C4040. README says `speed.exe` must be 5.75 MB (6,029,312 bytes). |
| Latest visible activity | Not shown. The page lists 28 commits on master but no commit date was visible. Archive name in README: NfSMW_AntagoNISt_v1.00.4.7z (mod version 1.00.4). |
| Status of this page | Read from the repo's public pages only. Nothing was cloned, downloaded or run. |

## 1. What it does

Each feature below is listed with its evidence.

- **Replaces background vehicles in supported NIS cutscenes.** The README says it swaps background vehicles in "cutscenes that support them", naming the Blacklist rival race entrances and the final pursuit ending. [README]
- **Per-cutscene vehicle lists.** Each of 16 cutscene sections (for example IntroNisDD, IntroNisBL15, EndingNis04) takes eight vehicle names, `car1` to `car8`. [Config/NFSMWAntagoNIStSettings.ini]
- **Fallback to vanilla.** A cutscene group that does not have exactly eight valid names keeps the original vehicles. [Config/NFSMWAntagoNIStSettings.ini header comment]
- **Vehicle validation against the game data.** A name is accepted only if the game's vault data has a `pvehicle` node whose `CLASS` attribute is `CAR` or `TRACTOR`. [Source/dllmain.cpp, GetVaultNode/GetNodeAttribute path; Config header comment]
- **Off switch.** Deleting the settings file disables the tool. [Config/NFSMWAntagoNIStSettings.ini header comment]
- **Bundling permission.** The README says AntagoNISt may be bundled with other mods, with "no credit required". [README]
- **Compatibility claims.** README says VltEd and binary mods should be fully compatible and that almost all other .asi mods should be too. [README] Not tested here.
- **Config-only tuning.** Users edit the INI file; no rebuild is needed. [README "Configuring"]

## 2. How it does it

### Load and gating
- The DLL checks on `DLL_PROCESS_ATTACH` that the executable entry point equals `0x3C4040`. If not, it shows a `MessageBoxA` error and returns `FALSE`. [Source/dllmain.cpp]
- It installs a hook on the initialisation function `InitializeEverything` at `0x665FC0`, through `PATCH_HOOK_FUNCTION(Initialise, 0x6665B4)`. [Source/dllmain.cpp]
- The hook calls the original function first. A debug build spins until a debugger attaches. [Source/dllmain.cpp]
- The hook opens `scripts/NFSMWAntagoNIStSettings.ini`, parses it, and applies patches only if at least one scene is valid. [Source/dllmain.cpp]

### Patches
Two assembly detours are applied. [Source/dllmain.cpp]

- **SceneVehicles** (code range `0x6F5607`–`0x6F5610`). It looks up the replacement types for the current scene, sets a `replacementTypes` value, calls `PrepareVehicles` (`0x6F2F60`) with `ecx = esi - 0x48`, then clears `replacementTypes`.
- **VehicleSource** (code range `0x6F30CB`–`0x6F30D1`). If `replacementTypes` is set, it writes that value to `[esp + 0x80]` and jumps to `replacementExit` (`0x6F3159`). Otherwise it runs the original `sub ebx, 3` / `cmp ebx, 7` instructions.

### Game functions used
- `GetVaultNode` at `0x455FD0` (`__cdecl`). [Source/dllmain.cpp]
- `GetNodeAttribute` at `0x454190` (`__thiscall`). [Source/dllmain.cpp]
- `PrepareVehicles` at `0x6F2F60`. [Source/dllmain.cpp]
- `InitializeEverything` at `0x665FC0`, hooked as above. [Source/dllmain.cpp]

### Name hashing
- Vehicle and scene names are converted to the game's vault hash by `GetVaultHash`, a Bob Jenkins-style mix seeded with `0x9E3779B9` and `0xABCDEF00`. A `_vlt` literal operator computes the hash at compile time. [Source/dllmain.cpp]

### Code layout
- `Source/dllmain.cpp` holds the entry point, hook and detours. [Source]
- `Source/Headers/` contains `FlatContainers.hpp`, `MemoryTools.hpp` and `StreamParser.hpp`. The listing said "View all files", so there may be more. [Source/Headers listing]
- `Config/NFSMWAntagoNIStSettings.ini` is the only config file. [Config listing]
- Solution and project files are `NFSMWAntagoNISt.sln` and `NFSMWAntagoNISt.vcxproj`. [repo root listing]

### Not verified
- The bodies of `PATCH_HOOK_FUNCTION`, `PATCH_ASSEMBLY_DETOUR`, `ASSEMBLY_DETOUR` and `MemoryTools` are in headers that were not read. The exact byte-level patching method is therefore [inferred] only.
- `StreamParser.hpp` and `FlatContainers.hpp` were not read.

## 3. Configuration

File: `scripts/NFSMWAntagoNIStSettings.ini` (copied from `Config/`). [README, Config listing]

- Section names are scene names. The file has 16 active sections plus one commented-out section. [Config]
  - `IntroNisDD`, `EndingNis03`, `IntroNisBL15`, `IntroNisBL14`, `IntroNisBL13`, `IntroNisBL12`, `IntroNisBL11`, `IntroNisBL10`, `IntroNisBL09`, `IntroNisBL07`, `IntroNisBL06`, `IntroNisBL05`, `IntroNisBL04`, `IntroNisBL03`, `IntroNisBL02`, `EndingNis04`.
  - `IntroNisBL08` is commented out; the file says that scene does not exist and that Jewels uses `IntroNisBL16`, which has no background vehicles. [Config]
- Keys `car1` to `car8` per section. Each value is a vehicle name. [Config]
- Default values are the names written in the shipped file. Examples:
  - `IntroNisDD`: car1 to car3 = `copmidsize_nis_ld`, car4 = `trafstwag`, car5 = `trafficcoup`, car6 = `traf4dsedc`, car7 = `trafminivan`, car8 = `trafpizza`. [Config]
  - `EndingNis04`: car1 to car7 = `copsportghost`, car8 = `copcross`. [Config]
- Some names are marked "(unused)" in the file's comments because that car slot is not shown in that cutscene. [Config]
- No other keys were found. The file has no global options section.

## 4. How we could implement it in nfs-mw

The repo is licensed CC0 1.0, which would permit reuse, but the project rule is still spec-first: describe behaviour and facts, do not copy code. The mechanism below is a description for a clean-room rewrite, not a port.

### Does it apply to a clean-room rewrite?
- **Partly.** The data side applies: a cutscene-to-vehicle mapping, validation of vehicle classes against the game's data, and fallback to vanilla.
- **No, for the loader-specific parts.** The ASI loader, the `0x3C4040` entry-point check, the `PATCH_HOOK_FUNCTION` and assembly detours, and the register-level tricks in the two detours are facts about the original 2005 binary. They do not apply to our Rust rewrite, which has no `speed.exe` to patch. The equivalent is a native data-driven scene model, not a patch.

### Behaviours to specify (spec-first)
1. Scene identifiers: a fixed list of NIS scene IDs that can carry background vehicles.
2. Per scene, an ordered list of eight vehicle references. The index (1 to 8) matters because the cutscene binds each slot to a fixed position.
3. Validation: a reference is valid only if it resolves to a vehicle record whose class is car or tractor.
4. Fallback rule: a scene with fewer or more than eight valid entries uses the original vehicles.
5. Global off switch: no config means no change.
6. Compatibility is a non-goal for the spec; our data layer replaces the binary patch.

### Data and formats needed
- A vehicle catalogue read from the install (vault / VltEd data). The rewrite must read this from the user's install, not bundle it. [docs/architecture.md, install discovery]
- A scene catalogue: scene name, index, and the cutscene's vehicle slots. This must come from our own format research, not from this repo's INI. Mark any values taken from the INI as [README/config] facts, not verified against the binary.
- Config format: our project should choose its own file format and location through the install-discovery path.

### Natural home
- Generic pieces (a scene-to-slot mapping, a validation rule on a vehicle record) fit in `libs/` only if they are engine-generic. Per AGENTS.md, `libs/` must contain no NFS:MW-specific names.
- NFS:MW-specific pieces (the scene names, the eight-slot cutscene layout, the vehicle class rules) belong in `crates/` and depend on `libs/`, never the reverse.
- The format parsing of the vault data belongs in a format crate that takes `&[u8]`. Only `game-install` touches the install. [AGENTS.md]

### Risks
- Cutscene slot semantics are tied to the 2005 build. Our reading must be checked against the install before we trust the eight-slot model.
- Vehicle names that look valid may resolve to records the cutscene cannot render. The original code has no crash guard that the page mentions; we cannot confirm one.
- Asset and rendering checks are outside a data-only spec. The mod changes only vehicle selection, so visual pop-in or missing models are a risk to test in the real game later.
- Licence: CC0 would allow reuse, but the project's spec-first rule still applies. Writing a spec from this repo's observed behaviour should be reviewed for facts-only wording before we rely on it.

## 5. Open questions and what could not be verified

- Commit date and recent activity: not visible in the fetched pages.
- Language bar and repo size: not visible in the fetch.
- Author name: only the owner handle `rng-guy` is visible. No credits section was found in the README.
- Whether the mod works on the latest game patch: README gives a `speed.exe` size check but no tested-build list beyond the 5.75 MB hint.
- Behaviour of the byte-level patch helpers (`PATCH_HOOK_FUNCTION`, `PATCH_ASSEMBLY_DETOUR`) and the `MemoryTools` and `StreamParser` headers: not read.
- Whether all 16 listed scenes are present in every copy of the game: the README does not say.
- Whether the "(unused)" slots are cosmetic or affect the cutscene: the file comments say unused but do not explain.
- Whether `.asi` loader requirements are as described: README says "install an .asi loader" and suggests WideScreenFix by ThirteenAG, but does not say which loader it expects.
- The dllmain.cpp summary came from an automated page summary of the raw file. The line-level addresses should be checked against the source before any use.

## 6. Sources

- https://github.com/rng-guy/NFSMWAntagoNISt (repo root listing and summary)
- https://raw.githubusercontent.com/rng-guy/NFSMWAntagoNISt/master/README.md (README)
- https://raw.githubusercontent.com/rng-guy/NFSMWAntagoNISt/master/LICENSE.txt (licence)
- https://github.com/rng-guy/NFSMWAntagoNISt/tree/master/Source (directory listing)
- https://github.com/rng-guy/NFSMWAntagoNISt/tree/master/Source/Headers (directory listing)
- https://github.com/rng-guy/NFSMWAntagoNISt/tree/master/Config (directory listing)
- https://raw.githubusercontent.com/rng-guy/NFSMWAntagoNISt/master/Source/dllmain.cpp (source)
- https://raw.githubusercontent.com/rng-guy/NFSMWAntagoNISt/master/Config/NFSMWAntagoNIStSettings.ini (config)
