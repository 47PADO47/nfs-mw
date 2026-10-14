# NFSMW_PhotoMode (Hydra455)

Native photo mode for Need for Speed: Most Wanted (2005, PC), described in the repo as "Native Photomode like in NFS Carbon". It is a Win32 `.asi` plugin that hooks the game by hand-written byte patches. It adds a "Photomode" entry to the pause menu, freezes the world, hides the HUD, mutes the game's audio, and draws an on-screen button overlay over Direct3D 9 Present. It also replaces the gameplay camera's LookAt with an orbit camera that the player controls.

Licence status: CONFLICTING. The repo has a GPL-3.0 `LICENSE` file, but the README says "(C) All rights reserved" and "You are not allowed to reupload this elsewhere without my approval". Until the author clarifies, treat the code as not licensed for reuse. Our rewrite must use behaviour and facts only, not code.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/Hydra455/NFSMW_PhotoMode |
| Author | Hydra455 (GitHub user; real name not stated) |
| Type | ASI plugin (DLL renamed to `.asi`), Win32 |
| Licence | Conflicting: `LICENSE` is GPL-3.0 (full text, FSF copyright line); README says "(C) All rights reserved" and forbids reupload without approval |
| Language | C++ (Visual Studio solution, `v145` toolset) |
| Target game build | Need for Speed: Most Wanted (2005, PC). The exact executable version is not stated; the hard-coded addresses are for one build (see section 3) |
| Latest visible activity | Commit "Updated" on Oct 10, 2026 (a92e681). 5 commits in total: "Initial commit" (Oct 4, 2026), "Update README with copyright and usage terms" (Oct 4), "by Hydra" (Oct 5), "Update NFSMW_PhotoMode.cpp" (Oct 5), "Updated" (Oct 10) |
| Size | One source file, `NFSMW_PhotoMode.cpp`; the exact line count was not verified |

## 1. What it does

Each feature below is backed by the source file unless marked README.

- **Photo mode toggle.** `F8` enters and leaves photo mode. [source: NFSMW_PhotoMode.cpp, hotkeys table]
- **Pause menu entry.** A "Photomode" option is added to the pause menu, with the label string "Photomode" and a hashed icon. Selecting it starts photo mode and unpauses the game. [source: `AddPhotoModePauseOption`, `kPhotoModeLabelHash`, `kPhotoModeIconHash`]
- **Orbit camera.** While active, the camera orbits the player's car. Left/Right change yaw, Up/Down change pitch, `9`/`NUMPAD9` zoom out and `0`/`NUMPAD0` zoom in. Pitch is limited to about 4.5 to 20 degrees and distance to 4.65 to 6.65 units. [source: key handling, clamp constants]
- **World freeze.** Game speed is set to zero, so the world stops while the camera moves. The saved speed is restored on exit. [source: `kTweakGameSpeed`, world freeze description]
- **HUD hide.** The HUD draw instruction is disabled while photo mode is active. [source: `HideGameplayHud`, `EnforceGameplayHudHidden`]
- **Audio mute.** Audio sessions belonging to the game's process are muted while photo mode is active, and their previous mute states are restored on exit. [source: `MuteCurrentProcessAudioSessions`, `RestoreCurrentProcessAudioSessions`]
- **On-screen overlay.** Labels "BACK", "ZOOM" and "VIEW CAR" are drawn with a built-in 5x7 bitmap font. A gradient quad and key boxes show the controls. [source: D3D9 overlay functions]
- **Option to hide buttons.** The overlay can be turned off in the INI file (section 4). [source: `LoadPhotoModeConfig`]
- **Exit with ESC.** `ESC` leaves photo mode only while it is active. [source: hotkeys table]
- **Error message.** If the LookAt hook cannot be installed, a message box says "This script couldn't be loaded." and suggests a conflicting camera mod. [source: install failure path]

## 2. How it does it

Tags: [source] verified in the code; [README] stated only in the README; [inferred] my reading, not confirmed by the code.

### Loading

- `DllMain` on attach calls `DisableThreadLibraryCalls`, `LoadPhotoModeConfig`, then `Install`. On detach it restores the state and each hook in turn. [source]
- The project builds a DynamicLibrary named `NFSMW_PhotoMode.asi` (Debug and Release, Win32, toolset v145). Link libraries: `d3d9.lib`, `ole32.lib`, `user32.lib`, `kernel32.lib`. [source: `.vcxproj`]
- The code exports nothing and has no ASI loader code. The DLL is therefore loaded by an external `.asi` loader. Which loader is not stated. [source for "no loader code"; loader identity is [inferred], probably a standard ASI loader]

### Memory patching

All hooks are hand-written byte patches. No MinHook or similar library is used. [source]

