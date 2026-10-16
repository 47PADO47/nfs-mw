# Front-end shadows

Status: spec only. Nothing is built. The original game draws no shadow maps in its front-end menus, and the
rewrite does not either by default. This spec adds an optional front-end shadow pass, off by default.
It covers the crib stage behind the main menu now, and the other front-end stages when the port has them.

Tags: **[decomp]** read from the original `speed.exe` decompile, not yet seen in game. **[verified]** checked
against install bytes or game data. **[ours]** this rewrite's current code or a proposed choice. **[community]**
outside knowledge (Direct3D 9 enum values). **[unconfirmed]** a guess that needs a check.

Provenance (addresses, Ghidra names, method): [docs/provenance/fe-shadows.md](../provenance/fe-shadows.md).
Feature list for the user-facing options: [docs/mods/nfsmwfeshadows.md](../mods/nfsmwfeshadows.md) (behaviour only).
Background: [menu-backdrop.md](menu-backdrop.md), [soft-shadows.md](../soft-shadows.md),
[garage-overlay.md](../garage-overlay.md).

## 1. Summary

- The original never renders a shadow map while a front-end menu is up. The shadow pass sits inside the world
  scene draw, and the front-end frame path skips that draw. [decomp]
- Even in the world, the pass needs the registry value `g_ShadowDetail` above zero. The default is on only when
  the performance level is above 3. [decomp]
- The rewrite already has dynamic sun shadows for the world (Bevy cascades, native prototype). `MenuScene` has
  no front-end shadow policy of its own. On Bevy the global `shadows` setting applies to its instances too, as
  far as the code shows. [ours, unconfirmed]
- Proposal: a `fe_shadows` setting (default off). When on, the front-end scenes use the existing shadow path
  with their own caster and receiver rules, a fixed sun direction, and optional bias and resolution tuning.
  Only the Bevy backend supports it in v1. [ours, proposed]

## 2. What the original does [decomp]

### 2.1 The front end skips the world draw

- `0x006e7220` is the per-frame render. It calls the world draw (`0x006de300`) only when the mode predicate
  `0x00517a20` returns 0. The predicate is true when the UI object at `0x0091cb10` has mode 3, 4 or 5 at
  offset `+0x11c`. [decomp]
- The shadow pass is called from inside the world draw. So in those modes no shadow pass runs. [decomp]
- The world draw also holds the front-end car block (`0x00925e90 == 3`). It is skipped in the same modes.
  [decomp]
- Which path draws the front-end stage while the menus are up is not yet known. No path found calls the shadow
  pass in those modes. [decomp, unconfirmed] See Next leads.
- Flow state `0x00925e90`: 3 is the front-end session, 6 is driving. Both are read from many gates (rain,
  weather, the world draw). The mode numbers 3, 4 and 5 are not named in the binary. [decomp, unconfirmed]

### 2.2 The shadow pass

- The driver sets the depth bias to `4 * 2^-24` (about `2.4e-7`) and restores the state afterwards. The
  numbers are the Direct3D 9 render-state values for depth bias (195) and slope-scaled bias (175). [decomp,
  community]
- The gate is `g_ShadowDetail` (registry, `lpData_00901830`) above zero, and a float pair compared in the world
  draw (`0.1` against `1.0`, both as loaded). The driver also tests a byte that is `1` in the file and has no
  writer, so it never closes. [decomp, verified]
- The matrix builder takes the sun direction from an object vector (normalised), a box fitted to the scene
  bounds, and two projection blocks of 54 floats each, one per distance band. A scale of `2000.0` pushes the
  light-space eye out along the sun. [decomp]
- The projection constants include near `0.01` and far `50.0`. The map edge is not in the matrix code, so its
  size is unknown. [decomp, unconfirmed]

### 2.3 Casters and receivers [decomp]

- Casters: scene objects drawn through the scene traversal with shadow draw flags; and a second list of
  vehicle-like objects, drawn through a virtual slot when their count is above one. The focus car is drawn
  separately. The flag depends on `g_ShadowDetail` (1 chooses one flag, other positive values another) and on
  `0x008faecc`.
- Receivers: the shader-parameter class in the car and world shading (its per-draw method reads
  `g_ShadowDetail` before setting the shadow constants). The per-frame setup writes fade and matrix constants
  for the pass block. Without the gate, no receiver takes the shadow term.

