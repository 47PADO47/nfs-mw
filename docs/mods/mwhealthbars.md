# MWHealthbars

Status: documented from the public repository only. Nothing was cloned, downloaded or run.

MWHealthbars is a small native mod for Need for Speed: Most Wanted (2005, PC). It draws a health bar above each police car during a pursuit so the player can see how close a cop is to being destroyed. The README says it does not change gameplay; it only visualises damage values the game already tracks. The mod is a C++ DLL packaged as an `.asi` file for an ASI loader. It hooks the game's GUI draw call, reads game state from fixed memory addresses, and renders lines and rectangles through the game's Direct3D 9 device.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/trelbutate/MWHealthbars (public) |
| Author | trelbutate (repository owner) |
| Type | ASI plugin (native DLL renamed to `.asi`) |
| Licence | None stated. No LICENSE file (HTTP 404 on raw.githubusercontent.com/.../HEAD/LICENSE) and no licence section in the README. Treat as all rights reserved. |
| Language | C++ (Visual Studio project `MWHealthbars.vcxproj`). The GitHub language bar was empty in the page view. |
| Target game build | Need for Speed: Most Wanted, game version 1.3 [README] |
| Latest visible activity | 25 Jan 2026, "Make compatible with other mods that replace the GUI call" [commit list] |
| Size | 7 commits; 6 stars, 2 watching, 0 forks [repo page] |
| Releases | Pre-built `healthbars.asi` is offered on the releases page per the README. Not downloaded; not inspected. |

## 2. What it does

Each feature below is backed by the README or by the source summary.

- **Health bars above cop cars.** Bars are drawn over each pursuing police car, positioned in world space above the car. [README] [src/healthbars.cpp]
- **Colour by health tier.** Bar colour steps through `green`, `lightGreen`, `yellow`, `orange` and `red` as health falls. [src/healthbars.cpp, constants]
- **Lagging "animated" health.** A second, slower-moving value shows recent damage; it drains at a fixed rate (0.75 per second) after a short countdown (0.5). [src/healthbars.cpp]
- **Death effect.** Destroyed cars get a flashing bar with a fade-out after about 1 second. The commit "Fix death pulse animation" touched this. [src/healthbars.cpp; commit list]
- **Disabled-car handling.** A car that has been disabled is hidden after 2.0 seconds of disabled time. [src/healthbars.cpp]
- **Distance fade.** Bars fade in and out by camera distance: near fade between 2 and 5 units, far fade between 70 and 110 units. [src/healthbars.cpp]
- **Back-to-front sorting.** Bars are sorted by view-space depth so nearer bars draw on top. [src/healthbars.h, operator<]
- **Hidden when not relevant.** Bars are suppressed when there is no active pursuit, when the game is paused (`hideOnPause`, default false), and during the cop freeze cam. [src/healthbars.cpp, Draw()]
- **Helicopters excluded.** Entries with the chopper vehicle class are skipped. [src/healthbars.cpp, UpdateCopCars()]
- **Menu fix.** "Fix healthbar sometimes showing up in the main menu" (27 Jul 2024). [commit list]
- **Compatibility with other GUI-replacing mods.** The 25 Jan 2026 commit changes how the draw hook chains to the original call. [commit list; src/dllmain.cpp]

Not visible from the sources read: whether bars appear for anything other than cop cars (the code filters to the cop car table), and any options menu. There is no in-game UI in the source summary.

## 3. How it does it

### 3.1 Loading

- The ASI loader loads the DLL. `DllMain` calls `HookFunction()` on `DLL_PROCESS_ATTACH` and does nothing for other reasons. [src/dllmain.cpp]
- There is no ASI-loader export and no `.asi`-specific code. The source is a plain DLL; the `.asi` extension is a packaging choice. [README] [src/dllmain.cpp] [inferred from the lack of exports]
- Loader requirement: an ASI loader such as Widescreen Fix, NFSMW Extra Options or Ultimate ASI Loader. [README]

