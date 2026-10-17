# Provenance: cop health bars

Where each value and rule in [specs/cop-health-bars.md](../specs/cop-health-bars.md) comes from, what was read, and what
was not checked. Nothing was built, run or launched. No Ghidra session was opened for this spec.

## Sources

| Source | Used for | Tag in the spec |
|---|---|---|
| [docs/mods/mwhealthbars.md](../mods/mwhealthbars.md) (sections 2, 3.3, 4, 5.3, 6) | The mod's features, timers, fade ranges, the size and lift constants, the height and alpha values, and the list of colour names | [community] |
| The mod's own repository | **Not read again for this spec.** The reference page is the only source. Its colour RGB values and tier thresholds are not on the page, so they are not recorded here. | none |
| `libs/blackbox-pursuit/src/wreck.rs` (lines 12-32, 36-45, 146-212) | The damage model: `damage()`, `is_destroyed()`, `impact`, `flip`, `WreckTuning::hit_points`, the constants | [verified] / [decomp] via the cop spec |
| `crates/nfsmw/src/scenes/world/ai/car/hits.rs` (lines 12-98) | `CopDamage`, `tuning_for` (the per-model numbers), `wreck_hit`, `is_wrecked`, `take_wreck` | [verified] |
| `crates/nfsmw/src/scenes/world/ai/car.rs` (lines 112-114, 282, 307-309) | `AiCar::dim`, `damage`, `is_cop`, the `debug` line that tells cop from patrol | |
| `crates/nfsmw/src/scenes/world/ai/mod.rs` (lines 90-98, 197-278) | `cars`, `is_pursuing`, `map_blips` (the cop-blip precedent), `cop_particles`, `car_glows` | |
| `crates/nfsmw/src/scenes/world/ai/pursuit.rs` (lines 94-105, 126-137, 180) | `end_pursuit` and its `retain`, `update_pursuit` and its `retain`, `spawn_cop` | |
| `crates/nfsmw/src/scenes/world/ai/roadblock.rs` (line 107) | Roadblock cars placed as `Role::Traffic` | |
| `crates/nfsmw/src/scenes/world/ai/police/world.rs` (lines 109-203) | `evade_level`, `cops_destroyed`, `pursuit_feed`, `nearest_cop`, `helicopter_blips` | |
| `crates/nfsmw/src/scenes/world/simulation.rs` (lines 40-134) | The per-frame order: `update_sight`, `speeding_pursuit`, `update_police` (line 90), `update_pursuit` on the drive | |
| `crates/nfsmw/src/scenes/world/hud.rs` (lines 22-47) | How `HudState` is assembled, and the `visible` flag | |
| `crates/nfsmw/src/scenes/world/mod.rs` (lines 449-456) | `hud_state`: the look-back rule (`show_hud_on_look_back`) | |
| `crates/nfsmw/src/scenes/world/effects/world.rs` (lines 17-75) | `upload_effects`: where flares and cop particles reach the effect layer | |
| `crates/nfsmw/src/scenes/world/car_light_flares.rs` (lines 61-132) | The world-space billboard precedent, and `focal_scale` | |
| `crates/nfsmw/src/viewer/camera/mod.rs` (lines 22-75) | `Lens`, `Lens::view_proj`, `projection` | |
| `crates/nfsmw/src/viewer/camera/pursuit/mod.rs` (lines 14-19, 48-151) | `CATCH_SECS` 2.5, `ARREST_SECS` 3.0, `ARREST_BLEND_SECS` 0.6, `shot`, `current_shot`, the `catch` and `arrest` fields | |
| `crates/nfsmw/src/hud/state.rs`, `hud/pursuit.rs`, `hud/cop_blip.rs` | `HudState` fields, the pursuit board, the cop icon colour (no per-car bar exists in the HUD) | |
| `libs/blackbox-gfx/src/effects.rs` (lines 1-84), `mesh.rs` (lines 17-25), `backend.rs` (line 65), `libs/blackbox-scene/src/textures.rs` (line 7) | Effect layers, depth test, `BlendMode`, texture creation | |
| `crates/nfsmw/src/settings/mod.rs`, `partial.rs`, `file.rs`, `env.rs`, `write.rs`, `saves.rs`, `cli/view_args.rs`, `devtools/console/settings_keys.rs`, `settings_cmd.rs`, `parse.rs`, `frontend/options.rs`, `options_rows.rs` | The `traffic_lights` and `rear_view_mirror` boolean paths, every surface the new setting needs | |
| `crates/nfsmw/src/settings/fairbusts.rs`, `radio_hud.rs` | The settings module pattern, for comparison (the bool needs no module) | |
| `docs/decisions/0002-ui-presentation.md` | The rule that game code knows `HudState` only, which decides against a `HudState` field | |
| `docs/specs/ai-pursuit-cop-cars.md` (section 6) and `docs/provenance/ai-pursuit-cop-damage.md` | The per-car damage numbers, the flip rule, and the aggregate HUD counters the original has | [decomp] / [verified] |