### 2.4 Settings in the original [decomp]

- `g_ShadowDetail` is a registry DWORD under `Software\EA Games\Need for Speed Most Wanted`. It is written with
  the other `g_` settings.
- Default: `1` only when `g_PerformanceLevel` is above 3, otherwise `0`. The value is then clamped by a cap
  (`0x009018b8`, written at start-up by `0x006d2100`, which is not yet read). [decomp, unconfirmed]
- In the caster draw, `0x008faecc` set selects a draw flag by detail: detail 1 gives one flag, other positive
  values another. With `0x008faecc` clear, the flag is `0x100` plus a state value that is `0x800` or `0`. [decomp]

### 2.5 Why the original has no front-end shadows

1. The shadow pass lives only in the world draw. [decomp]
2. The front-end modes skip that draw. [decomp]
3. Even in the world, the detail setting defaults to off on lower tiers. [decomp]

So the front end has no shadow path to turn on. It was not an artistic choice in the code that we can see. Our
evidence covers the call graph and the gates. It does not yet cover the mode names or the stage's draw path.

## 3. Which scenes get shadows

| Stage | Source file (menu-backdrop) | Port status | Shadows in v1 |
|---|---|---|---|
| 1 Platform crib | `FRONTEND\PLATFORMS\PLATFORMCRIB.BIN` | `frontend/room.rs` and `MenuScene` draw it (the room is written; the crate was not rendered in the last check) | Yes |
| 2 Career safe house | `CAREER_SAFEHOUSE.BIN` | No scene | Later, when the scene exists |
| 3 Customization shop | `CUSTOMIZATION_SHOP.BIN` | No scene | Later |
| 4 Shop backroom | `CUSTOMIZATION_SHOP_BACKROOM.BIN` | No scene | Later. This is the front-end garage, assumed [ours] |
| 5 Car lot | `CAR_LOT.BIN` | No scene | Later |

- The in-world garage overlay (`docs/garage-overlay.md`, key F6) is part of the driving world, not the front
  end. It uses the world's shadow setting and is out of scope.
- The stage number comes from the front-end state, as in menu-backdrop section 1. A scene asks for shadows
  through its own `Scene` implementation, not through a global switch.

## 4. Sun and light direction

- The original's front-end light is not known. The world sun is an object vector (`0x009b392c + 0x180`) that
  the front end may not set. [decomp, unconfirmed]
- The front-end block in the original uses constants (`450`, `19`, `190`, `89`) that are not yet mapped to a
  light. [decomp, unconfirmed]
- The rewrite uses a fixed direction for the viewer and the car: `LIGHT_DIR` in `crates/nfsmw/src/scenes/car/mod.rs`,
  set to `(-0.4, -0.3, -1.0)`. The key light of the car rig is the sun opposite to it (`scenes/car/lighting.rs`).
  [ours]
- Decision for v1: the front-end shadow sun is `-LIGHT_DIR`, the same direction as the car key light. Stage data
  is not read for light, because none has been found. [ours, proposed]
- A stage that does have a light vector later overrides the fixed one, per stage. The override is a constant
  set in the stage's module, not a setting.

## 5. Casters and receivers (our rules)

Receivers and casters are chosen per scene. The engine takes lists of instances and does not know about scenes.

| Object | Receives | Casts | Notes |
|---|---|---|---|
| Crib room solids (walls, floor, fixtures) | Yes | Yes, except floor-only solids | A ceiling or high wall may shade the car at a steep sun; check in the screenshot test |
| Placeholder floor (only when the room is absent) | Yes | No | Off when the room exists (`MenuScene` already does this) |
| Car body and parts | Yes, when `fe_shadows_cars` is on | Yes, when `fe_shadows_cars` is on | Self-shadowing follows from casting. Off removes both |
| Car glass, lights, decals (non-opaque) | Per their material | No | Same rule as the world: additive, blended and sky never cast |

- `fe_shadows_cars` is off by default in the mod, and on by default in our proposal. This is an open decision
  (see Next leads).
- The engine side needs one new engine-generic flag per instance: whether it receives sun visibility. A
  no-cast flag already exists for the Bevy path (`apply/instances.rs`, `casts()`). Receiving has no flag yet.
  Add it to `blackbox-gfx`'s instance, not to the car crate.

## 6. Resolution, bias and defaults