### 3.2 Hook on the GUI draw call

- The hook site is the address `0x6e75a7`, where a 5-byte call instruction is located. [src/dllmain.cpp]
- `HookFunction()` makes four bytes at that address writable with `VirtualProtect`, reads the existing relative call offset, resolves it to the original draw function, saves that pointer as `realDrawGUIFunc`, writes a new relative offset that points at `DrawGUIHook`, and restores protection. [src/dllmain.cpp]
- So this is an inline patch of a call site, not a vtable or IAT hook. It rewrites four bytes of the game's code. [src/dllmain.cpp]
- `DrawGUIHook(bool)` calls `renderer.Draw()` when its argument is false, then calls the original function through `realDrawGUIFunc` if it is set. [src/dllmain.cpp]
- The code assumes a 32-bit build and an unchanged game binary at that address. [src/dllmain.cpp] [inferred that this is the reason for the compatibility commit: another mod that replaces the GUI call would clash with this patch.]

### 3.3 Game state read from fixed addresses

These are read directly from the game's memory. Each is a build-specific constant. [src/healthbars.cpp, as summarised]

| Address | Name used in source | Purpose (per source) |
|---|---|---|
| `0x92ce9c` | `copCarTable` | Table of cop car pointers |
| `0x92cea4` | `numCopCars` | Count for that table |
| `0x9259bc` | `deltaTime` | Frame delta |
| `0x9885d8` | `simTime` | Simulation time |
| `0x9885e0` | sim system pointer | Offset `0x2c` = state, `0x24` = time scale |
| `0x9842D0` | `viewMat` | Camera view matrix |
| `0x984350` | `viewProjMat` | View-projection matrix |
| `0x993cc8` | sound/AI pointer | Offset `0x130` = pursuit AI |
| `0x91cf90` | jump camera pointer | Chain via `0x10`; flag at `0x47` = freeze cam |
| `0x982bdc` | D3D9 device pointer | Read in the render code (see 3.4) |

Struct offsets used for each cop car: `0x6c` (vehicle class), `0x4c`, `0x30`, `0x0`, `0x3c` (destruction health), `0x80`, `0x10` (world position), `0x84` (car state). [src/healthbars.cpp]

Vehicle class identifiers are hash values: `CAR = 0x336fcacf`, `CHOPPER = 0xb80933aa`. [src/healthbars.h]

### 3.4 Rendering

- Uses the game's Direct3D 9 device, read from `0x982bdc`. Every public call returns early if the device is null. [src/primrender.cpp]
- `Begin()` saves all state with a state block, sets the flexible vertex format to XYZ plus diffuse, sets world and projection to identity and view to the caller's matrix, clears shaders, and sets render states: alpha blend on, Z-write off, Z-test on, culling off, scissor on; fog, lighting, specular, stencil and alpha test off. `End()` restores the state block. [src/primrender.cpp]
- Primitives: `DrawLine` (line strip, up to 64 points) and `DrawRect` (two triangles built from six vertices), both via `DrawPrimitiveUP`. [src/primrender.cpp]
- Math uses the D3DX helpers `D3DXMatrixInverse`, `D3DXMatrixMultiply` and `D3DXVec3Transform`. [src/healthbars.cpp]

### 3.5 Per-frame update

- `UpdateState()` reads the sim state and time scale, computes the simulation delta, loads the view matrix, builds the projection from the inverse view matrix times view-projection, detects pursuit and the freeze cam. [src/healthbars.cpp]
- `UpdateCopCars()` walks the cop car table, updates each car's health, animated health, world position and disabled timer, and removes map entries for cars no longer in the table. [src/healthbars.cpp]
- `Draw()` builds one `HealthBarDraw` per car (colour tier, death flash, distance fade), sorts them and draws each bar. [src/healthbars.cpp]

### 3.6 Known issues found in the source summary