## Values checked against the port's code

- `WreckTuning::hit_points` is a divisor of impact units, not a maximum health. The maximum is 1.0, because
  `Wreck::is_destroyed` triggers at damage over 1 (`DAMAGE_EPS` slack). Read in `wreck.rs:146-188`.
- `Wreck::flip` can destroy a car at once while its damage is below 1 (`wreck.rs:207-208`). So the death rule reads
  `is_destroyed()`, not a health threshold.
- `AiCar::is_cop()` is false for a wrecked cop (`car.rs:308`). A membership test built on it would never show a dead cop.
- `end_pursuit` keeps wrecked cops (`pursuit.rs:103`, `retain(|c| !c.is_cop())`), and `update_pursuit` keeps them too
  (`pursuit.rs:134-137`). So a wrecked cop stays in `cars` after the chase. The bar gate (`is_pursuing`) hides it then.
- `AiCar` has no identifier. `name` is not unique (the same model can spawn many times), so a serial is proposed.
- `upload_effects` already builds the flare and cop-particle quads from the camera (`effects/world.rs:38-49`,
  `ai/mod.rs:260`). The bars use the same entry point.
- The effect layers are depth tested and write no depth (`effects.rs:3`), and `TexturedEffect` is caller-ordered. Both
  fit the mod's behaviour (depth test on, depth write off, caller sort).
- The original's HUD has no per-car bar. `provenance/ai-pursuit-cop-damage.md` lists `Cops_TakenOut` and
  `Cops_Damaged` as aggregate icons, and `hud-cop-blips.md` covers the minimap icons only.

## Values from the mod reference, unconfirmed

- The fade ranges (2 to 5, 70 to 110), the lag drain (0.75 per second), the hold (0.5 s), the flash period (0.05 s), the
  disabled hide (2.0 s), the death fade start (about 1 s), the size and lift (1.4, 0.13, 1.15), and the alpha (0.7) all
  come from the reference page. They were not checked against the mod's source or against play.
- "Disabled" is read as destruction, because the port has no other disabled state. The mod's meaning is not in the
  reference. **[unconfirmed]**
- The mod's unit is taken to be one metre. **[unconfirmed]**

## Design values (not in the reference)

- Tier thresholds 0.75, 0.50, 0.25, 0.10, and the five RGB colours, the lag colour, the border colour, the destroyed
  border colour. The reference names the colours and gives no values.
- The border thickness, 0.02 m.
- The death fade length, 1.0 s. This is inferred from the mod's timing (the fade starts near 1 s and the bar is hidden at
  2 s). **[unconfirmed inference]**
- The linear shape of both distance fades.
- The lag reset rule: each new drop restarts the countdown.
- The HUD gate, and hiding the bars with the HUD and during the port's caught pan and arrest shots. The mod's freeze cam
  is an original cutscene flag; the port has no such flag, and its own shots are the closest match.
- Keeping a bar's state while the setting is off.

## Not done, and why

- **No Ghidra pass.** The spec needs the port's damage numbers, and those are already [decomp] and [verified] in the cop
  spec. An original-game check (whether any per-car bar exists in `speed.exe`) is a lead in the spec, not a blocker.
- **No mod source read beyond the reference page.** The page is the agreed reference (licence: none). Colour RGB values and
  tier thresholds are therefore not recorded. If the user wants them, that is a separate decision.
- **No build, no test run, no game launch.** This is a docs-only task.
- **`CopDamage` is `pub(super)` and `wreck` is private.** The accessor the spec proposes goes into `hits.rs`. The
  proposal is not implemented here.

## Lessons

- Read the state a rule depends on before writing the rule. The obvious membership test (`is_cop()`) hides the cops the
  death effect is for. The role check is the right test.
- A "health" that is really a damage total needs the fraction written out. Here the port has no max-health field, so
  the spec states the maximum (1.0) and says where the damage is.
- Destruction and health are separate facts in the port. A flip can destroy a car with health left, so an effect keyed to
  health alone would miss it.
- Look for the existing world-space path before reaching for a screen-space one. The flare code already does
  camera-facing quads, and it needs no projection. The projection function is only needed for a screen-space route.
- When the reference gives a name and not a value (the colours) or a range and not a curve (the fades), write the
  choice down as a design value and list it. That keeps the spec honest about what the mod gave and what was chosen.
