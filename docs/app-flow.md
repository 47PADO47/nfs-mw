# App flow: screens, scenes and renderers

How the `nfsmw` binary moves from launch to driving, which parts are Bevy and which are not, and how the
renderer is chosen. This is the walkthrough; the crate map is in [architecture.md](architecture.md), the
renderer settings in [renderers.md](renderers.md), and the Bevy backend in [bevy-backend.md](bevy-backend.md).

## Short answer

- The app is a **Bevy `App`** (window, loop, input, ECS resources). Bevy does **not** draw the game by
  default and does **not** draw the menus.
- The **flow** is a state machine (`Stage`), not a URL router. There is no `/settings` route. Menus are a
  **stack of screens**.
- A **scene** is anything that implements the `Scene` trait (`init`, `update`, `frame`). The world and the
  car viewer are scenes. The frontend swaps which scene is active.

## Layers

| Layer | What it is | Where |
|---|---|---|
| Bevy shell | `bevy_app`, `bevy_ecs`, `bevy_winit`, `bevy_input`, `bevy_gilrs` | `app/` |
| Flow | `Frontend` resource, holds `Stage` | `frontend/flow.rs` |
| Screens | `Screens` (stack of FEng packages), `Runtime`, `ScreenLogic` per screen | `frontend/`, `ui/` |
| Scenes | `Scene` trait objects: world, car viewer | `viewer/mod.rs`, `scenes/` |
| Renderer | `RenderBackend` from `blackbox-gfx`: native `blackbox` (default) or `bevy` | `app/render/` |