- `DrawLine` does not check `count < 2`, so a count of 0 or 1 gives a zero or negative primitive count. [src/primrender.cpp]
- HRESULT errors are combined with `|=` and tested with `assert`, which can mask failures and is stripped in release builds. [src/primrender.cpp]
- The hard-coded device and game addresses break if the game binary differs. [src/primrender.cpp, src/dllmain.cpp]

## 4. Configuration

There is no config file read in the source. The only configuration is compile-time constants in `src/healthbars.cpp`:

| Key | Default | Note |
|---|---|---|
| `hideOnPause` | `false` | Hide bars while paused |
| `barHeight` | 0.13 | Bar size |
| `barWidth` | 1.4 | Bar size |
| `height` | 1.15 | Height above car |
| `colorAlpha` | 0.7 | Bar opacity |
| `nearFadeOutMin` / `nearFadeOutMax` | 2 / 5 | Near fade range |
| `farFadeOutMin` / `farFadeOutMax` | 70 / 110 | Far fade range |
| Colours | `green`, `lightGreen`, `yellow`, `orange`, `red`, `healthAnimationColor`, `borderColor`, `borderColorDestroyed` | Set in source |

Fixed timing values (not named): drain 0.75 per second, reduce countdown 0.5 s, disabled hide threshold 2.0 s, death fade starts at 1.0 s, flash period 0.05 s.

Changing any value means rebuilding. The README does not describe an ini file.

## 5. How we could implement it in nfs-mw

### 5.1 Licence and approach

The repository states no licence, so its code must not be copied, translated or closely paraphrased into nfs-mw. This page is for facts and behaviour only. Any implementation must be spec-first: write the behaviour spec from this page and from observation of the game, then implement against that spec. Do not use the addresses or code structure from this repo as a source.

### 5.2 Does the mechanism apply to a clean-room rewrite?

- **ASI loader, `VirtualProtect` call-site patch, and the D3D9 device pointer: not applicable.** nfs-mw is a rewrite that reads its own install and renders through its own backends (docs/architecture.md, ADR 0004). It does not inject into the original executable. Loader-specific hacks do not carry over.
- **Fixed-address reads: not applicable.** Our data path is the format crates and `game-install`, which take `&[u8]` and read the install. Game state should come from parsed data, not from a running process.
- **The feature itself (per-cop health bars with fade and sort): applies.** The behaviour is a clean target for a rewrite once our own game-state model exposes pursuit cars and their damage.

### 5.3 Behaviours to specify

1. Which cars get a bar (cop cars in pursuit; exclude helicopters) and how that set is determined.
2. Health tiers and their colour thresholds.
3. Animated (lagging) health: drain rate, countdown, and the rule for when it resets.
4. Death effect: flash rate, fade delay and fade duration.
5. Disabled-car hide rule and its time threshold.
6. Distance fade ranges and the fade curve.
7. Visibility rules: no pursuit, paused, freeze cam, menus.
8. Back-to-front ordering.
9. Bar size and offset above the car, and opacity.

Each needs a numbered entry in the spec with the observed game behaviour. Where a value is tuned by feel, record it as a default and mark it as a tuning value.

### 5.4 Data and formats needed

- Cop car state (position, health, vehicle class, disabled state) from the parsed game data, not from memory.
- Pursuit state: whether a pursuit is active, and whether the freeze cam is on.
- Camera view and projection matrices from our own camera model.
- Simulation time and time scale, so that the animation is paused with the game.

These will need format notes in docs/ first (see docs/README.md for the evidence-tag convention) before they go into code.

### 5.5 Natural home in nfs-mw

