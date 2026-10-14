# NFS-XtendedInput

NFS-XtendedInput is an ASI plugin for the Black Box era Need for Speed games (Most Wanted 2005, Carbon, ProStreet, Undercover, World). It replaces the game's input layer with a Win32/XInput based one. It adds controller and keyboard handling, button icon swapping, mouse input, and rebindable actions. The repository states it is undergoing a major rewrite and that current issues will not be addressed in it.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/xan1242/NFS-XtendedInput |
| Author | xan1242 (repository owner). Credits in the README: AeroWidescreen (button icons), LINK/2012 (injector) |
| Copyright lines | Copyright (c) 2022-2023 Lovro Plese; Copyright (c) 2023 Berkay Yigit [source: LICENSE] |
| Type | ASI plugin (C++ source, built against an ASI loader) |
| Licence | MIT License [source: LICENSE] |
| Language | C++ (Visual Studio solution and vcxproj in the repo, headers in `src/`) [source: file listing] |
| Target builds | NFS Most Wanted (2005), Carbon, ProStreet, Undercover, World (alpha). Underground and Underground 2 are covered by a separate repository, NFSU-XtendedInput [README] |
| Default branch | master [source: file listing] |
| Latest activity | Not visible on the fetched pages. The repo page showed 209 commits, 212 stars, 17 forks, 60 open issues, but no commit date |
| Status | "Currently undergoing a major rewrite"; existing issues "will not be addressed" [README] |

## What it does

Each feature is listed with its evidence. The README is the only source for the feature list; the code confirms the hooks that implement them (see section 3).

- **Button icon swapping by input device.** Button prompts switch between keyboard and controller icons depending on which device was last used. Icon sets exist for Xbox, PlayStation and Switch controllers [README].
- **Console control parity.** The controller layout follows the Xbox 360 control scheme. Example defaults: throttle on the right trigger, brake on the left trigger, steering on the left stick X axis [README, default mapping].
- **Debug camera analog control.** Analog input drives the debug world camera [README]. Code: `DebugWorldCameraMover_Update` and its constructor are hooked [source: src/Main.hpp].
- **Keyboard via Win32.** Keyboard state is read with `GetAsyncKeyState` or `GetKeyboardState`, not DirectInput 8 [README].
- **Mouse input.** Mouse input is read through Win32. The cursor hides after 5 seconds of inactivity. An optional mode renders the OS cursor [README]. Code: front-end cursor position update is redirected to `UpdateFECursorPos` [source: src/Main.hpp].
- **Mouse look for the debug camera** [README].
- **Rebindable actions.** All game actions can be remapped per save profile [README]. Code: a remapping mode hook and a reset-to-defaults hook exist [source: src/Main.hpp].
- **Rumble is not supported** [README].
- **Windows 7 support.** A separate "Win7" build is provided for Windows 7, and possibly Vista and XP [README].

## How it does it

Tags: [source] = verified in code, [README] = stated only there, [inferred] = my reading.

### Loading

- Loaded as an ASI plugin. The release bundles a special ASI loader built without CPU extensions and patched to run on Windows XP [README].
- Requirements listed by the README: an Xbox 360 controller driver, the DirectX runtime (XInput 9.1.0), and the Visual C++ Redistributable 2019 [README].

### Hooks and patches (in `Init()`, `src/Main.hpp`) [source]

The plugin writes code and data directly into the game image. The calls below are from `Init()`:

- Replaces the input device factory pointer (`INPUTDEVICE_FACTORY_INITIALIZER_ADDR`) with `InputDeviceFactory`.
- Redirects a call in the input mapping constructor (`INPUTMAPPING_CONSTRUCTOR_CALL_ADDR`) to `InputMapping_Constructor`.
- NOPs five bytes at `DINPUT_KILL_ADDR`. This disables the DirectInput path. The repository TODO says the plugin "KILLS Direct Input" [README, TODO list].
- Replaces the window procedure pointer (`WNDPROC_POINTER_ADDR`) with `CustomWndProc`.
- Replaces a call at `FEMOUSE_UPDATE_CALL_ADDR` with a jump to `UpdateFECursorPos`.
- Replaces the debug world camera vtable entry (`DEBUGWORLDCAMERAMOVER_UPDATE_VT_ADDR`) and hooks its constructor.
- Writes `ResetMappingsToDefault` into a function pointer at `0x00991AFC`.
- Hooks a call at `0x007028AA` (`GetRemappingModeHook`).
- Hooks the memory card load call at `0x00563F33` (`MemoryCard_Load_Hook`) to read the profile name, which selects the per-save mapping folder.
- The front-end cursor and tuning-screen code also have hooks, and World has a patch at `0x0075E358` (`UnscrewWorldDeviceScalars`).

