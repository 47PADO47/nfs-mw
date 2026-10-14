# NFSMWDaylightSavingTime (DST)

Third-party `.asi` mod for Need for Speed: Most Wanted (2005, PC). It stops the in-game
time of day from advancing while the game is paused, and makes it scale with the
Speedbreaker time dilation. The repository contains a small C++ DLL that patches one
instruction sequence in `speed.exe` at runtime. It is a reference for how the original
game drives its clock, not a feature we need to copy.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/rng-guy/NFSMWDaylightSavingTime |
| Author | Not named on the pages read. Owner is `rng-guy`. [README] |
| Type | `.asi` plugin (runtime code patch loaded by an ASI loader) |
| Licence | CC0-1.0 (Creative Commons Zero, public domain dedication). Stated in `LICENSE.txt` and on the repo page. [source] |
| Language | C++ (MSVC, C++20, 32-bit Windows). Visual Studio project files present. [source] |
| Target build | `speed.exe` of 6,029,312 bytes (about 5.75 MB), entry point `0x3C4040`. [README] [source] |
| Latest visible activity | Not shown on the repo page. "20 Commits" is listed, no date. Changelog ends at v1.00.00 with revisions 01 to 04. [README] |
| Commit seen | `644f8384f494944713b4065695195fe87e625267` (short `644f838`) on the Source folder view. [source listing] |

## 2. What it does

Each feature is listed with its evidence.

- **Time of day freezes while paused.** When the game is paused, the clock does not
  advance. [README] [source: `GetTimeScale` returns `0.f` when `IsPaused()` is true, in `Source/dllmain.cpp`]
- **Time of day follows Speedbreaker time dilation.** Normal time progression is multiplied
  by the simulation time scale, so slow-motion affects the clock too. [README]
  [source: `GetTimeScale` reads the float at `simSystem + 0x24`]
- **Shadows do not move in pause menus or photo finishes.** Stated as a side effect of the
  time freeze. [README] (Not separately verified in code.)
- **Installs without a rebuild.** Users copy a `scripts` folder into the game folder. [README]

## 3. How it does it

Mechanism, as read from `Source/dllmain.cpp`. Claims marked [source] were read in the code.
Claims marked [README] are stated only in the README. Claims marked [inferred] are my reading.

- **Loader.** The DLL is loaded by an ASI loader (the README suggests ThirteenAG's
  WideScreenFix as a host mod). The DLL's `DllMain` runs at `DLL_PROCESS_ATTACH`. [README] [source]
- **Compatibility gate.** `DllMain` compares `MemoryTools::GetEntryPoint()` against `0x3C4040`.
  On mismatch it shows the message "This .exe isn't compatible with DST." and returns `FALSE`.
  The patch is applied only after this check passes. [source]
- **Compile-time checks.** The source refuses to compile unless MSVC `_MSC_VER >= 1930`
  (Visual Studio 2022), 32-bit Windows, and C++20. [source]
- **Helper functions.** `Source/Headers/MemoryTools.hpp` provides `MemoryTools::address`,
  `AsReference`, `AsFunction`, `GetEntryPoint`, and `PATCH_ASSEMBLY_DETOUR` /
  `EXIT_ASSEMBLY_DETOUR` macros. I did not read this header, so its internals are [inferred]
  from the macro names.
- **Game function called: `IsPaused`.** Address `0x468390`, signature `bool __cdecl()`. [source]
- **Game global read: sim system pointer.** Address `0x9885E0`. The DLL reads the pointer
  there; if it is null, the time scale is `1.0f`. [source]
- **Game field read: time scale.** Float at offset `0x24` in the sim system object. [source]
- **Detour site: time-of-day update.** Bytes `0x7693AF` to `0x7693B6` of `speed.exe`.
  The detour (`TimeOfDayUpdate`, inline assembly) does the following. [source]
  1. Calls `GetTimeScale()`.
  2. Multiplies the returned float into the x87 stack (`fmulp st(1), st(0)`).
  3. Reloads `ecx` from `[esi+0x4]`.
  4. Adds `[esi+0x8]`.
  5. Stores the result to `[esp+0x18]`.
  6. Jumps back through `EXIT_ASSEMBLY_DETOUR`.