- **Generic maths and rules in `libs/`:** world-to-screen projection, the distance fade curve, health tier mapping, the lagging-value drain, and back-to-front sort. None of these is NFS:MW-specific, so they belong in `libs/` and must not name the game. [AGENTS.md]
- **Renderer-agnostic draw data in `libs/`:** a list of bar primitives (position, size, colours, opacity) that any backend can draw.
- **Drawing backend:** a Bevy-based or other renderer backend, as an optional leaf crate under `libs/` per ADR 0004 (`docs/decisions/0004-swappable-renderers.md`).
- **NFS:MW-specific logic in `crates/`:** the rule that selects cop cars in pursuit, the mapping from the game's vehicle classes to our parsed types, and the overlay that connects game state to the renderer. `crates/` depends on `libs/`, never the reverse. [AGENTS.md]
- Keep each file under 500 lines and split by concern (for example one struct per file). `cargo xtask size-check` enforces this. [AGENTS.md]

### 5.6 Risks

- **Behaviour is only partly known.** The source summary gives the rules, but the reference was not read verbatim. Verify each rule against the real game before relying on it.
- **Game data layout is not confirmed for our parser.** Whether our reader can yield cop car health and class for live pursuit is unknown until the format work is done.
- **Visual parity.** Colours, sizes and fade curves are tuned values; expect to tune them against the game.
- **Legal.** With no licence, reusing code or constants from this repo is off the table. Behaviour descriptions written in our own words are the safe path.
- **Scope.** This is a HUD overlay. It should follow the existing HUD work (see recent commit "fix(hud): move map blips with the widescreen minimap") and not block core work.

## 6. Open questions and what could not be verified

- **Licence.** No LICENSE file (404) and no licence section. Confirm with the author before any reuse. Not verified beyond this.
- **Verbatim source.** The source files were read through a summarising fetch, not as raw text. Function names, addresses and constants come from those summaries. They are tagged [src/...] because the summaries cite the files, but a byte-exact check was not done. The header summary reported no addresses, while the `.cpp` summary listed them; the addresses are attributed to `healthbars.cpp` on that basis.
- **Source files not read in full.** `src/healthbars.h` was read as a summary, and `include/dx9`, `lib` and `MWHealthbars.vcxproj` were not read. The `src` listing also showed a "View all files" link, so more files may exist.
- **Releases.** The release binary was not downloaded or inspected (out of scope for this task). Whether the released `.asi` matches the source is unverified.
- **Which game build the addresses target.** The README says version 1.3. The source does not show a version check, so the addresses are assumed to match that build. [inferred]
- **Latest activity.** The most recent commit is dated 25 Jan 2026 in the commit list. Whether the repo is still maintained is not known.
- **Other mods.** The compatibility commit is not explained beyond its title. Which mods replace the GUI call was not identified.
- **README.** Only part of the README was visible in the page view; the raw fetch returned a summary. Full text was not obtained verbatim.

## 7. Sources

- https://github.com/trelbutate/MWHealthbars (repository page; description, root listing, stars, commit count)
- https://raw.githubusercontent.com/trelbutate/MWHealthbars/HEAD/README.md (README, summarised by fetch)
- https://raw.githubusercontent.com/trelbutate/MWHealthbars/HEAD/LICENSE (HTTP 404; no licence file)
- https://github.com/trelbutate/MWHealthbars/tree/HEAD/src (file listing of `src`)
- https://github.com/trelbutate/MWHealthbars/commits/HEAD (commit list and dates)
- https://raw.githubusercontent.com/trelbutate/MWHealthbars/HEAD/src/healthbars.cpp (summarised by fetch)
- https://raw.githubusercontent.com/trelbutate/MWHealthbars/HEAD/src/dllmain.cpp (summarised by fetch)
- https://raw.githubusercontent.com/trelbutate/MWHealthbars/HEAD/src/primrender.cpp (summarised by fetch)
- https://raw.githubusercontent.com/trelbutate/MWHealthbars/HEAD/src/healthbars.h (summarised by fetch)

Local project references used for section 5: `docs/architecture.md`, `docs/README.md`, `docs/decisions/0004-swappable-renderers.md`, `AGENTS.md`.
