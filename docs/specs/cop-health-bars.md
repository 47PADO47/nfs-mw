# Cop health bars (optional overlay)

An optional overlay that draws a small health bar above each police car that is chasing the player. It is a
reimplementation, in our own words, of the behaviour of the MWHealthbars mod (reference: [../mods/mwhealthbars.md](../mods/mwhealthbars.md),
which summarises a repository with no licence). No code, layout or address from that repository is used. The
original game shows no per-car bar: its pursuit board has only aggregate counters ([hud-pursuit.md](hud-pursuit.md),
[../provenance/ai-pursuit-cop-damage.md](../provenance/ai-pursuit-cop-damage.md)). So this is a rewrite extension, and it
is **off by default**.

Provenance and the list of unconfirmed values are in [../provenance/cop-health-bars.md](../provenance/cop-health-bars.md).

- **Evidence tags.** As in [docs/README.md](../README.md#evidence-tags), plus the tags of this spec:
  **[community]** = a value or rule from the mod reference (a summary of the public repository, not the code text);
  **[verified]** and **[decomp]** = the port's cop damage model, as its spec ([ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md)
  section 6) records it; **[design]** = a value this spec chooses because the reference does not give it;
  **[unconfirmed]** = a design value that needs a look at play before it is trusted.
- **Units.** World units are metres in the port's render space (x forward, y left, z up, as in
  [../../crates/nfsmw/src/viewer/camera/mod.rs](../../crates/nfsmw/src/viewer/camera/mod.rs)). The mod's unit is taken to be
  one metre. **[unconfirmed]**
- **Timers** count simulated seconds, not frame time. The bars pause with the game and follow the game-speed setting.

## 1. What the player sees

- One bar per cop car that is chasing the player, drawn in the world above the car. Helicopters never get a bar.
- The bar is a border, a fill whose length is the car's health, and a pale "lagging" segment that shows the health lost
  to the latest hits and then drains away. **[community, section 2 of the reference]**
- The fill colour steps through five tiers as health falls. **[community]**
- A destroyed cop flashes, then fades out. The bar is removed after the fade. **[community]**
- The bars are hidden when no chase is on, during the port's caught pan and arrest shots, and when the HUD is hidden.
  Nearby and distant bars fade out. **[community, with the HUD rule as design]**

## 2. Which cars get a bar

The mod reads a table of cop cars, skips helicopters, and draws only while a pursuit is on. The port's equivalents:

| Rule | Port test | Note |
|---|---|---|
| Chasing cop | `AiCar` whose `role` is `Role::Cop(_)` | The doc comment on `AiCar::is_cop` says "a cop chasing the player". Patrol cops (`patrol` flag, `Role::Traffic`) are excluded. Roadblock cars are placed as `Role::Traffic` (`ai/roadblock.rs:107`), so they get no bar either. Whether they should is open question 8. |
| Wrecked cop still gets a bar | Do **not** use `AiCar::is_cop()` for membership | `is_cop()` is false once `is_wrecked()` is true (`car.rs:307`). A death effect needs the wrecked car, so the test is the role alone. |
| Helicopter skipped | Helicopters are not in `TrafficWorld::cars`; they live in `police/heli.rs` with `helicopter_blip` | Keep an explicit guard in the sample function and a test for it. |
| Chase on | `TrafficWorld::is_pursuing()` (`ai/mod.rs:197`) | Same test as the minimap's `active` flag (`ai/mod.rs:245`). |

Two facts about the port affect the bars. First, `TrafficWorld::end_pursuit` removes cops with `retain(|c| !c.is_cop())`
(`ai/pursuit.rs:103`), so the cops still chasing go, and wrecked cops stay in `cars` as ordinary wrecks. Second,
`update_pursuit` drops a cop that falls farther than `LOST_DISTANCE` from the player (`ai/pursuit.rs:134-137`). A cop
that is dropped takes its bar with it. The bar state is removed when its cop is no longer sampled (section 12.4).

## 3. Health

### 3.1 The port's numbers

The port keeps no health value. It keeps the **damage** taken, and the car is wrecked when the damage passes 1
(`libs/blackbox-pursuit/src/wreck.rs`):