### Addresses per build [source]

The file defines hard-coded addresses per game build:

- Most Wanted: memory card load function at `0x005189F0`; a controls entry jump at `0x0052905D`; a tuning screen hook write at `0x008A2A78`.
- Carbon: load-done function at `0x0058E250`; a front-end find-object call hook at `0x005C3916`.
- ProStreet: load-done function at `0x00543CC0`.
- World: relative offsets from `MainBase` (for example `0x75E375`).

Some names used in the file (for example `UTL_ILIST_CONSTRUCTOR_ADDR`, `GAMEFLOWMANAGER_STATUS_ADDR`, `GAME_HWND_ADDR`) are defined in headers that were not fetched, so their values are not verified here.

### Functions (signatures as written in `src/Main.hpp`) [source]

- `HRESULT UpdateControllerState()`
- `void __stdcall ReadControllerData()`
- `bool bIsHudVisible()`
- `LRESULT WINAPI CustomWndProc(HWND, UINT, WPARAM, LPARAM)`
- `void __stdcall MemoryCard_Load_Hook(const char* profileName)`
- `void __stdcall FEPhotoModeStateManager_HandleScreenTick_Hook()`
- `void(__thiscall* DebugWorldCameraMover_Update)(void*, float)`
- `void*(__thiscall* DebugWorldCameraMover_Constructor)(void*, void*, void*, int, int)`

### Game-side hooks in the README

- A window procedure hook [README, src/Main.hpp].
- `InputDevice::PollDevice()` runs right before `eDisplayFrame`. The plugin hooks this point for per-frame polling [README].
- Front-end functions used for icons: `FEngSetVisible`, `FEObjectCallback_Function`, `SetUnbindableButtonTextures` (called but defined elsewhere, not verified) [source].

### Configuration files (read at runtime)

- `scripts/NFS_XtendedInput.ini`: general settings (deadzones, omni mode, etc.) [README]. The page does not list the keys.
- `scripts/NFS_XtendedInput.default.ini`: default mappings for new profiles and resets [README].
- `scripts/XtendedInputMaps/<save name>/NFS_XtendedInput.usermap.ini`: per-profile remaps. Secondary maps use `[Events_Secondary]` and `[EventsKB_Secondary]` [README].

### Known limits

- MW's Controls settings menu is disabled because it crashes the game [README].
- World support is alpha [README].

## 4. Configuration

The keys below come from the README. Defaults are not listed on the pages fetched, and the `.ini` files were not fetched.

| File | Purpose | Keys and defaults |
|---|---|---|
| `scripts/NFS_XtendedInput.ini` | General settings | Deadzones, omni mode, and "other settings". Keys and defaults: not verified |
| `scripts/NFS_XtendedInput.default.ini` | Default mapping for new and reset profiles | Xbox 360 layout per README. Full key list: not verified |
| `scripts/XtendedInputMaps/<save>/NFS_XtendedInput.usermap.ini` | Per-profile remap | Sections `[Events_Secondary]`, `[EventsKB_Secondary]` [README]. Other keys: not verified |

## 5. How we could implement it in nfs-mw

### Relevance to a clean-room rewrite

Most of this project is specific to hooking the original game. It does not apply to our Rust rewrite as-is:

- The ASI loader, DLL injection, and the hard-coded game addresses are loader-specific hacks. A rewrite of the game does not need them. Our rewrite reads its own data from the install (`game-install` is the only crate that touches the install, per AGENTS.md).
- The DirectInput kill patch, the window procedure hook, and the per-frame poll hook are all ways of patching the original binary. Not applicable.

What does transfer is the behaviour and the data model:

