# Loading screen (free roam)

What the original showed while free roam streamed in, and how this rewrite gates entry into the world. The
packages are described in [formats/frontend.md](../formats/frontend.md), the flow around them in
[frontend-menus.md](frontend-menus.md) section 4.

- **Sources read:** the install's FEng packages through `nfsmw list-screens` and `nfsmw dump-screen` (the install is
  `Need For Speed Most Wanted Black Edition`, the packages in `GLOBAL/WIDESCREEN_GLOBAL.BUN`, `GLOBAL/GLOBALA.BUN`,
  `GLOBAL/GlobalB.lzc` and `FRONTEND/FrontB.lzc`); `speed.exe` through the Ghidra MCP (strings, byte searches,
  `FUN_00549460`, `FUN_0057E050`). The dbalatoni13 decompilation was not searched for loading code in this pass, so
  nothing here is **[decomp]**.
- **Evidence tags** as in the [docs README](../README.md#evidence-tags). **[verified]** = checked in the install or
  `speed.exe`; **[unconfirmed]** = observed but the use is not proven; **[ours]** = a decision of this rewrite.

## 1. The packages of the install [verified]

| Package | File | Objects | What it is |
|---|---|---|---|
| `loading_boot.fng` | `GLOBAL/GLOBALA.BUN` | 5 | The boot-time logo card (`MW_Logo.tga` on `BasePoly.tga`). Not used by free roam. |
| `WS_Loading.fng` | `GLOBAL/WIDESCREEN_GLOBAL.BUN` | 34 | The loading screen: a dark panel, the word `LOADING` twice, a pair of cop-light and headlight glow groups that sweep across, and a stripe (`BaseStripe.tga`) whose UVs scroll on a looping script. |
| `WS_Loading_Controller.fng` | `GLOBAL/WIDESCREEN_GLOBAL.BUN` | 74 | The same screen with a controller hint. Not used yet. |
| `Loading_Tips.fng` | `GLOBAL/WIDESCREEN_GLOBAL.BUN` | 33 | A loading tip (`Tip_Safehouse.tga`) with `Press` / `To Continue` and a key glyph (`PC_key_enter.tga`). Not used yet. |
| `FadeScreen.fng`, `FadeScreenNoLoadingBar.fng` | `GLOBAL/GlobalB.lzc` | 18, 1 | A fade with and without a loading bar (the executable has the `EFadeScreen*LoadingBar*` event names). Not used yet. |

There is **no** 4:3 loading package in the install (`Loading.fng` is named in the executable but not present), so
`WS_Loading.fng` serves every aspect ratio.

**Progress.** `WS_Loading.fng` has no object whose size or colour follows a load percentage: the stripe runs a looping
script of 2400 ticks. The original bar is therefore a looping animation, not a progress meter. **[verified]**

**Messages of `WS_Loading.fng`.** Name hashes are `fe_hash_upper` of the name (`ids.rs`).

| Hash | Name | Sent to | Use in the package |
|---|---|---|---|
| `0x3D4B7270` | unknown (not in the candidate names tried) | the groups and the splash image | runs their `0x3D4B7270` scripts (fade-in colours and vectors) |
| `0x5D7C6A21` | unknown | the `LOADING` group | runs a script on that group |
| `0x6A0831E8` | `LOADED` **[verified name]** | the `LOADING` group | runs script `0x0016A259` (empty, probably the reset or hide) |
| `0x75BB3638` | unknown | the `LOADING` group | runs script `0x5079C8F8` |
| `0xCB835EE3` | `CURRENT_GEN_WIDESCREEN` | the widescreen background | the same as the title screen (`splash.rs`) |

`LOADED` is the message that tells the screen the load is over. No code in `speed.exe` pushes it as an immediate
(`68 E8 31 08 6A` and the bare bytes `E8 31 08 6A` both have no match), so the place that sends it is **[unconfirmed]**.

## 2. The original's code, as far as it was read [verified]

- `WS_Loading.fng` is pushed at `0x0054950F` inside `FUN_00549460`, next to `ControllerUnplugged.fng`,
  `Loading.fng` and `Pause_Main.fng`. `FUN_00549460` picks the screen to show while the game waits (the
  controller-unplugged case first, then the loading case); the code path that loads it has not been traced further.
- `Loading_Tips.fng` is named by `FUN_0057E050`, a constructor of a loading-tips state that loads the package and
  creates a 16-byte object. It is called from `FUN_0058F050`. Whether tips are shown during every load is
  **[unconfirmed]**.
- Timing and the exit animation after `LOADED` were not found. **[unconfirmed]**

## 3. What this rewrite does [ours]

- **Stage.** Free roam no longer enters `Stage::Playing` at once. `start_free_roam` builds the world with
  `wait_for_load: true` (`flow.rs`, `free_roam_options`), wraps it as `Pausable` with a `ConcealFlag` set, and enters
  `Stage::Loading`, where `WS_Loading.fng` is opened over the empty window.
- **Readiness: the existing `Scene::ready()`.** Each frame, `Frontend::loading_frame` reads `host.scene.ready()`.
  For the world that is the `wait_for_load` branch (grounded and `residency.complete()`) before the car exists, and
  the driving branch (residency complete, the road under the car found, the script done, the traffic settled) once
  it does. Nothing in `scenes/world` changed.
  - Why not Bevy's ready mechanism: the Bevy 0.20 asset-ready pattern tracks `AssetServer` handles. The world's
    readiness is the streamer's residency state (`residency.complete()`), which Bevy does not see; using Bevy's would
    need a second signal that mirrors it. This was judged from the codebase, not from the Bevy 0.20 release notes
    (not checked in this pass).
- **Concealment.** The world is still updated while loading, so it streams. `Pausable` (`scene.rs`) conceals it while the
  flag is set: the frame is computed (camera and physics read the aspect) but not drawn, the HUD is hidden, the engine
  and traffic sounds are dropped, and the mouse is not captured. The black frame behind the loading package is the
  flat frame of the menus.
- **Reveal.** `loading::step` clears the flag once `ready()` is true, the loading screen is cleared, and the stage
  becomes `Stage::Playing` in the same frame, with the latch set so a held pad button cannot pause at once.
- **Screen logic.** `WS_Loading.fng` has no logic of its own (`factory.rs` runs it as `Plain`). The loading screen
  takes no input: the stage ticks the screens with pad mask 0, so Start and Escape do not pause during loading.
- **Not done [ours]:** `LOADED` is not sent (its send site is unknown, and the package's exit is not traced); the
  overlay is removed at once rather than animated out; `Loading_Tips` is not shown; there is no timeout, so a
  scene that never reports ready keeps the loading screen up.

## 4. How to check it

1. `cargo test -p nfsmw --lib loading` runs the stage step and the free-roam options (`loading.rs`), and
   `cargo test -p nfsmw --lib concealed` the concealment rules in `scene.rs`.
2. With the install set by `NFSMW_GAME_DIR`, `nfsmw play --screenshot out.png` (offscreen, no focus) should show
   `WS_Loading` while the start area streams and the world with the car after it. Earlier frames show no world
   geometry and no car.
