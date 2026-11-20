# Parity status

One row per gameplay system. The page is an index: every cell repeats what a linked doc or the code says, and
"not recorded" means the docs read for this page say nothing.

The three status columns mean different things:

- **Implemented:** the code exists. It says nothing about the original.
- **Compared with the original:** what evidence the docs record of a comparison with the original game, and of what kind
  (isolated routines run offline, data read from the install or the executable, a recorded session, a comparison in a
  running game).
- **Playtested:** what the docs record of a person playing the rewrite.

A row counts as matching the original only when the comparison column has evidence, and only for what that evidence
covers. No row below has a comparison of the whole system in a running game.

**Known rewrite-own rules** is the number of lines under `crates/` and `libs/` that carry an `[ours]` tag, attributed by
directory (counted 2026-11-17: 99 lines, 100 tags). It counts tagged rules only; an untagged rewrite choice is not in it.
23 more tagged lines are in the multiplayer and co-op network code (`crates/nfsmw-net`, `scenes/world/net/`,
`ai/net_export.rs`, `players/step.rs`), which has no row here.

## Systems

Code paths without a crate are under `crates/nfsmw/src/`.

| System | Implemented | Compared with the original | Playtested | Known rewrite-own rules | Detail docs |
|---|---|---|---|---|---|
| Vehicle physics | Yes for the player car: `libs/blackbox-vehicle`, `scenes/world/drive/`. Not built: the drift handling variant and the dragster engine, damage and input behaviours ([roadmap](roadmap.md)) | Offline function-level cases: 38,764 tire calls of two original sessions replayed (residual force differences remain, no tolerance set), 52 one-step engine cases and four 16-tick sequences match, wheel-helper and spring fixtures. No matched-route trajectory comparison | The user reports the latest candidate works well: qualitative, not a matched trace. Flat-tire handling deferred | 0 | [player-driving-status.md](player-driving-status.md), [driving-state-audit.md](driving-state-audit.md) |
| Collisions and props | Partial: `libs/blackbox-collision`, `scenes/world/props/`, `scenes/world/ai/collide/`. Open items in the todo | Offline cases: 5,012 isolated PC box-contact cases match; plane response and swept contacts against measured PC vectors. Live comparison pending in every ledger row | Open user reports: fences that return too quickly, parked cars that feel too light, light player-car collision feel | 0 | [world-interactions.md](world-interactions.md), [world-interactions-todo.md](world-interactions-todo.md), [native-physics-parity.md](native-physics-parity.md), [traffic-interactions-audit.md](traffic-interactions-audit.md) |
| Traffic AI | Yes: `scenes/world/ai/traffic/`, `scenes/world/ai/population/`, `libs/blackbox-roads`, `libs/blackbox-driver`. Traffic lights are a rewrite extension | Static PC analysis and isolated original execution for single rules. Matched-route driving and retirement validation are open; mid-road lane-change triggers unverified | Not recorded | 8 | [ai-drivers.md](ai-drivers.md), [specs/traffic-driving-parity.md](specs/traffic-driving-parity.md), [specs/traffic-presets.md](specs/traffic-presets.md) |
| Police and pursuit | Partial: `libs/blackbox-pursuit`, `scenes/world/ai/` (`police/`, `pursuit/`, `cop_manager/`, `roadblock/`, `support/`), `hud/pursuit_meter.rs`. The docs call it an incomplete integration | Static and instruction-level reads; twelve isolated executions of the original meter routine; a private recorded PC pursuit (heats 1 to 5) used for corrections and for roadblock request rates. No matched original/remake pursuit; the recorded-target city stress fails | Yes, with open failures: catch-up, driving and formations, spike strips and roadblock placement, minimap icons, helicopter deployment | 20 | [pursuit-system.md](pursuit-system.md), [police-ai-parity.md](police-ai-parity.md), [police-ai-driving-handoff.md](police-ai-driving-handoff.md), [provenance/ai-pursuit-roadblocks.md](provenance/ai-pursuit-roadblocks.md) |
| Racer AI | Yes: `scenes/world/ai/racer_route.rs`, `scenes/world/ai/car/racing.rs`, `scenes/world/race/shortcuts.rs`, `scenes/world/race/catch_up.rs`. The stock cars of the street rivals are a stable pick, not the game's | None recorded in the roadmap or the race spec; the catch-up spec points to its own provenance record | Not recorded | 0 | [specs/ai-racers.md](specs/ai-racers.md), [specs/ai-racers-route.md](specs/ai-racers-route.md), [specs/ai-racers-catchup.md](specs/ai-racers-catchup.md) |
| Race modes | Yes: `libs/blackbox-race`, `scenes/world/race/`, `frontend/quick_race.rs`, `frontend/custom_race/`. Not used: shared checkpoints, cash grabs, barrier exemptions, the busted-in-a-race flow | Rules read from the executable and the install data; an install test walks all 212 courses through the rules. No comparison in a running game recorded | Not recorded for races; the countdown is recorded as improved in playtesting, exact comparison open | 22 | [specs/race-mode.md](specs/race-mode.md), [provenance/race-mode.md](provenance/race-mode.md), [roadmap](roadmap.md#race-open-items-milestone-7) |
| Career, Blacklist, milestones, bounty | Yes: `crates/nfsmw-career`, `career/`, `scenes/world/career/`, `frontend/career_screens/` | Unit tests, and one real-install test of the table counts (15 bins, 55 milestones, 39 speed traps). No played career compared | Not recorded. The markers screen and the final race are not yet checked in a running game | 8 | [specs/career.md](specs/career.md), [provenance/career-runtime.md](provenance/career-runtime.md), [provenance/career-core.md](provenance/career-core.md), [roadmap](roadmap.md#career-open-items-milestone-7) |
| Save games | Yes: `career/io.rs`, `career/autosave.rs`, the profile file glue | The layout is verified on three saves (header words, block ends, both MD5 sums). Imported original saves are listed as a comparison still needed | Not recorded | 0 | [formats/career-save.md](formats/career-save.md), [formats/career-state.md](formats/career-state.md), [garage-parity.md](garage-parity.md) |
| Garage and shops | Yes: `garage/` (`garage/pricing.rs` for prices), `frontend/garage_ui/`, `frontend/career_screens/car_lot.rs`. Arbitrary vinyl and decal movement is not implemented | Original captures of the manager, main menu, parts, performance, engine, visual, paint and empty cart screens; static evidence for packages, performance bars and trade-in. Live purchase comparisons not made | Yes, with substantial open bugs: the doc is a draft and not ready for a parity sign-off | 2 | [garage-parity.md](garage-parity.md), [specs/garage-shop.md](specs/garage-shop.md), [garage-overlay.md](garage-overlay.md) |
| Front-end UI and HUD | Yes: `frontend/`, `hud/`, `libs/blackbox-feng`. Open: HUD heat arc masks, the in-chase statistics action | Continuous reference recordings and opt-in remake tracing; private screenshots, which establish visible state only | Yes: every backlog item remains open for playtest acceptance | 10 | [menu-ui-parity.md](menu-ui-parity.md), [ui-options-coverage.md](ui-options-coverage.md), [ui-reference-capture.md](ui-reference-capture.md), [hud-layout.md](hud-layout.md); history below |
| NIS and cutscenes | Yes: `libs/blackbox-nis`, `scenes/world/nis/`. Open: audio take mapping, FEng overlays drawn as text, character lights and hide events, the arrest-spot search | None on screen: the framing of the scenes is not checked | Not recorded | 3 | [specs/nis-player.md](specs/nis-player.md), [provenance/nis-player.md](provenance/nis-player.md), [roadmap](roadmap.md#story-mode-milestone-7) |
| Audio | Partial: `audio/`. The ambience music is open | The FXEDIT processor has an isolated PC PCM comparison; a trace comparison tool exists. No matched live driving comparison; radio and sirens not compared by ear | The engine muted at the rev limiter was reported and fixed. The radio songs and the pursuit music are recorded as not yet heard; a playtest reported no audible chase radio before the speech port | 0 | [car-audio-coverage.md](car-audio-coverage.md), [pursuit-system.md](pursuit-system.md), [formats/audio.md](formats/audio.md) |
| World triggers | Partial: `scenes/world/entry_zones.rs`, `scenes/world/career/speedtraps.rs`, `scenes/world/career/safehouse.rs`. Static scenery trigger creation and reactivation are still needed | None recorded. Race gate extents are read as half sizes on evidence, not from the trigger code | Not recorded | 0 | [world-interactions-todo.md](world-interactions-todo.md), [specs/race-wrongway.md](specs/race-wrongway.md), [roadmap](roadmap.md) |
| Camera | Yes for Bumper, Hood, Close, Far, Super Far, Drift and Pursuit: `scenes/world/camera_original/`. Chase and Dynamic FOV keep custom follow models | Native arithmetic and pose fixtures, a native barrier producer replay, a numeric projection comparison. A complete matched drive remains necessary | Not recorded | 3 | [specs/camera-views.md](specs/camera-views.md), [specs/original-camera-coverage.md](specs/original-camera-coverage.md), [extra-options.md](extra-options.md) |
| Input | Yes: `input/`, `frontend/controller_remap.rs`. Open: controller rumble, PlayStation and Switch button faces | The roadmap records calibration against the original as done; brake-only reverse while staging is confirmed from the original's input instructions | Controller testing on real hardware is recorded as done; a real steering wheel is open; the held D-pad report (UI-005) is unverified on physical input | 0 | [controller-settings.md](controller-settings.md), [specs/settings-input-parity.md](specs/settings-input-parity.md), [specs/feng-input.md](specs/feng-input.md) |

## Parallel code paths

Places where two implementations of one thing run side by side. Each was checked in the code on 2026-11-17 unless the
entry says otherwise.

- **Road state of a pursuit target.** The persistent road tracker serves the primary free-roam target only
  (`scenes/world/ai/target_road.rs`). Secondary targets and races keep the previous local road adapter
  ([police-ai-parity.md](police-ai-parity.md), Remaining parity limits). The old adapter's use for secondary targets and
  races is from the doc; it was not traced in the code.
- **Police tire model.** Police cars use the simple chassis tire (`libs/blackbox-vehicle/src/tires/simple.rs`), separate
  from the racer tire solver in the same folder. The flat-road diagnostic checks acceleration, not that the two models
  are equivalent ([police-ai-parity.md](police-ai-parity.md)).
- **Prop fragments.** A detached fragment is a full rigid body in the resident prop solver
  (`scenes/world/props/fragments.rs`). The install's simple-physics flag is read (`crates/nfsmw-data/src/world/sequences.rs`)
  but no simple-physics path exists ([world-interactions-todo.md](world-interactions-todo.md)).
- **`cop_share`.** The setting is a percentage (0 to 100) that acts as a switch: any positive value enables police
  patrols and zero disables them (`scenes/world/ai/population.rs`, `patrol_enabled` and `configure`). The patrol count
  comes from the heat row, not from the percentage ([pursuit-system.md](pursuit-system.md)).

## UI parity history

Dated batch records of the UI repair work. They are kept as history; the current state is in the docs of the UI row.

- Backlog and plan: [playtest backlog](ui-parity-playtest.md), [handoff](ui-parity-handoff.md),
  [repair plan](ui-parity-repair-plan.md), [capture coverage](ui-parity-capture-coverage.md),
  [tracing](ui-parity-tracing.md)
- Batches: [12](ui-parity-batch12.md), [14 follow-up](ui-parity-batch14-followup.md), [17](ui-parity-batch17.md),
  [18](ui-parity-batch18.md), [18 follow-up](ui-parity-batch18-followup.md)
- Passes: [A](ui-parity-pass-a.md), [B](ui-parity-pass-b.md), [A+B follow-up](ui-parity-pass-ab-followup.md)
- Repair batches: [R1](ui-parity-r1.md), [R2](ui-parity-r2.md), [R3](ui-parity-r3.md), [R4](ui-parity-r4.md),
  [R5](ui-parity-r5.md), [R6](ui-parity-r6.md), [R7](ui-parity-r7.md), [R8](ui-parity-r8.md)