- **Net effect.** The time-of-day increment is scaled by the sim time scale, and forced to
  zero while paused. The exact arithmetic of the original code path is [inferred] from the
  multiply and add in the detour.
- **Config files and registry.** None read. [source]
- **Hook style.** Runtime code patch in process memory, with no DirectX or D3D hooks. [source]

## 4. Configuration

None. The repository reads no config file, environment variable, or registry key [source].
The README lists no configuration keys or defaults [README].

## 5. How we could implement it in nfs-mw

The DLL is a loader hack, so the mechanism does not carry over to a clean-room Rust rewrite.
The behaviour does. Per the repo licence (CC0-1.0) the code could legally be read, but our
rule is spec-first: describe behaviour and facts, and do not copy the code.

**Does not apply.** The ASI loader, `speed.exe` address patches, the x87 detour, the entry
point gate, and the MSVC checks are specific to the original binary. Our rewrite has no
patched executable and must not use those addresses.

**Behaviours to specify.**
- The time-of-day clock advances each simulation step by `base_increment * time_scale`.
- When the simulation is paused, `time_scale` is `0`, so the clock does not advance.
- `time_scale` comes from the Speedbreaker time-dilation state. When no dilation is active,
  it is `1.0`.
- Shadows and other pause-driven visuals are not updated during pause. This is a
  consequence of the paused state, to be confirmed against our render path.

**Data or formats needed.**
- A documented field layout for the time-of-day value and the sim time scale, from the
  game's own reading in `docs/` (format notes), not from this repo's offsets.
- Pause state and Speedbreaker state as explicit simulation inputs.
- Test vectors: a time-of-day value with pause on, pause off, and dilation at a known factor.

**Natural home.**
- The clock stepping and time-scale rule are NFS:MW-specific game logic, so they belong in
  `crates/` (for example a gameplay or simulation crate).
- Any engine-generic timing helper (fixed step, scaled delta) belongs in `libs/`.
- `crates/` depends on `libs/`, never the reverse. Do not put NFS:MW names or addresses in
  `libs/`.
- Use early returns, no `else`, per `AGENTS.md`.

**Risks.**
- Our pause and time-scale semantics must match the original game. The repo does not
  document the frame timing, so this needs our own captures from the install.
- The detour approach must not be copied. Any test that reads `speed.exe` must follow the
  game-data rules in `docs/` and must not add install data to the repo.
- A zero time scale during pause must not freeze unrelated timers (UI animation, input).
  This needs its own spec.

## 6. Open questions and what could not be verified

- Latest commit date and branch activity. Not visible on the repo page through the fetch tool.
- Author name. Not stated in the README or the pages read; only the owner handle `rng-guy`.
- `Source/Headers/MemoryTools.hpp` was listed but not read. Its internals are unverified.
- `Source/cpp.hint` and the Visual Studio project files were not read.
- The README mentions a changelog attribution notice that was not included in the fetched text.
- Whether the shadow behaviour is caused by the time freeze or by a separate patch: not
  verified.
- Behaviour on other `speed.exe` builds: the code gates on one entry point, so other builds
  are refused. Other patches are not verified.
- The exact original code path around `0x7693AF`: only the detour bytes were read, not the
  full disassembly.
- No code was run and no release binaries were downloaded, per the task rules.

## 7. Sources

- https://github.com/rng-guy/NFSMWDaylightSavingTime (repo page, read via WebFetch)
- https://raw.githubusercontent.com/rng-guy/NFSMWDaylightSavingTime/HEAD/README.md
- https://raw.githubusercontent.com/rng-guy/NFSMWDaylightSavingTime/HEAD/README.txt
- https://raw.githubusercontent.com/rng-guy/NFSMWDaylightSavingTime/HEAD/LICENSE.txt
- https://raw.githubusercontent.com/rng-guy/NFSMWDaylightSavingTime/HEAD/Source/dllmain.cpp
- https://github.com/rng-guy/NFSMWDaylightSavingTime/tree/HEAD/Source (folder listing)
- https://github.com/rng-guy/NFSMWDaylightSavingTime/tree/HEAD/Source/Headers (folder listing)