| Quantity | Value | Source |
|---|---|---|
| Quality table (edge, cascades, distance) | Low 1024, 2, 80; Medium 2048, 3, 160; High 2048, 4, 240 | `libs/blackbox-gfx/src/settings/shadows.rs` [ours] |
| Front-end quality | Global `shadows` if it is not off, otherwise Medium | Proposed [ours] |
| Front-end map edge divider `fe_shadows_divider` | 1, 2 or 4; default 1 | Proposed [ours]. The original's map size is unknown |
| Bevy depth bias | 0.02 (default) times `fe_shadows_bias` | `apply/shadows.rs` [ours] |
| Bevy normal bias | 1.0 (default) times `fe_shadows_bias` | `apply/shadows.rs` [ours] |
| `fe_shadows_bias` | Multiplier, default 1.0, clamped to 0.25 to 4.0 | Proposed [ours] |
| Original depth bias | `4 * 2^-24` | [decomp] (not copied; the value is a fact) |

- The bias and divider settings change only the front-end scenes. The world keeps its own values.
- A divider of 2 on Medium gives an edge of 1024. A divider cannot make the edge smaller than 512.

## 7. Settings, names and surfaces

Default: off. The names are proposed and follow the existing `shadows` setting. [ours, proposed]

| Surface | Name | Values | Default | Where |
|---|---|---|---|---|
| TOML | `fe_shadows` | `true` or `false` | `false` | Same table as `shadows` in `crates/nfsmw/src/settings/file.rs` |
| TOML | `fe_shadows_cars` | `true` or `false` | `true` (open) | As above |
| TOML | `fe_shadows_bias` | number 0.25 to 4.0 | `1.0` | As above |
| TOML | `fe_shadows_divider` | `1`, `2` or `4` | `1` | As above |
| Environment | `NFSMW_FE_SHADOWS` | `on`, `off` | unset | `crates/nfsmw/src/settings/env.rs`, pattern of `NFSMW_SHADOWS` |
| Console | `fe_shadows on\|off` | as above | off | `devtools/console/settings_cmd.rs`, `settings_keys.rs`, `parse.rs` help |
| Menu row | "Front-end shadows" | Off, On | Off | `frontend/graphics_options.rs` (new variant beside `Shadows`) |

- The menu row is shown only when `caps.soft_shadows` is true (the Bevy build). On native, the setting is forced
  off, the same way `shadows` is (`settings/availability.rs`, `graphics_tests.rs`).
- The menu's `step` cycles off and on. Bias and divider have no menu row in v1; they are TOML and console only.
- Saved values round-trip through `write.rs`, like `shadows`.

## 8. Behaviour per backend

- **Bevy (`libs/blackbox-bevy-render`)**: supported. The sun is one shadow-only `DirectionalLight` synced from the
  frame's light direction (`apply/shadows.rs`). Every spawned instance in the frame casts, except those marked
  non-casting (`apply/instances.rs`). So a front-end frame that carries car and room instances already gets
  shadows when the global `shadows` setting is on. This is read from code, not run in game. [ours,
  unconfirmed] Front-end shadows then need only the setting's effective value and the rules in section 5.
- **Native (`libs/blackbox-render`, `gpu/shadows`)**: a prototype behind `RenderExperiments.sun_shadows`, which
  the game never sets (only the graphics lab does). Native reports `soft_shadows` false, so the setting is off.
  Native front-end shadows are out of scope for v1. If enabled later, the front-end scene must set the
  experiment, and the casters fall back to the frame's visible list (`set_shadow_casters` is only called by
  the world). [ours]
- **OpenGL**: no shadows. The native path reports `stats.supported` false for GL, and `docs/soft-shadows.md` says
  the GL backend downgrades the setting to off. Off.
- **Headless `--screenshot`**: uses the same backend as the window. Verification uses it (section 10).

## 9. Target modules

Engine-generic (no NFS:MW names) goes in `libs/`. Game policy goes in `crates/nfsmw/`.

- `libs/blackbox-gfx/src/settings/shadows.rs`: keep the quality table. Add a divider helper (edge over divider,
  clamped), engine-generic.
- `libs/blackbox-gfx/src/instance.rs` (or where `Instance` lives): add a receive flag, default on, engine-generic.
- `libs/blackbox-bevy-render/src/apply/shadows.rs`: accept a bias multiplier and a per-frame override of the
  effective quality and bias. Keep the global path unchanged.