- `Wreck::damage()` (`wreck.rs:146`): the damage so far. It starts at 0, only grows, and is not reset by a repair.
  A wrecked car keeps the value it had.
- `Wreck::is_destroyed()` (`wreck.rs:151`): true once the damage is over 1 (`DAMAGE_EPS` slack), or when a player-caused
  flip wrecks the car at once (`wreck.rs:207-208`).
- `WreckTuning::hit_points` (`wreck.rs:38`): the `HIT_POINTS` of the car's `damagespecs` set. Each hit adds
  `ratio * zone_scale / hit_points` to the damage (`wreck.rs:183`). It is a divisor in impact units. It is **not** a
  maximum health.
- `hit_points` values (`scenes/world/ai/car/hits.rs:32`, table in [ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md)
  section 6): base cop 5, `copghost` 6, `copgto` 7, `copgtoghost` 8, `copsport` 10, the SUV set 10, `copcross` 400.

### 3.2 Which field gives the maximum

The maximum health is **1.0**, a constant. There is no max-health field. `WreckTuning::hit_points` changes how many hits
a car takes, not the scale of the bar. So the bar fraction is:

```
health = clamp(1.0 - wreck.damage(), 0.0, 1.0)        -- fraction of full, 1 = unhurt
```

Consequence for the bar: a flipped cop can be destroyed while its health is still above 0, because a flip is a
destruction, not a damage total. The destruction rule (section 7) must read `is_destroyed()`, never `health <= 0`.
**[verified in the port's code, `wreck.rs:194-212`]**

### 3.3 The mod's value

The reference says the mod reads a "destruction health" value at a fixed offset of each car and draws the bar from it.
It does not give that value's scale, so the port formula above is the one used here. **[community, scale unconfirmed]**

## 4. Health tiers and colours

Five tiers. The mod's tier names are known, and its thresholds are not in the reference. The thresholds and colours below
are port choices.

| Tier | Name (mod) | Lower bound, inclusive | Port colour (RGB) | Source |
|---|---|---|---|---|
| 1 | green | `health >= 0.75` | `(0x40, 0xd0, 0x40)` | [design] |
| 2 | lightGreen | `health >= 0.50` | `(0x9a, 0xe6, 0x5a)` | [design] |
| 3 | yellow | `health >= 0.25` | `(0xf0, 0xd2, 0x3c)` | [design] |
| 4 | orange | `health >= 0.10` | `(0xf0, 0x8c, 0x28)` | [design] |
| 5 | red | `health < 0.10` (includes 0) | `(0xe0, 0x28, 0x28)` | [design] |

- `tier_of(1.0)` is green, `tier_of(0.75)` is green, `tier_of(0.7499)` is lightGreen, `tier_of(0.0)` is red.
- The lagging segment uses its own colour, `LAG_COLOUR = (0xf4, 0xf4, 0xf4)`. **[design]** The mod names it
  `healthAnimationColor` and gives no value in the reference.
- The border colour is `BORDER_COLOUR = (0x10, 0x10, 0x10)`. A destroyed car's border, and the flash colour, is
  `BORDER_DESTROYED_COLOUR = (0x60, 0x10, 0x10)`. **[design]** The mod names both as `borderColor` and
  `borderColorDestroyed`.
- All colours are multiplied by `BAR_ALPHA` (section 5) and by the fade of section 6.
- The mod's exact RGB values are not in the reference, and this spec does not look them up. That is a choice for the
  user (see the provenance file). **[unconfirmed]**

## 5. Bar size, position and alpha

| Constant | Port name | Mod value (reference) | Port value | Tag |
|---|---|---|---|---|
| Bar length | `BAR_WIDTH` | 1.4 | 1.4 m | [community] |
| Bar height | `BAR_HEIGHT` | 0.13 | 0.13 m | [community] |
| Lift above the car | `BAR_LIFT` | 1.15 | 1.15 m, along world up (z) | [community] |
| Opacity of all colours | `BAR_ALPHA` | 0.7 | 0.7 | [community] |
| Border thickness | `BORDER_WIDTH` | not given | 0.02 m | [design] |
| `hideOnPause` | (not a setting) | `false` | `false`: the bars stay visible while the world is paused | [community] |

Layout, in world space, with `centre = car_position + Z * BAR_LIFT`, `right` = the camera's right vector
(`forward.cross(Z)`, normalised, as `car_light_flares.rs` takes it), and `up = Z`:

1. **Border:** a quad of `BAR_WIDTH x BAR_HEIGHT`, centred on `centre`, in `BORDER_COLOUR`.
2. **Inner area:** the border inset by `BORDER_WIDTH` on each side. The fill and lagging segments are laid inside it,
   starting at its left end.
3. **Fill:** from the left end, `inner_width * health` long, in the tier colour (or the flash colour, section 7).
4. **Lagging segment:** from the end of the fill to `inner_width * lag`, in `LAG_COLOUR`. It is empty when `lag <= health`.

The quads sit in the order border, lag, fill, so the fill draws over the lag where they meet.

## 6. Lagging damage value

The lagging value is the pale segment. When a hit takes health away, the segment stays at the old level for a short
time, then falls to the new level. The reference gives the timing, and the reset rule is not given. **[community]**
for the constants, **[design]** for the reset rule.

| Constant | Port name | Mod value | Port value |
|---|---|---|---|
| Countdown before the drain | `LAG_HOLD_SECS` | 0.5 s | 0.5 s |
| Drain rate, fraction of a full bar per second | `LAG_DRAIN_RATE` | 0.75 | 0.75 |

Rule, per cop, once per simulated step, with `health` the current value and `last_health` the value of the step before:

1. If `health > lag`, the lag jumps up to `health` and the countdown is cleared. The port's damage never goes down,
   so this only happens on the first step.
2. If `health < last_health` (a new drop), the countdown restarts at `LAG_HOLD_SECS`. **[design: each new drop restarts it]**
3. If the countdown is above 0, subtract `dt` from it and leave the lag where it is.
4. Otherwise, if `lag > health`, the lag falls by `LAG_DRAIN_RATE * dt`, but not below `health`.
5. Store `last_health = health`.

A cop that is never hit has `lag == health == 1.0`, and the lagging segment is empty.

## 7. Death: flash, then fade

Death means `Wreck::is_destroyed()` (section 3.2). The death time is recorded in simulated seconds on the first step
where it is true. The displayed health of a destroyed cop is 0, and its lag follows section 6 down to 0. **[design]**

| Constant | Port name | Mod value | Port value | Tag |
|---|---|---|---|---|
| Flash period | `FLASH_PERIOD_SECS` | 0.05 s | 0.05 s | [community: the period is given, whether it is the full cycle is not] |
| Fade starts | `DEATH_FADE_START_SECS` | about 1.0 s | 1.0 s | [community] |
| Fade ends and the bar is removed | `DISABLED_HIDE_SECS` | 2.0 s (disabled hide, section 8) | 2.0 s | [community] |

With `dead_for = now - died_at`:

- **Flash** (`dead_for < 1.0`): the fill and the border take turns between the tier colour and `BORDER_DESTROYED_COLOUR`,
  switching every `FLASH_PERIOD_SECS`. The fill is empty, so the flash reads as the border. `flash_on = floor(dead_for /
  FLASH_PERIOD_SECS) % 2 == 0`. **[unconfirmed: period taken as the switch interval]**
- **Fade** (`1.0 <= dead_for < 2.0`): the whole bar's alpha goes from 1 to 0 linearly. The fade length, 1.0 s, is inferred
  from the timing of the mod: the fade starts at about 1 s and the bar is hidden at 2 s. **[unconfirmed inference]**
- **Gone** (`dead_for >= 2.0`): nothing is drawn, and the state is kept until the cop is no longer sampled.

## 8. Disabled cars

The reference hides a car "that has been disabled" after 2.0 s of disabled time. The reference does not say what
"disabled" means in the car's state. In the port, the only such state is destruction. So the port's rule is:

- `disabled := is_destroyed()`, and the disabled time is `dead_for` (section 7).
- The bar is hidden at `dead_for >= 2.0`.

The mod's "disabled" may mean something other than destruction. **[unconfirmed, open question 1]**

## 9. Distance fade

The mod fades a bar in near the camera and out far from it. The reference gives the two ranges, not the curve. The
curve here is linear. **[community for the ranges, design for the shape]**

| Constant | Port name | Mod value |
|---|---|---|
| Near fade starts (alpha 0) | `NEAR_FADE_MIN` | 2 m |
| Near fade complete (alpha 1) | `NEAR_FADE_MAX` | 5 m |
| Far fade starts (alpha 1) | `FAR_FADE_MIN` | 70 m |
| Far fade complete (alpha 0) | `FAR_FADE_MAX` | 110 m |

```
d       = distance from the camera position to the bar's centre
near    = clamp((d - NEAR_FADE_MIN) / (NEAR_FADE_MAX - NEAR_FADE_MIN), 0, 1)
far     = clamp((FAR_FADE_MAX - d)  / (FAR_FADE_MAX - FAR_FADE_MIN), 0, 1)
distance_alpha = near * far
```

The final alpha of a bar is `BAR_ALPHA * distance_alpha * death_alpha`, where `death_alpha` is 1 before the death, the
fade value of section 7 after it, and 0 after the bar is gone.

## 10. Visibility gates

A bar is drawn only when every gate is open. Each gate is a plain boolean, so the rule is testable.

| Gate | Open when | Port source | Tag |
|---|---|---|---|
| Chase on | `TrafficWorld::is_pursuing()` | `ai/mod.rs:197` | [community] |
| Not in a pursuit shot | no caught pan and no arrest framing is running | a new `PursuitCamera::shot_running()` (section 12.2) | [community: the reference's "freeze cam"; design: the port's own shots] |
| HUD shown | `HudState::visible` is true, after the look-back rule | `scenes/world/mod.rs:449-456`, the same rule the HUD uses | [design] |
| Scene has a car | the drive exists | `upload_effects` returns early without one (`effects/world.rs:20-23`) | [design] |

The original camera's freeze cam (the mod reads a flag in a jump camera structure) is an original cutscene. The port's
`PursuitCamera` does not read those tracks (`viewer/camera/pursuit/mod.rs:2`). The port's catch pan (2.5 s,
`CATCH_SECS`) and arrest framing (3.0 s, `ARREST_SECS`) are the closest match. **[design]**