Frame order is `Prepare` → `SceneUpdate` → `Frontend`/`Ui` → `Draw` (see
[architecture.md](architecture.md#the-application-shell)).

## Stages

`Stage` in [crates/nfsmw/src/frontend/flow.rs](../crates/nfsmw/src/frontend/flow.rs):

```
Stage::Boot(..) ──► Stage::Menus ──► Stage::Playing ⇄ Stage::Paused
 movies, title      profile step,    world scene       pause menu over the
                    main menu,                         frozen world
                    options, ...
```

| Stage | What the window shows | Notes |
|---|---|---|
| `Boot` | Queue of `Boot` items: `ealogo` movie, `psa` movie (has a `bypass` flag), then the `Splash` screen (title). The movies are left out when the `skip_intro` setting is on | Movies play through the movie scene (`movie/`, `blackbox-movie`). |
| `Menus` | The profile step after the title screen (see [Start-up profile step](#start-up-profile-step)), then the main menu (`IconMenu`), option categories, `WidgetMenu` rows | Run as the game's own FEng screens. |
| `Loading` | The original `WS_Loading.fng` screen over an empty window while free roam streams in | The world is built and updated but hidden and silent until `Scene::ready()`; then `Playing`. See [specs/loading-screen.md](specs/loading-screen.md). |
| `Playing` | Free roam: the world scene, with the HUD | Career runs here (see [Career flow](#career-flow)). |
| `Paused` | Pause menu over the world | The world is wrapped in `Pausable`, so it freezes. Escape or Start opens it. |

Your sequence (video, splash, profile step, main menu, world) is exactly `Boot` → `Menus` → `Playing`.

### Start-up profile step

After the title screen and before the main menu the game asks for a profile, as the original does (`MC_Main.fng` in start-up
state `0x20`; [specs/boot-profile-prompt.md](specs/boot-profile-prompt.md), `frontend/flow_startup.rs`):

- saved profiles (this build's folder and the original's `Documents\NFS Most Wanted`, read only): the list `MC_List.fng` ("load")
  over a black screen. A pick loads the profile into the session and says "Load complete."; Back goes on with none loaded;
- none saved: the question "Create a new alias?", the two autosave notices and the name prompt; the alias is made, saved and
  loaded;
- then the main menu. The step is over when its last screen closes (`Frontend::poll_startup`).

How `play` starts (`commands/start.rs`): no switch is the whole flow; `skip_intro` drops only the movies; `--skip-boot` and
`--ui-script` start at the main menu with no prompt; `--aliases` runs the profile step alone (the way to test it in a script).

## Screens (menus)

The menus are the install's own FEng screen files, not Bevy UI and not a web-style router.

- `Screens` keeps a **stack** of loaded packages and one `Runtime`. Moving into an option pushes a screen;
  leaving it pops.
- Each screen has a `ScreenLogic`, which is the equivalent of the original screen classes:
  - `IconMenu`: the main menu, option categories and pause menu (icon scroller).
  - `WidgetMenu`: option rows (titles, values, sliders).
  - `Splash`: the splash screen.
  - Screens without their own logic run as their package says.
- `options.rs` maps option rows to settings.
- Pad or keyboard input becomes a pad mask (`Menu*` actions). The runtime moves focus by geometry.

**Settings from menus:** an option row changes `Settings` at once (volume, vsync, frame cap, overlay, HUD,
renderer). The config file is written when a screen is left (`settings/write.rs`), and it keeps `game_dir`
and unknown keys.

**Not run yet:** quick race, customisation, online, dialogs and the keyboard screens. The career screens are run
(see [Career flow](#career-flow)). No
confirmation dialogs, no mouse, no 3D backdrop behind the menus.

## Career flow

Career on the main menu starts or resumes a profile, then goes to free roam with it ([career-flow](specs/career-flow.md)
sections 2 to 4; code in `crates/nfsmw/src/career/start.rs` and `frontend/flow_career.rs`).

1. **Career** (main menu) calls `career::start_or_resume`. The latest profile is loaded when its career has started;
   otherwise a new career is made on the default alias. A failure stays on the main menu and shows an alert.
2. The career being played is set, the world is given it, and the game goes to `Playing` (free roam) in career mode.
   Outside career mode nothing of the career runs.
3. **Safe house request.** A safe house entry in the world asks for the safe house menu. The world is frozen and the
   menu (`MainMenu_Sub` in safe house mode) opens over it. Its items are Resume, Blacklist, Car Select, Rap Sheet and
   Save.
4. **Resume** in the safe house menu leaves the menu and restarts free roam (`SetRoaming`). That restart asks for the
   roaming-start autosave. Other resumes (the pause menu, the bust ticket) do not.
5. The car lot (`Car_Select`), the region unlock screen and the rap sheet open over the frozen world the same way when
   the world asks for them. Confirmed car lot purchases and sales are applied to the career; a refusal shows an alert.

Autosave runs at three points: roaming start, event start and pursuit evaded
([career-save-screens-2](specs/career-save-screens-2.md) section 8).

## Scenes

The `Scene` trait in [crates/nfsmw/src/viewer/mod.rs](../crates/nfsmw/src/viewer/mod.rs):

- `init(renderer)` uploads resources once.
- `update(renderer, input, dt)` advances camera, streaming and uploads.
- `frame(aspect)` returns the frame to draw.
- Optional: `speed_streaks`, rear-view mirror frame, commands, readout, HUD state, and `set_transmission`.

Scenes in the tree:

- `scenes/world/`: free roam. Map streaming, drive, AI, props, effects, vehicle effects, exhaust, sound.
- `scenes/car/`: the car viewer (`view-car`).
- `viewer/`: cameras (`fly`, `orbit`, `cut`) and the `Scene` trait.

Scenes do not know about screens. The frontend hands them input and decides when they are active.

## Renderer selection

Two separate settings (see [renderers.md](renderers.md)):

- `renderer`: **who draws**, `blackbox` (default, native) or `bevy`.
- `backend`: **which graphics API**, `auto`, `vulkan`, `dx12`, `gl`.

Chain, in [crates/nfsmw/src/app/render/select.rs](../crates/nfsmw/src/app/render/select.rs):

1. `decide(requested, bevy_compiled)`: `bevy` without the `renderer-bevy` feature falls back to `blackbox`
   with a reason.
2. `decide_with_probe(...)`: if Bevy was compiled, probe for a Vulkan or DX12 adapter. Bevy panics on
   startup with no adapter, so the probe runs first. A failed probe falls back to `blackbox` with a reason.
3. Chosen renderer starts. A fallback is logged once and never fails startup.

Precedence for the value: command line, then environment (`NFSMW_RENDERER`), then config file, then preset,
then default, key by key.

### Using the Bevy renderer

Build:

```bash
cargo run --release -p nfsmw --features renderer-bevy -- play
```

Needs Rust 1.97.1 or newer for this build. Other crates stay on 1.95.

Switch: Video options, **Renderer** row (main menu or pause menu) → **Bevy**, then restart.
Or set `renderer = "bevy"` in the config file, `--renderer bevy`, or `set renderer bevy` in the F12 console.

What the Bevy renderer does and does not do (status as of the spike; see
[bevy-backend.md](bevy-backend.md) for detail):

| Area | `blackbox` (native) | `bevy` |
|---|---|---|
| World meshes, textures, fog, alpha test, blending, sky | yes | yes |
| Anti-aliasing `smaa`, `taa` | no | yes |
| Upscaler `fsr3` | yes (Vulkan/DX12) | yes (Vulkan/DX12) |
| Upscaler `dlss`, `fsr4` | no | yes (Vulkan/DX12; `dlss` NVIDIA RTX only, separate build option; `fsr4` DX12 RX 7000/9000 only) |
| Ray tracing | no | yes, on GPUs with ray queries |
| Car reflection captures | yes | no (static sky) |
| HUD, menus, UI layer | native UI layer | **not drawn through the Bevy renderer yet** (docs say the spike draws the world only; not verified in code) |
| Graphics API | Vulkan, DX12, OpenGL | Vulkan, DX12 (no OpenGL) |

Unsupported requests are downgraded per renderer and logged once. Stored values for features the active
renderer lacks stay in the config file.

## Checking it

```bash
cargo test -p nfsmw                                          # flow, settings, renderer fallbacks (no GPU)
cargo run --release -p nfsmw --features renderer-bevy -- view-world --renderer bevy   # world only, quick check
nfsmw list-screens                                           # screens in the install
nfsmw view-screen NAME --screenshot out.png                  # one screen
nfsmw play --skip-boot                                       # the flow from the main menu
```

## Open items

- Confirm in code that the Bevy renderer skips the HUD and menu layers (documented, not checked).
- Quick Race opens Quick Play, Custom Race (mode select, track list, options, car select, transmission): [specs/custom-race.md](specs/custom-race.md), or Split Screen (the same screens for two players, with a Press Start screen before each player's car select; the race is handed to the world with `Command::StartSplitRace`): [specs/split-screen.md](specs/split-screen.md).
- Full list of unrun screens: [specs/frontend-menus.md](specs/frontend-menus.md).