- `libs/blackbox-bevy-render/src/apply/instances.rs`: honour the receive flag in the material (sun visibility 1.0
  when off).
- `crates/nfsmw/src/frontend/fe_shadows.rs` (new): the policy. Given the front-end scene, return the quality,
  bias, divider and caster and receiver flags. Pure functions, no renderer.
- `crates/nfsmw/src/frontend/scene.rs`: `MenuScene` asks the policy for the flags and sets them on its instances
  (cars and room parts).
- `crates/nfsmw/src/settings/{mod.rs, partial.rs, file.rs, env.rs, write.rs, availability.rs, graphics.rs}`: the new
  keys, with the same layering as `shadows`.
- `crates/nfsmw/src/frontend/graphics_options.rs`: the menu row.
- `crates/nfsmw/src/devtools/console/{settings_cmd.rs, settings_keys.rs, parse.rs, gfx_cmd.rs}`: console.
- `crates/nfsmw/src/scenes/car/lighting.rs` and `scenes/car/mod.rs`: export the front-end sun (`-LIGHT_DIR`) for
  the policy. Do not change the car's rig.
- Not touched: `crates/nfsmw/src/scenes/world/shadow_feed.rs` and the world's shadow setting. The world must not
  change.

## 10. Tests

Unit (CPU, no install):

1. `fe_shadows` defaults to `false`; absent from TOML gives `false`; `true` and `false` parse; a bad value is an error.
2. `NFSMW_FE_SHADOWS` parses like `NFSMW_SHADOWS`.
3. Console `fe_shadows on` and `fe_shadows off` set and report the value; the help text lists it.
4. Native test caps: the setting is forced off, and the menu row is hidden.
5. Effective front-end quality: global `off` with `fe_shadows` on gives Medium; `fe_shadows` off gives off;
   global `high` gives High.
6. Bias clamp: 0.1 gives 0.25; 9.0 gives 4.0; default is 1.0 and gives 0.02 and 1.0 on Bevy.
7. Divider: Medium with divider 2 gives edge 1024; divider 4 gives 512; 8 is rejected.
8. `fe_shadows_cars` off gives car instances the no-cast and no-receive flags; on gives neither.
9. Front-end sun equals `-LIGHT_DIR`, and is the car key light (`lighting::rig` test).
10. Menu step cycles off and on.

GPU (Bevy, ignored, needs a device):

11. A caster above a receiver darkens the receiver with `fe_shadows` on, and not with it off. Add to
    `libs/blackbox-bevy-render/src/shadow_tests.rs`. Run:
    `cargo test --release -p blackbox-bevy-render shadow_tests -- --include-ignored --test-threads=1`.
12. The world shadow tests still pass unchanged.

Screenshot (offscreen, never focus):

13. `nfsmw view-screen MainMenu.fng --screenshot <file>` with `fe_shadows` off and on. The on image has a shadow
    under the car. Do not start the game window.

## 11. Open questions

- Default of `fe_shadows_cars`: on (proposed) or off (mod default)? Decide before building.
- Does the room's ceiling or high wall shade the car at the fixed sun angle? Decide from the screenshot.
- Should the front-end sun ever follow a stage's own light? Not until stage light data is found.
- The original's map size and bias values are not known. The divider and bias defaults are our choice, not the
  game's.

## Next leads

1. Name the modes 3, 4 and 5 of `0x0091cb10` at offset `+0x11c`, and find the writer of that field.
2. Find the draw path that renders the front-end stage while the menus are up. Candidates: `0x006e6e40` with
   argument 0, and `0x0064a7c0` (called from `0x00663d30` when `0x00925854 == 0`).
3. Find the shadow map size: the render-target creation near the shadow matrix code, or the device calls in
   `0x006e54e0`.
4. Decode the draw flags `0x100`, `0x200`, `0x1000`, `0x2000` and `0x00982b9c` in the scene traversal
   `0x00723fa0`.
5. Identify the object at `0x009b392c + 0x180` and check whether the front end fills it (the front-end sun).
6. Read `0x006d2100`, the writer of the `g_ShadowDetail` cap, to learn the start-up limit on shadow detail.
7. Map the front-end light block in `0x006e6e40` (constants `450`, `19`, `190`, `89`).
8. Check in game (not in the spec) whether the global `shadows` setting already shadows the crib on Bevy. This
   decides how much of section 8 is new work.