The HUD gate follows the rule the rest of the HUD uses. With look-back on and `show_hud_on_look_back` off, the scene
returns no HUD (`scenes/world/mod.rs:449-456`), so the bars hide as well.

`hideOnPause` is `false` in the mod, so the bars stay up when the pause menu freezes the world. The port has no such
gate. **[community]**

## 11. Draw order and layer

- **Order.** Back to front. For each bar, `depth = dot(centre - camera_position, camera_forward)`. The bars are sorted by
  depth, largest first. Equal depths are ordered by cop key, so the order is the same every frame. **[community: the mod
  sorts by view depth, nearer on top]**
- **Layer.** The bars are world-space quads in `EffectLayer::textured` as one `TexturedEffect` with
  `BlendMode::AlphaBlend`, in the sorted order. **[design]** The reasons are in section 13.2.
- **Depth.** The effect layers are depth tested and do not write depth (`libs/blackbox-gfx/src/effects.rs:3`). The mod
  also tests depth and does not write it (its device state, reference section 3.4). So a bar behind a wall is hidden in both.
- **Under the HUD.** The effects draw before the HUD, so the FEng gauges and the pursuit board sit on top of the bars.
  The mod draws the bars inside its GUI hook, so there the two are drawn together. **[design difference]**

## 12. Settings

