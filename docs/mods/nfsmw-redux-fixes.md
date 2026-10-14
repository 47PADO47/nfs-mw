# nfsmw-redux-fixes

A small public repository of byte patches and a PowerShell patcher. It changes three third-party `.asi` modules from the Need for Speed: Most Wanted (2005, PC) "Redux V3" modpack so they waste less CPU time per frame. The author reports no visual change and a large frame-rate gain in races on a laptop. The repository contains no game assets and no new code for the game itself; it only rewrites bytes in three existing DLL-style modules.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/1Filipek2/nfsmw-redux-fixes (public) |
| Author | 1Filipek2 (GitHub account name; no other identity shown) |
| Type | Binary patcher for third-party `.asi` plugins, with benchmark notes (performance tool) |
| Licence | None stated. No LICENSE file appears in the repository root listing. The README ends with a note that the mods belong to their authors. |
| Language | PowerShell (`scripts/nfsmw_fixes.ps1`) and batch files. The GitHub language bar was empty in the fetched page. |
| Target build | Need for Speed Most Wanted (2005) with the Redux V3 (v3.03) modpack, Windows 10 or 11 |
| Stars / forks / releases | 3 stars, 0 forks, no releases listed |
| Latest activity | Not visible in the fetched pages. The repository showed 10 commits on `main`, with no dates. |

## 1. What it does

Each feature below is a user-visible effect or a documented setting. Evidence is given after each item.

