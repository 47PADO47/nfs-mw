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
| `Loading_Tips.fng` | `GLOBAL/WIDESCREEN_GLOBAL.BUN` | 33 | The career's loading screen: `LOADING...` with cop-light glows at the bottom left, a tip (title, text, picture) in the middle, and a `Press (key) To Continue` prompt that is not used. Used by the career (section 5). |
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
  overlay is removed at once rather than animated out; there is no timeout, so a scene that never reports ready keeps
  the loading screen up.
- **Which screen.** Free roam and Quick Race use `WS_Loading.fng`; the career (Career on the main menu, a loaded
  profile, a new career) uses `Loading_Tips.fng` (section 5). The choice is `LoadingScreen` in `flow_loading.rs`.
- **No freeze.** The screen opens in the frame the player pressed Career. The world is read from the install on a
  thread (`WorldScene::load`, which holds nothing of the renderer) while the screen animates; the main thread only
  puts the world together (`WorldLoad::build`, the scene holds `Rc`s and cannot cross threads) and puts it in the
  window. Before this, `WorldScene::open` ran inside the menu's frame: the window did not repaint for the 0.4 s (warm
  disk cache) to 4 s (cold) the load took. A load that fails returns to the main menu. [ours]

## 4. How to check it

1. `cargo test -p nfsmw --lib loading` runs the stage step and the free-roam options (`loading.rs`), and
   `cargo test -p nfsmw --lib concealed` the concealment rules in `scene.rs`.
2. With the install set by `NFSMW_GAME_DIR`, `nfsmw play --screenshot out.png` (offscreen, no focus) should show
   `WS_Loading` while the start area streams and the world with the car after it. Earlier frames show no world
   geometry and no car.

## 5. The career's loading screen (`Loading_Tips.fng`) [verified in the install and `speed.exe`]

- **Layout.** `LOADING...` and the three cop-light glows sit at the bottom left; the tip group is a title string, a
  body string, a stripe and a picture (`Tip_Safehouse.tga` is only the package's placeholder), faded in by the
  package's message `0xB23748F4` (name unknown). Two prompt groups exist, the keyboard's (`PC_key_enter.tga`) and the
  console's (`Cross.tga`); both stay hidden until the package's message `0x9938A38F` arrives.
- **The tip's content.** The state of the original (`FUN_005A3400`, `FUN_0057DFA0`) takes a tip index (1 to 26; any
  other value falls back to `TIP_HIDING_SPOTS`) into a table of 16-byte entries (key pointer, context mask word, 0x10,
  0x200), hashes the key, and sets the title to the label `TIP_<KEY>_HEADER`, the body to `TIP_<KEY>_DESC` and the
  picture to the texture `TIP_<KEY>` (`0xC5FBC710`, `0x0D555245` and `0xC9D77CB6` are the objects). The 24 pictures
  are in `GLOBAL/DYNTEX.BIN`, the pack the original loads on demand (302 textures; only the tips are decoded here,
  `ui::assets::add_tips`). The key list is `ui/tips.rs`; the keys and subjects are in
  [career-text-2.md](career-text-2.md) section 2.1. A test checks that every key has its picture and both labels.
- **Which tip [ours].** The context mask word is not decoded (career-text-2.md), so no tip is held back by the career's
  state: the first tip of a run is picked by the clock and the next loads follow in table order.
- **No prompt [ours].** The package has a `Press (key) To Continue` prompt (message `0x9938A38F`, name unknown, hides
  `LOADING...` and flashes the prompt). The career does not send it: the world is revealed as soon as it reports
  ready, as free roam is.

## 6. A race starting in a world that is already up [ours]

The first loading screen ended when the free-roam start had streamed. A career race (a new career's `16.1.0`, a resume, a challenge
of the Blacklist) is queued in that world and then moves the car to its grid, so the player saw the world at the start of free roam,
then the empty world at the grid while its scenery streamed, then the intro movie and NIS. The same happened between the races of
a chain (`16.1.0` to `16.2.2` and on: the outro ends, the next race is queued, the car moves to the next start).

- `Scene::race_staging()` (`scenes/world/readiness.rs`, `race_is_staging`) is true while a race is queued or in its `Loading` stage
  (the car goes to the grid and the racers' models load). `Frontend::loading_frame` reveals the world only when the scene is ready
  **and** not staging (`loading::start_over`), so the first loading screen now covers the grid too.
- In driving, `Frontend::poll_race_cover` (`flow_cover.rs`) puts the loading screen (the career's tips, else the plain screen) over the
  concealed world as soon as a race is staging, and the stage is `Loading` until the grid stands. Restarts (the pause menu's Restart, the
  results screen's Restart, a retry) stay on their start and are not covered (`Frontend::restarting`).
- **Original:** the load of the next race of a chain keeps the loading screen up from the end of the previous race to the start of the
  next one; the order of loading screen and cinematic in the original was not re-read for this change (loading screen first, then the
  intro movie and NIS, as the Lua graph's `prerace` waits for the world load before `intronis`). **[unconfirmed]** in `speed.exe`.
- **Not covered:** the frame in which the previous race has just ended and the next is queued (one frame, the old place);
  streaming that a NIS needs away from the race start; the car lot after `16.2.1` (shown over the frozen world); the menu backdrop
  that is built when a movie of the first safe house visit ends.