### 12.1 The setting

| Surface | Name | Value | Default |
|---|---|---|---|
| TOML config | `cop_health_bars` (top level, like `traffic_lights`) | `true` / `false` | `false` |
| Environment | `NFSMW_COP_HEALTH_BARS` | `on` / `off` (as `NFSMW_TRAFFIC_LIGHTS`) | not set |
| Command line | `--cop-health-bars` / `--no-cop-health-bars` | flag pair | not set |
| Console | `set cop_health_bars on`, alias `cop-health-bars`; `get` lists it | `on` / `off` | `off` |
| Options menu | row "Cop Health Bars", on / off | toggled with `toggle!` | off |

The default constant is `pub const DEFAULT_COP_HEALTH_BARS: bool = false;` in `settings/mod.rs`, beside
`DEFAULT_TRAFFIC_LIGHTS`. It is not in a new module, because a boolean needs no more than that.

### 12.2 Where each surface goes

The `traffic_lights` boolean is the pattern. Every line below has a matching line for it:

| File | Change |
|---|---|
| `crates/nfsmw/src/settings/mod.rs` | `DEFAULT_COP_HEALTH_BARS` (near line 112); field `cop_health_bars: bool` in `Settings` (near 185); default from `Partial` (near 306) |
| `crates/nfsmw/src/settings/partial.rs` | `pub cop_health_bars: Option<bool>` (near 91); layering line (near 172) |
| `crates/nfsmw/src/settings/file.rs` | `cop_health_bars: field(&table, origin, "cop_health_bars", boolean)` (near 100) |
| `crates/nfsmw/src/settings/env.rs` | `pub const COP_HEALTH_BARS: &str = "NFSMW_COP_HEALTH_BARS";` (near 38) and the `value(&get, ..., parse_bool)` line (near 114) |
| `crates/nfsmw/src/cli/view_args.rs` | flag pair with `switch(..)` (near 162-167 and 297) |
| `crates/nfsmw/src/settings/write.rs` | `put("cop_health_bars", Value::Boolean(v))` (near 102) |
| `crates/nfsmw/src/saves.rs` | copy of the setting (near 157) |
| `crates/nfsmw/src/devtools/console/settings_keys.rs` | a key in `KEYS`. The array is typed `[&str; 59]`, so its length becomes 60. |
| `crates/nfsmw/src/devtools/console/settings_cmd.rs` | `get` (near 46), `set` (near 123), and the flip target (near 180) |
| `crates/nfsmw/src/devtools/console/parse.rs` | the key list (near 115) |
| `crates/nfsmw/src/frontend/options.rs` | `Setting::CopHealthBars` in the enum (near 43); `data` (`on_off`, near 224); `step` with `toggle!` (near 441) |
| `crates/nfsmw/src/frontend/options_rows.rs` | `row(Setting::CopHealthBars, Title::Text("Cop Health Bars"))` next to the rear-view mirror row (near 60) |