- Device-aware button icon selection (keyboard vs controller, icon set per controller family).
- An Xbox-style default action map with per-profile overrides and a reset-to-defaults action.
- Keyboard and mouse input read through the platform input layer, with cursor auto-hide and an optional OS cursor.
- Debug camera analog and mouse look.

### Behaviours to specify (spec-first, no code copied)

1. Input model: the game's logical actions (throttle, brake, steer, camera, menu navigation, etc.) mapped from physical inputs (keyboard keys, mouse buttons and axes, gamepad buttons and axes).
2. Device switching: track the last-used input device; choose the matching icon set for prompts. Define the switch rule exactly (on any event from a device, or only on a button press).
3. Deadzones and axis shaping for analog sticks and triggers.
4. Mouse cursor: auto-hide after a fixed idle time (5 s in the reference behaviour), show again on movement.
5. Profiles: one remap file per save profile, falling back to a defaults file. Define what happens for a missing or malformed file.
6. Secondary bindings: a second map layer for alternate keyboard or gamepad bindings.
7. Remap UI: a mode where the next input is captured as a binding.
8. Rumble: out of scope in the reference; decide for our rewrite.

### Data and formats needed

- An action table: logical action ID, name, default keyboard and gamepad bindings.
- An INI-like format (or TOML, if we choose) for the defaults file and the per-profile remap file. Specify the section and key names ourselves; do not copy the reference names unless we decide to keep them for compatibility.
- Icon metadata: which glyph set applies to which controller family.

### Natural home

- Generic input abstraction (action map, device model, deadzones, binding file parsing): `libs/`. It must not mention NFS:MW names or paths. Per AGENTS.md, engine-generic code goes in `libs/`.
- NFS:MW-specific action list, default bindings, and icon mapping for the game's prompts: `crates/`. Crates depend on libs, never the reverse.
- Any Bevy-based input or UI backend: an optional leaf crate in `libs/` only if needed (ADR 0004).
- Format parsing that takes `&[u8]` belongs in a format crate; the file path handling belongs to the install or config layer, not the format crate.

### Risks

- Input latency and polling order differ between a rewritten engine and the original; verify with recorded input.
- Profile file corruption or version drift. Define a migration or a fallback.
- Keyboard-only mouse-look or camera controls need a clear spec so camera behaviour matches expectations.
- The reference project is mid-rewrite and says current issues will not be addressed, so its behaviour may change. Spec from the code we verified, not from its status.
- Licence: the repository is MIT. Our `docs/licensing.md` governs what can be ported and the spec-first process. This page did not read that file, so the decision on copying is left to it. The safe default is to specify behaviour from our own observation and not copy code.

## 6. Open questions and what could not be verified

- Full list of config keys and their defaults in `NFS_XtendedInput.ini` and `NFS_XtendedInput.usermap.ini`. Not fetched.
- The source files `NFSW_XtendedInput.h`, `NFS_XtendedInput.h`, `NFS_XtendedInputButtons.h`, `NFS_XtendedInput_XInputConfig.h`, and the `inc/`, `_win32/` and `_vs/` folders were not fetched. Only `src/Main.hpp` was read.
- The repository file listing is incomplete ("View all files" link was not followed). The `data/` folder contents were not checked.
- Exact game executable versions the addresses target. The file lists addresses, but their build IDs were not confirmed.
- Latest commit date and activity: not visible on the fetched pages.
- Whether the Win7 build differs in code or only packaging: not verified.
- Keyboard input on Windows 7 and older systems: not verified.
- Whether the "rewrite" mentioned in the README has a public branch: not checked.
- No release binaries were downloaded or run, per the task rules.

## 7. Sources

- https://github.com/xan1242/NFS-XtendedInput (repository page, file listing, metadata)
- https://github.com/xan1242/NFS-XtendedInput/tree/master/src (src folder listing)
- https://raw.githubusercontent.com/xan1242/NFS-XtendedInput/master/README.md (README)
- https://raw.githubusercontent.com/xan1242/NFS-XtendedInput/master/LICENSE (licence)
- https://raw.githubusercontent.com/xan1242/NFS-XtendedInput/master/src/Main.hpp (hooks, addresses, function signatures)