- **Lower CPU load in races.** The author reports frame rates going from roughly 20-40 FPS (stock Redux V3, in-game counter) to 60-80 FPS on a laptop with an RTX 3060 Laptop GPU at 1080p 144 Hz. Evidence: README, "Patches applied" and "Testing notes"; TECHNICAL.md, "Earlier runs".
- **Hotkey thread stops pinning a core.** The hotkey polling loop in `NFSMWGraphics.asi` sleeps 10 ms instead of 0 ms. Evidence: README; TECHNICAL.md, "NFSMWGraphics.asi"; script byte patch table.
- **Idle worker thread sleeps.** The `io_context` worker in `NextGenGraphics.MostWanted.asi` sleeps 1 ms when it has no work. Evidence: README; TECHNICAL.md, "Worker thread".
- **Material walk runs every 8th frame.** The per-frame walk that assigns the water shader in `NextGenGraphics.MostWanted.asi` runs on every eighth frame. Evidence: README; TECHNICAL.md, "Material walk".
- **No retry of four missing textures.** `MW360Tweaks.asi` stops calling the PNG texture loader every frame for `BlurMask.png`, `ColorTint.png`, `RocksTex.png` and `RocksNormal.png`, which Redux V3 does not ship. The author says nothing changes visually. Evidence: README; TECHNICAL.md, "MW360Tweaks.asi". Note: the author warns not to use this patch if the PNG files are added by hand.
- **Safe patch and restore workflow.** `patch.bat` patches only files whose SHA-256 matches a known original or known patched value, keeps `.orig` backups, and skips files it does not recognise. `restore.bat -GameDir <path>` reverts changes. Evidence: README, "Install and usage"; script, "Hash verification" and "Restore".
- **Optional settings changes (manual, not applied by the patcher).**
  - `Settings.ini` in `SAVE\NFS Most Wanted\`: lowering `g_CarEnvironmentMapEnable` and `g_RoadReflectionEnable`. The author reports frames over 33 ms dropping from 237 to 0 in one 2-minute race. Evidence: README, "Optional settings"; TECHNICAL.md, "Earlier runs".
  - `scripts\NFSMostWanted.WidescreenFix.ini`: `SimRate = -1` (match monitor refresh) instead of `60`, and `WindowedMode = 0` (exclusive fullscreen). The author reports 84 to 146 FPS at 144 Hz with `SimRate = -1`, and present latency falling from 8.9 ms to 3.7 ms in exclusive fullscreen. Evidence: README; TECHNICAL.md, "Measurements".
- **Patcher accepts a per-patch ID.** `-Only <id>` applies a single patch (`hotkeys`, `ngg-worker`, `ngg-walk`, `mw360-png`). Evidence: script, "-Only".

The repository also states that DXVK made performance worse with this modpack. Evidence: README, "Testing notes" [README].

## 2. How it does it

Tags: [source] = verified in the script or in documented addresses, read from the raw file; [README] = stated only in the README or TECHNICAL.md prose; [inferred] = my reading.

### Patcher (`scripts/nfsmw_fixes.ps1`, launched by `patch.bat`)

- Game folder detection: the path from `-GameDir`, else the current directory, else the parent of the script folder. A folder qualifies if it contains `scripts\*.asi`. Otherwise it prompts once and exits with code 1 if the answer does not qualify. [source]
- Refuses to run while any process runs from the game folder. [source]
- Targets three files in `scripts\`: `NFSMWGraphics.asi`, `NextGenGraphics.MostWanted.asi`, `MW360Tweaks.asi`. Missing files are skipped. [source]
- Verifies each file's SHA-256 against hard-coded original, patched and (for NGG) older-patched values. Unknown hashes are left alone. After the bytes are changed in memory, the new hash must match the patched value before anything is written. [source]
- Backs up each original to `<file>.asi.orig` before the first write. Files upgraded from the older NGG patch get no new backup. [source]
- Does not read or modify `Settings.ini` or the WidescreenFix INI. [source]

### Byte patches

The patches are in-place edits of x86 code in the three modules. No DLL injection, no new imports, and no loader is involved; the modules are patched on disk.

- **`NFSMWGraphics.asi` (id `hotkeys`).** A thread created with `CreateThread` at RVA `0x23A0` polls keys with `GetAsyncKeyState` and calls `Sleep(0)` in its loop. Two `push 0` operands (`6A 00`) at VA `0x100023C7` and `0x10002415` (file offsets `0x17C7` and `0x1815`) become `push 10` (`6A 0A`). [README, TECHNICAL.md; byte table matches the script at file offsets `0x17C8` and `0x1816`, value `0x00` to `0x0A`, which is [source]]
- **`NextGenGraphics.MostWanted.asi` (id `ngg-worker`).** An ASIO `io_context` loop starts at VA `0x1007EC10`, calls `run()`, and loops back without pausing when idle. The back-jump at `0x1007EC42` (`74 D2`) is redirected to a stub at `0x1007EC4E` that does `push 1` and calls `Sleep` through the existing import thunk at `0x100D7540`, then jumps back. The stub sits in bytes that were unreachable after a `noreturn` call. The script writes this as file offset `0x7E043` (`D2` to `0A`) and a 9-byte region at `0x7E04E`. [source for the script bytes; the VA details are [README, TECHNICAL.md]]
- **`NextGenGraphics.MostWanted.asi` (id `ngg-walk`).** `0x100706E0` runs every frame and calls `0x100705B0`, which walks the model list at `0x91A0D0`, every mesh and material entry (`0x68` bytes each). For entries whose effect id `[+0x30]` is 0, it hashes `ANM_WATERA_` and `ANM_WATERA_001` with the game's string hash at `0x460BF0`. On a match it sets id 8 and writes the effect pointer from table `0x93DE78`. The patch places a 27-byte stub in the `int3` padding at `0x100706B0` (file offset `0x6FAB0`). The stub increments a counter at `0x1011AFFC` (found relative to its own address via `call`/`pop`) and calls the walk only when the low three bits are zero, i.e. every 8th frame. The call at `0x100706FE` changes from `FE` to `FF` at file offset `0x6FAFE`. [source for the script bytes; the logic is [README, TECHNICAL.md]]
- **`MW360Tweaks.asi` (id `mw360-png`).** Each frame, four texture pointers are checked and `D3DXCreateTextureFromFileA` from `d3dx9_26.dll` is called for any that are null. The four `jne` opcodes (`75`) at VA `0x100015D5`, `0x10001611`, `0x1000163F`, `0x1000166D` (file offsets `0x9D5`, `0xA11`, `0xA3F`, `0xA6D`) become `jmp` (`EB`). [source for the file offsets and opcodes]

### Data and sign-off

- The benchmark method is documented in `bench/` (`record.ps1` captures with PresentMon 2.5.1; `stats.py` analyses frame times) [README, TECHNICAL.md]. I did not read these files.
- The author says the root-level Redux V3 files were verified at the hashes above. I did not verify that against a game install.

### ASI loader, DirectX and hooks

- No ASI loader is named in any file read. The modules are referred to by `.asi` file name only. [README, TECHNICAL.md]
- The documents never use the word "hook". Changes are described as patched call sites, jump redirects and a stub in padding. [TECHNICAL.md]
- DirectX appears only through `D3DXCreateTextureFromFileA` and the `UserGpuPreferences` registry key used to pick the GPU for the exe. [TECHNICAL.md]
- DXVK is mentioned only as a negative test result. [README]

## 4. Configuration

The patcher has no configuration file. Its parameters are:

| Parameter | Default | Effect |
|---|---|---|
| `-GameDir <path>` | current directory, then parent of script folder, then prompt | Game folder containing `scripts\*.asi` |
| `-Only <id>` | all patches | Apply one patch: `hotkeys`, `ngg-worker`, `ngg-walk`, `mw360-png`. Requires original files. Cannot combine with `-Restore`. |
| `-Restore` | off | Revert changes |

The game settings described in the README are not written by the patcher:

| File | Key | Value suggested | Default in benchmark |
|---|---|---|---|
| `SAVE\NFS Most Wanted\Settings.ini` | `g_CarEnvironmentMapEnable` | 1 (reduced car reflection update) | Not stated |
| `SAVE\NFS Most Wanted\Settings.ini` | `g_RoadReflectionEnable` | 2 | 2 (benchmark config) |
| `scripts\NFSMostWanted.WidescreenFix.ini` | `SimRate` | -1 (match monitor) | 60 |
| `scripts\NFSMostWanted.WidescreenFix.ini` | `WindowedMode` | 0 (exclusive fullscreen) | Not stated |

The benchmark also used `g_VSyncOn = 0`, `g_PerformanceLevel = 5`, `g_WorldLodLevel = 3`, `g_CarLodLevel = 1`, `g_ShadowDetail = 2`, and a DirectX GPU preference of `GpuPreference=2` for the exe. [TECHNICAL.md]

Note: the README gives `g_CarEnvironmentMapEnable= 1` and `g_RoadReflectionEnable= 2` as settings to change, but the benchmark list shows `g_RoadReflectionEnable = 2` as a baseline. The page therefore does not state a clear default for that key.

## 5. How we could implement this in nfs-mw

### Summary of applicability

This repository is a patcher for third-party binary mods. It does not apply to the nfs-mw clean-room rewrite as code, and we must not copy or port it: the repository has no licence, and nfs-mw cannot take code from an unlicensed source. Two parts are relevant as facts and behaviour to learn from:

1. The per-frame costs the author found in the original game plus modpack: a busy-wait hotkey poll, a worker loop that never sleeps, a per-frame material walk, and a per-frame retry of missing texture loads. These are performance lessons for our renderer and content loader, not code to copy.
2. The measurement method (frame time, 99th percentile, CPU busy time per frame, sampling of busy threads). This is method, not code, and matches what our performance work will need.

The byte-level patches, offsets and hashes are specific to one Redux V3 build. They do not transfer to nfs-mw.

### Behaviours to specify (spec-first)

Specify these as behaviour in `docs/` before any implementation:

- Input polling interval: the game must not spin on input. Target: input is sampled at a bounded rate with no busy wait, and key-event latency stays within an agreed bound. Measure with the same frame-time method.
- Idle worker behaviour: a background job queue must sleep or block when empty. Target: idle CPU use per background thread near zero.
- Per-frame scene walks: any walk over all materials or meshes must be cached, changed-only, or run at a lower rate for effects that do not need every frame. Target: no full-scene walk per frame unless the state changed.
- Missing-asset handling: a missing optional texture must be recorded once, not retried every frame. Target: at most one load attempt per asset per session, with a logged, visible fallback.
- Frame pacing settings: a simulation-rate setting that can follow the display refresh, and a windowed or exclusive mode choice, with the latency trade-off documented.

### Data and formats needed

- Our own asset and material tables, read from our data layer (the game's install is read, but no game assets are committed). This needs the material list format and an effect table, which must come from our own format spec, not from the modpack.
- A missing-asset registry (path, first failure time, count).
- Frame-time capture output in a format our benchmark tool can read (PresentMon-style CSV is a common target; we would define our own).

### Natural home

- Input polling, worker job queue and frame pacing: engine-generic, so `libs/` (for example a timing or job-system crate). These must not name NFS:MW.
- Material and effect walk, missing-asset registry tied to NFS:MW asset names, and simulation-rate defaults: `crates/` (NFS:MW-specific).
- Patch tooling and binary patching of third-party modules: not planned. Out of scope for a clean-room rewrite.

### Risks

- Copying the code or stub logic from this repository would breach our licensing rule. Specify from behaviour only.
- Performance numbers are from one laptop (i5-11300H, RTX 3060 Laptop). They are the author's, not verified by us.
- Changing simulation rate affects physics timing. Physics is spec-first in nfs-mw, so any `SimRate`-like setting must be tied to the physics spec, not made a free setting.
- Exclusive fullscreen and DXVK results are mixed in the source; do not assume a single best setting.

### Verdict

Not applicable as an implementation. Useful as a checklist of per-frame costs to test for in our own engine. Loader-specific hacks (ASI loader, in-place patching of third-party DLLs, hard-coded addresses) do not apply to a clean-room rewrite.

## 6. Open questions and what could not be verified

- The README and TECHNICAL.md could not be read verbatim. The fetch returned a summary for the README, and I did not read the full raw README text. The patch details come from the full TECHNICAL.md and the script, both fetched as raw files.
- I did not verify the Redux V3 file hashes against a game install, and did not run anything.
- The latest commit date, language bar and full commit history were not visible in the fetched pages.
- It is not clear from the README which byte patch versions were tested with which Redux V3 build, beyond v3.03.
- The `bench/` scripts, `patch.bat`, `restore.bat` and `.gitignore` were listed but not read.
- Whether the author is the mod author or a third party is not stated.
- The README's FPS claims (20-40 to 60-80) and the TECHNICAL.md numbers (84 to 146 with `SimRate = -1`) are not in agreement; the page records both as the author's figures and does not reconcile them.
- The "8% of main-thread time" and "a third of main-thread time" figures are the author's profiling claims; I did not check them.
- No ASI loader name is given in the sources read. If the modpack uses a particular loader, that is unverified here.

## 7. Sources

- https://github.com/1Filipek2/nfsmw-redux-fixes (repository page, via WebFetch; summary of README, file list, licence, stars, language)
- https://raw.githubusercontent.com/1Filipek2/nfsmw-redux-fixes/main/TECHNICAL.md (via WebFetch)
- https://raw.githubusercontent.com/1Filipek2/nfsmw-redux-fixes/main/scripts/nfsmw_fixes.ps1 (via WebFetch)
- https://raw.githubusercontent.com/1Filipek2/nfsmw-redux-fixes/main/README.md (via WebFetch; returned a summary, not verbatim)
- No LICENSE file was requested or found in the root listing; the root listing did not include one.