`options.rs` is 485 lines. That is close to the 500-line limit, so the new `Setting` arms should go into the existing
toggle group, or the menu part should move to a sub-module first. `settings/mod.rs` is 431 lines.

### 12.3 Behaviour of the setting

- The setting only decides whether the bars are drawn. The state of section 12.4 is kept in either case, so turning the
  bars on mid-chase shows the current health, not a fresh bar.
- Changing the setting in the menu applies on the next frame. The scene reads it from its tuning (`self.tuning`, which
  already carries `turn_signals`), so the same path that refreshes the tuning must refresh this flag. **[verify where
  the tuning is rebuilt when a setting changes]**

### 12.4 Bar state

The bar state is kept per cop, keyed by a serial. A serial is needed because `TrafficWorld::cars` is a `Vec` that cops
leave by `retain`, so an index is not stable. `AiCar` has no id today, so the serial is a new field, `serial: u32`,
set from a counter on `TrafficWorld` in the cop spawn path (`ai/pursuit.rs:180`, `spawn_cop`). **[design]** Each frame,
state for a key that is not sampled is dropped, as the mod drops map entries of cars that left its table.

## 13. The data path from the world to the bars

### 13.1 Why not `HudState`

`HudState` is the HUD's plain numbers, which the FEng presenter binds to the package (`hud/state.rs`; ADR
[0002](../decisions/0002-ui-presentation.md), "game code knows `HudState` only"). A bar is a 3-D object placed in the world,
so it has the same nature as the car lights and the cop light bars. Those reach the screen through the scene's effect
layer (`scenes/world/effects/world.rs`), not through `HudState`. The bars follow that path. `HudState` gets no field.

The "world-to-screen overlay" precedent is `car_light_flares.rs`. It does not project to the screen. It builds
camera-facing quads in world space, and sizes them with `focal_scale(height, fov)` (`car_light_flares.rs:126`). The bars
take the same route and use world units, as the mod does. A screen-space route (egui painter with `Lens::view_proj()`,
`viewer/camera/mod.rs:49`) would work, but it has no depth test, and the mod has one. So it was not chosen.

### 13.2 The chain