- `WriteProtected`, `IsReadableRange`, `IsExecutableAddress` check memory before writing. [source]
- `ResolveRelativeCallTarget` and `PatchRelativeCall` rewrite `E8` relative calls. [source]
- A background thread, `OverlayHookThread`, re-checks the overlay hook every 500 ms after it is installed, or every 100 ms before that. [source]

### Hooks (addresses are for one game build)

| Hook | Address | What it does | Source |
|---|---|---|---|
| LookAt call site | `0x0047DCBC` | Rewrites the `E8` call to `NFSMW_PhotoMode_LookAtHook`. The original function is at `0x006CF0A0`; its prologue `55 8B EC 83 E4 F0` is checked before chaining. Replaces eye position with the orbit position while active. | [source] |
| Localization lookup | `0x0056BB80` | Inline hook: 5-byte `E9` jump to `NFSMW_PhotoMode_LocalizationHook`, with a 16-byte trampoline from `VirtualAlloc`. Expected prologue `53 8B 1D 80 CF 91 00`. | [source] |
| Pause populate A | `0x005463A6` | Redirects the call to `NFSMW_PhotoMode_PausePopulateHookA`. Calls the original populate, then `AddPhotoModePauseOption`. | [source] |
| Pause populate B | `0x005463AD` | Same, for `NFSMW_PhotoMode_PausePopulateHookB`. | [source] |
| D3D9 Present | vtable index 17 | Replaces `&vtbl[17]` with `NFSMW_PhotoMode_PresentHook`. Draws between `BeginScene` and `EndScene`, then calls the chained Present. | [source] |
| HUD draw | `0x0057CAA7` / `0x0057CAA8` | Verifies the expected instruction bytes at `0x0057CAA7`, then writes `0` to `0x0057CAA8` to hide the HUD. Rewritten each frame. | [source] |
| Game speed | `0x00901B1C` | Writes `0.0f` after a 100 ms delay (`kHudPropagationDelayMs`). | [source] |

Other addresses used by the code (names from source):

- `kD3DDevicePtr` at `0x00982BDC`, global pointer to the D3D9 device. [source]
- `kPauseMenuVtable` at `0x0089E5A0` (expected), `kPauseOptionRetailVtable` at `0x0089C25C` (copied, 12 entries, slot 1 replaced by `NFSMW_PhotoMode_OptionReact`). [source]
- `kJMalloc` at `0x00652AD0`, game allocator. The new pause option is 0x4C bytes. [source]
- `kPauseOptionCtor` at `0x0051D790`, `kIconScrollerAddOption` at `0x00573960`, `kUnPause` at `0x00632190`. [source]
- `kAcceptEventHash` `0x0C407210`. When this event arrives, `NFSMW_PhotoMode_OptionReact` calls `kUnPause`. [source]

### Audio

- Uses `IMMDeviceEnumerator` and `IAudioSessionManager2` to find sessions owned by the current process ID, then calls `SetMute(TRUE)`. [source]

### Rendering

- Overlay drawn with `DrawPrimitiveUP`, saving and restoring the D3D9 state block. [source]

### Known weaknesses

- Hard-coded addresses and byte checks. Any other build of the game will fail or patch the wrong code. [inferred from the design; the check on expected bytes gives a clean failure in some cases]
- Several hooks write to code and data without any sync, so they depend on timing. [inferred]

## 3. Configuration

File: `NFSMW_PhotoMode.ini`, in the same folder as the DLL. [source for name and location]

Full file content, as fetched:

```ini
[Overlay]
ShowButtons = false
```

| Section | Key | Default in code | Values | Effect |
|---|---|---|---|---|
| `[Overlay]` | `ShowButtons` | `"true"` | `false`, `0`, `no`, `off` (case-insensitive) disable; anything else enables | Shows or hides the on-screen button overlay |

Note: the repo's INI file sets `ShowButtons = false`, which differs from the code default of `true`. [source]

No other keys were found.

## 4. How we could implement it in nfs-mw

Rules: this repo has conflicting licence statements, so we do not copy its code. We describe only behaviour and facts, and specify them from observation of the game. Spec first.

### Does the approach apply to a clean-room rewrite?

- Byte patches at fixed addresses, `E8` call rewrites, vtable swaps, and the ASI loader are specific to the original binary and to a hooking approach. A Rust rewrite should not patch the original executable at all. These parts do not apply.
- The features do apply: a photo mode with orbit camera, world freeze, HUD hide, audio mute, pause menu entry, and an overlay.

### Behaviours to specify

1. Enter and exit photo mode from gameplay (keyboard trigger, default `F8`, and `ESC` to exit).
2. Pause menu option "Photomode" that enters photo mode.
3. Orbit camera around the player's car with yaw, pitch and distance limits, and input mapping as in section 1.
4. World time freeze on entry and restore on exit.
5. HUD hidden on entry and restored on exit.
6. Game audio muted on entry and restored on exit. Only the game's own audio, not system audio.
7. On-screen control labels, optional, toggled by config.
8. Config file: one `[Overlay]` section with `ShowButtons`. Defaults to be decided by us.
9. Failure behaviour: if a required hook or data is missing, show a clear error rather than crash.

### Data and formats needed

- The game's camera and player-vehicle state (position, orientation). Source of these values must come from our own reverse engineering or from the engine's data, documented in our specs.
- The game's speed or time-scale variable and the HUD toggle, once we have confirmed them in our own work.
- Input bindings for keys, from the config.
- Config format: INI is fine, matching the original. We may choose a different format.

### Natural home in our workspace

- Engine-generic parts go in `libs/`: the orbit camera math (yaw, pitch, distance clamps and easing), the config reader, input-mapping logic, and a generic overlay/text renderer that does not depend on NFS:MW.
- NFS:MW-specific parts go in `crates/`: the photo mode feature, the pause-menu entry, the NFS:MW camera and HUD addresses or layout tables, and the mapping from game state to our engine.
- `crates/` depends on `libs/`, never the reverse. No NFS:MW names or defaults in `libs/`.
- Any renderer backend for the overlay follows ADR 0004 (swappable renderers).
- Source for the layout and addresses comes from our own format docs and tools in `docs/`, not from this repo.

### Risks

- Licence: the conflict must be resolved or avoided by writing our own spec and code. Do not copy the source, the address tables or the byte patches from this repo as-is. Facts about the game (values, names of functions we confirm ourselves) are fine.
- Game-build dependence: any address we use must come from our own verification against the install, not from this repo.
- Behaviour that depends on the game's timing (world freeze delay of 100 ms, the audio-mute path) must be re-derived and tested.
- Anti-cheat or sandboxing is not relevant to offline play, but we should not make the game's own memory tampering a dependency of the rewrite.
- Audio: session-level mute depends on Windows audio APIs. We should check the exact behaviour on our target OS.

## 5. Open questions and what could not be verified

- Licence: the repo contains both GPL-3.0 (`LICENSE`) and "All rights reserved" (README). Which one the author intends is unknown. We should ask the author before treating either as the grant.
- Which external `.asi` loader is used is not stated. [inferred]
- The exact game build (executable version or hash) that the addresses target is not stated. The addresses listed above are the only evidence.
- Whether the code works on the current game build was not verified. We did not run anything.
- The README is short: it contains only the title, one-line description, and the copyright and usage notice. There is no feature list, install guide, or screenshot. [README]
- The repo root shows `NFSMW_PhotoMode.ini`, `NFSMW_PhotoMode.sln`, `README.md`, `LICENSE`, and the folder `NFSMW_PhotoMode/` (containing `NFSMW_PhotoMode.cpp`, `.vcxproj`, `.vcxproj.filters`, `.vcxproj.user`). The folder listing says "View all files", so there may be more files that were not shown. Only the `.cpp` was read in full, through a summary.
- The `.cpp` was read through a summarising fetch, not as raw text. Addresses, names and key bindings are quoted from that summary and should be re-checked against the raw file before any use.
- The commit list shows dates but no per-commit diffs. We did not check what changed between the initial commit and the latest one.
- No release binaries were checked, and none were downloaded or run, as instructed.

## 6. Sources

- https://github.com/Hydra455/NFSMW_PhotoMode (repo page, description, root listing, licence sidebar)
- https://raw.githubusercontent.com/Hydra455/NFSMW_PhotoMode/main/README.md (README, read through a summarising fetch)
- https://raw.githubusercontent.com/Hydra455/NFSMW_PhotoMode/main/LICENSE (GPL-3.0 text, first lines)
- https://raw.githubusercontent.com/Hydra455/NFSMW_PhotoMode/main/NFSMW_PhotoMode.ini (config, `ShowButtons = false`)
- https://raw.githubusercontent.com/Hydra455/NFSMW_PhotoMode/main/NFSMW_PhotoMode.sln (solution: Debug/Release Win32)
- https://github.com/Hydra455/NFSMW_PhotoMode/tree/main/NFSMW_PhotoMode (folder listing)
- https://github.com/Hydra455/NFSMW_PhotoMode/commits/main (commit list and dates)
- https://raw.githubusercontent.com/Hydra455/NFSMW_PhotoMode/main/NFSMW_PhotoMode/NFSMW_PhotoMode.cpp (source, read through a summarising fetch)
- https://raw.githubusercontent.com/Hydra455/NFSMW_PhotoMode/main/NFSMW_PhotoMode/NFSMW_PhotoMode.vcxproj (build settings)