| Step | Where | What |
|---|---|---|
| 1. Damage is read | `scenes/world/ai/car/hits.rs`: a new `AiCar::cop_health_sample(&self) -> Option<CopHealthSample>` | Returns `Some` for `Role::Cop(_)` only, with `damage` (`CopDamage::wreck.damage()`), `destroyed` (`wreck.is_destroyed()`), `position` (render space) and `key` (the serial). `CopDamage` is `pub(super)` and `wreck` is private, so this accessor goes in `hits.rs`, where `CopDamage` is defined. `is_wrecked()` must not be used, and neither should `take_wreck()`, which is consumed by the cops-wrecked counter. |
| 2. Samples are gathered | `scenes/world/ai/mod.rs`, next to `map_blips` (line 244): `TrafficWorld::cop_health_samples(&self, out: &mut Vec<CopHealthSample>)` | Walks `self.cars` and skips anything `cop_health_sample` refuses. Helicopters are never in `cars`, so there is nothing to skip for them. |
| 3. State steps | `scenes/world/simulation.rs`, after `traffic.update_police(steps as f32 * STEP, ..)` (line 90) | Runs once per simulated step with `dt = STEP`, and gives the same sim-time clock as the police. It runs whether or not the bars are shown, as the mod's update runs regardless of drawing. Each step, the state takes the samples, updates the lag and death timers, and drops the state of keys that were not sampled. |
| 4. Bars are built | `scenes/world/cop_health.rs` (new): `CopHealthBars::bars(&self, camera: &Lens) -> Vec<CopHealthBar>` | Applies the distance fade and death alpha, and sorts back to front. Returns nothing when a gate of section 10 is shut. |
| 5. Quads are emitted | `scenes/world/effects/world.rs`, inside `upload_effects`, beside the flares (line 38) | Each `CopHealthBar` becomes its quads (section 5) in `layer.textured`. The gates are read here: the setting, `is_pursuing()`, the shot, and the HUD flag. |

### 13.3 Proposed types

Pure data, engine-generic, in `libs/blackbox-pursuit/src/health_bar/` (a folder, so no file goes over 500 lines):

```rust
/// One cop as the world sees it this step. No NFS:MW names: `key` is any stable id.
pub struct CopHealthSample {
    pub key: u32,
    pub position: Vec3,   // render space, metres
    pub damage: f32,      // Wreck::damage()
    pub destroyed: bool,  // Wreck::is_destroyed()
}

/// One bar, ready to draw: plain fractions and a depth. The renderer and colours are not in here.
pub struct CopHealthBar {
    pub centre: Vec3,
    pub depth: f32,        // view depth, for the sort
    pub health: f32,       // 0..=1, 0 once destroyed
    pub lag: f32,          // 0..=1, >= health
    pub alpha: f32,        // BAR_ALPHA * distance fade * death alpha
    pub flash_on: bool,    // the fill shows the flash colour
    pub destroyed: bool,
}
```

Per cop state, in `crates/nfsmw/src/scenes/world/cop_health.rs`: a `HashMap<u32, CopHealthTrack>` (or a `Vec` sorted by
key) of `{ lag: LagTracker, last_health: f32, died_at: Option<f32> }`. The lag tracker is in libs; the colour tables and
the `TexturedEffect` assembly are in `crates/nfsmw/src/hud/cop_health/` (colours.rs, quads.rs), next to the other HUD
glue, so the rules stay in `libs/`.

### 13.4 Texture

The bars need one opaque 1 x 1 white texture, so that `TexturedEffect` (which always has a texture) can draw flat colour.
Create it once with `RenderBackend::create_texture(&TextureDesc)` (`libs/blackbox-gfx/src/backend.rs:65`), in the same
place `CarLightFlares::upload` runs. **[design; check the `TextureDesc` fields]** The bars are not textured with an image,
so the white texture only carries the colour the vertex supplies.

## 14. Rules that stay in `libs/`

Engine-generic, no names of the game, and testable without a renderer (`libs/blackbox-pursuit/src/health_bar/`):

- `health_fraction(damage) -> f32` (section 3.2)
- `tier_of(health) -> Tier` and the five-tier enum (section 4)
- `LagTracker` with `step(health, dt) -> lag` (section 6)
- `death_phase(dead_for) -> Death { Flash { on }, Fade { alpha }, Gone }` (section 7)
- `distance_alpha(d) -> f32` (section 9)
- `shows(pursuing, shot, hud_visible) -> bool` (section 10)
- `bar_corners(centre, right, up, width, height) -> [Vec3; 4]` and the fill and lag widths (section 5)
- `sort_back_to_front(&mut [CopHealthBar])` (section 11)

## 15. Constants in one place

| Name | Value | Unit | Tag |
|---|---|---|---|
| `BAR_WIDTH` | 1.4 | m | [community] |
| `BAR_HEIGHT` | 0.13 | m | [community] |
| `BAR_LIFT` | 1.15 | m, up | [community] |
| `BAR_ALPHA` | 0.7 | 0..1 | [community] |
| `BORDER_WIDTH` | 0.02 | m | [design] |
| `NEAR_FADE_MIN` / `NEAR_FADE_MAX` | 2 / 5 | m | [community] |
| `FAR_FADE_MIN` / `FAR_FADE_MAX` | 70 / 110 | m | [community] |
| `LAG_HOLD_SECS` | 0.5 | s | [community] |
| `LAG_DRAIN_RATE` | 0.75 | per s | [community] |
| `FLASH_PERIOD_SECS` | 0.05 | s | [community] |
| `DEATH_FADE_START_SECS` | 1.0 | s | [community] |
| `DISABLED_HIDE_SECS` | 2.0 | s | [community] |
| `TIER_THRESHOLDS` | 0.75, 0.50, 0.25, 0.10 | fraction | [design] |
| tier, lag, border, destroyed colours | section 4 | RGB | [design] |
| `DEFAULT_COP_HEALTH_BARS` | false | | [design, asked for] |
| `hideOnPause` | false | | [community] (not a setting) |
| max health | 1.0 | fraction | [verified in the port] |

## 16. Test list

Pure functions, in `libs/blackbox-pursuit/src/health_bar/` tests:

1. `health_fraction`: damage 0 gives 1; 0.4 gives 0.6; 1.2 gives 0; a negative damage gives 1.
2. `tier_of`: 1.0 and 0.75 give green; 0.7499 gives lightGreen; 0.5, 0.25 and 0.10 each start their tier; 0.0 gives red.
3. `LagTracker` with no damage: the lag equals the health at every step.
4. `LagTracker` after one drop from 1.0 to 0.6: the lag holds at 1.0 for 0.5 s, then falls 0.75 per second, and stops at 0.6.
5. `LagTracker` with a second drop during the countdown: the countdown restarts from 0.5 s.
6. `death_phase`: `flash_on` is true at 0.0 and 0.049 and false at 0.05 (the switch interval is 0.05 s); `Fade` starts at
   exactly 1.0 with alpha 1, is at alpha 0.5 at 1.5, and reaches alpha 0 at 2.0; `Gone` from 2.0 on.
7. `distance_alpha`: 2 m gives 0; 3.5 m gives 0.5; 5 m and 70 m give 1; 90 m gives 0.5; 110 m and beyond give 0.
8. `shows`: the truth table over pursuing, shot and HUD flags. Only the all-true row draws.
9. `sort_back_to_front`: the farther bar comes first; equal depths keep the given order; the sort is in place.
10. `bar_corners`: a bar facing the camera along +x with `right = -y` puts its corners at the expected offsets; the fill
    width is `inner * health` and the lag segment is `inner * (lag - health)`.
11. Gathering: `cop_health_sample` gives `Some` for a `Role::Cop` car, including a wrecked one, and `None` for a traffic car
    and for a patrol cop.
12. Helicopter: a helicopter sample is never produced (no helicopter is in `cars`).
13. Keys: a cop that is no longer sampled has its state dropped; a sampled cop keeps its state.
14. Flipped cop: a `Wreck` destroyed by a player-caused flip has `health > 0` and `destroyed == true`, and the bar shows the
    death, not a zero-health bar that never flashes.
15. Settings: the default is `false`; `cop_health_bars = true` in the file parses; `NFSMW_COP_HEALTH_BARS=on` parses; the
    CLI pair sets it; the console `get` shows `off` and `set ... on` changes it; the menu row toggles it and reports the
    change; the `KEYS` array has 60 entries.
16. Integration (in the world scene tests, with a fake traffic world): a chase with hits gives decreasing `health` and a
    lag that holds; the bars are not in the layer when the shot is running or the HUD is hidden.

## 17. Open questions

1. What "disabled" means in the mod (section 8). Destruction is the only candidate in the port.
2. The mod's exact colour values and tier thresholds. They are not in the reference. Getting them means reading the
   repository's source, which the reference was written to avoid. The user decides whether that is worth it.
3. Whether the flash period is the full cycle or the switch interval (section 7).
4. Whether the fade lasts 1 s, as inferred (section 7).
5. Whether a wrecked cop should keep its bar after the chase ends. This spec says no, because the gate needs a chase.
6. The unit of the mod's distance ranges and of its height (section 5).
7. Whether `BAR_LIFT` is measured from the car's origin or from its roof. The mod's origin rule is not in the reference,
   and the port's car position convention needs a check.
8. Whether roadblock cars (`Role::Traffic`, placed by `ai/roadblock.rs`) should get a bar. The mod's table is the cop
   cars, and a roadblock car is a cop car, but it is not chasing. This spec gives them none.

## Port status

What is in the tree, and what is not yet wired.

- **Rules.** `libs/blackbox-pursuit/src/health_bar/` (sections 3 to 11 and 14). Pure, tested.
- **Samples.** `AiCar::cop_health_sample` (`scenes/world/ai/car/hits.rs`) gives a sample for `Role::Cop` only, wrecked
  ones included. `TrafficWorld::cop_health_samples` and `TrafficWorld::cop_health_bars` (`scenes/world/ai/cop_health.rs`)
  gather the samples and step the bars.
- **Key.** The key is `AiCar::render_id`, not a new `serial` field (section 12.4). It comes from `keys::next_car()`, is
  unique per placed car, and does not change while the car is alive, so `car.rs` and `pursuit.rs` are not edited.
- **Quads.** `scenes/world/effects/cop_health.rs`: `CopHealthBars` holds the bar state, the last step's bars and the
  white texture (`TextureDesc`, 1 x 1, `Rgba8`, created on first use). Each bar is emitted as `segment_quads` in one
  `TexturedEffect` with `BlendMode::AlphaBlend`, from `upload_effects`. The vertex colours are sRGB bytes: the effect
  shaders decode them on an sRGB target, so the linear rule colours are encoded back on the way out.
- **Setting.** `cop_health_bars`, default off, in `Settings` and `WorldTuning`: TOML, `NFSMW_COP_HEALTH_BARS`,
  `--cop-health-bars` / `--no-cop-health-bars`, console `get` / `set` / flip, and the Gameplay options row "Cop Health
  Bars". The console `KEYS` array is `[&str; 65]`. The setting only gates the emit; the state keeps stepping.
- **Wired.** The per-step call is in `scenes/world/simulation.rs`, after `update_police`, with the sim `dt` (section
  13.2 step 3). The gates are built there: the shot gate from `Drive::pursuit_shot_running()` (`PursuitCamera::shot_running`,
  `viewer/camera/pursuit/mod.rs`), and the HUD gate from `WorldScene::hud_shown()` (`scenes/world/hud.rs`), which applies
  the look-back rule without building the HUD frame each step.
- **Not checked by eye.** The colours, thresholds, flash period and fade are the design values of sections 4 to 7 and
  have not been seen in a run.

## Next leads

1. Play the port with the bars on and check the tier thresholds and the flash period by eye, then change the design
   values in this spec.
2. Check the port's car position: whether `AiCar::position()` is the centre of the body or the ground, for `BAR_LIFT`.
3. Find where `TrafficWorld`'s tuning is rebuilt on a settings change, so that the new flag follows the same path
   (section 12.3).
4. Check `TextureDesc` for the 1 x 1 white texture (section 13.4).
5. Read the cops' `Role::Cop` spawn path for the serial (section 12.4). The spawn path is `ai/pursuit.rs:180`.
6. Check whether the original game ever draws a per-car bar. The HUD dumps so far show aggregate counters only. A
   Ghidra search for a per-vehicle HUD element was not run, because the port's damage rules already cover the numbers
   this spec needs.
