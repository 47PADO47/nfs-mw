# Vehicle components: migration plan

Part of [ADR 0014](../decisions/0014-vehicle-components.md); specs in [specs/vehicle-components/](../specs/vehicle-components/README.md).
The model and resolver are built (`libs/blackbox-components`). This plan moves the 2005 garage, the 2012 content mod and the
RIDE 6 mod onto it in stages that each leave the game working: **old tables keep working until the last stage, and nothing
is removed without a mod API version bump**. Each stage is one or a few small PRs.

| Stage | Goal | Touches | Exit |
|---|---|---|---|
| 0 | design and prototype | `docs/`, `libs/blackbox-components` | this PR: ADR, specs, crate with the three worked examples as tests |
| 1 | the loader | `libs/blackbox-modding/src/content/components.rs` | mods can declare components; they are checked and logged; the game ignores them |
| 2 | numbers: `[stats]` and `[bike]` binders, `nfsmw:upgrades` | `nfsmw-data/src/car/`, `blackbox-vehicle` stats | the shop's seven upgrade categories run through components, equal to today |
| 3 | looks of mod cars: `[customization]` desugared | `nfsmw-data/src/car/mods/custom/`, garage | same choices, hashes and paints; mod cars can have body kits and hoods |
| 4 | the 2005 adapter | `nfsmw-data/src/car/`, `garage/` | stock cars generate definitions; garage reads `slot_views`; saves byte-identical |
| 5 | the content mods | `mods/ride6-bikes`, `mods/nfsmw-2012-content` | per-bike slots and parts; 2012 tiers |
| 6 | network | `nfsmw-net`, `blackbox-net-core` | selections on the wire, digest handshake |
| 7 | SDK and screens | `blackbox-modding` SDK, `ui/theme/`, `garage/` | `bb.components`, `[[extend]]`, a themed generic screen |

Stages 1 to 3 do not depend on 4; 5 depends on 1 to 3; 6 on 4 or 3; 7 on 3. Stage 4 is the largest and can slip without
blocking mods.

## Stage 1: the loader

- `content/components.rs`: parse `components/*.toml` (`[[component]]`, `[[slot_set]]`, `[[extend]]`) and the new tables of a
  car manifest (`[[slot]]`, `slot_sets`, `[parts]`, `[sets]`) with the model types, `deny_unknown_fields`; id syntax
  (`[a-z0-9_.:-]`, 64 bytes); unique ids across mods; `VehicleDef::check` for every car; report with the mod and the file.
- `CarManifest` gains `slots`, `slot_sets`, `parts`, `sets`; an old manifest (none of them) still parses to what it does now.
- No effect in the game yet: the vehicles and catalog are built and checked, the result is logged and kept.
- **Tests:** the fixtures of the crate as `.toml` files; a refusal per mistake class; an old manifest unchanged.
- **Risk:** a schema fixed too early. Mitigation: documented as *preview* until stage 3, so mods know key names may still change.

## Stage 2: numbers

- A `ParamBinder` per vehicle kind in `nfsmw-data`: `stats.*` keys over `Stats`, `bike.*` keys over `Bike`. A key table
  (path to getter/setter with the base table's range) is the single list of what a mod may change; `Catalog::used()` is
  checked against it at load.
- The built-in slot set `nfsmw:upgrades` generated from the rules of `libs/blackbox-vehicle/src/stats/upgrades.rs`
  (seven slots, three tiers). The shop's mod-car path reads `slot_views`/`quote`; bars still come from `rating.rs`.
- **Tests (the gate):** for a sweep of stats (the stock cars' converted stats, the 2012 cars' manifests, a bike) and every
  level combination, `physics_from_stats(bind(resolve(...)))` equals `physics_from_stats(upgrade(stats, levels))`
  field by field. Differences in rounding are fixed in the table, not accepted.
- **Risks:** an upgrade that is not a multiplier (the suspension level lowers the ride height by 8 mm and the centre of
  gravity by 15 mm: `add`; the `max 1` of gear efficiency: `max`/`min`); upgrades that depend on the car's own value at
  several keys (torque at every engine speed: `apply_list` with `mul`). All are expressible; the sweep proves it.
- **Bikes:** the shop's levels change nothing of a bike today ([bikes.md](../modding/bikes.md)); with `bike.*` keys a bike
  manifest can list its own tuning components. No default set for bikes.

## Stage 3: looks of mod cars

- Desugar `[customization]` as in [manifest.md](../specs/vehicle-components/manifest.md#migrating-what-exists) inside
  `car/mods/custom/` (the readers keep their names; they now build a catalog and slots).
- A **parts binder**: `ResolvedVehicle.parts` to solids by name through `[parts]`/convention; markers of drawn parts to the
  marker list; `surfaces` to the existing paint, vinyl, livery and plate pipelines.
- `my-cars.toml` gets the `components` field; the loader keeps the table from old choice hashes to ids and the readers accept
  both. The F12 `parts` command and the F6 panel list `slot_views`.
- **Tests (the gate):** a snapshot test of every mod fixture: the old `Catalog.choices` and the new views list the same
  choices in the same order; a saved selection (old form) resolves to the same paint, rims, vinyl and livery; an offscreen
  `view-car --part` screenshot of the 2012 mod's car is pixel-identical before and after
  (the repository's `cargo xtask img-diff`).
- **Gain:** body kits, hoods, any number of slots for mod cars, with rules.
- **Risk:** `Category` indexes arrays in many places. Keep the old `Selection` as a derived view during the stage
  (`ResolvedVehicle` to `Selection`), delete it in stage 4.

## Stage 4: the 2005 adapter

- Read-only first: build a `VehicleDef` and `Catalog` per stock car from `PartsDb`, `SlotTypes`, the customization catalog
  and `PerformanceCatalog` ([examples.md](../specs/vehicle-components/examples.md#the-2005-game)); ids `nfsmw:part/<hash>`,
  `nfsmw:perf/<category>/<level>`.
- **Round-trip tests on a real install** (ignored without a game, like `tests/real_install/`): for every stock car, the old
  `Selection` and `PerformancePackage` of sampled and random valid builds map to a component selection and back to identical
  values; every preset ride (`PresetRide`) maps without loss; `validate` accepts all and `normalize` changes none.
- Then switch the garage: the Parts and Performance tabs, the cart and the checkout use `slot_views` and `quote`; the
  shop's trade-in and cart rules stay in `garage/pricing`. The FEng front end's rows get slot ids.
- Behind a setting (`components_garage`, default off, then on) for two releases; **cart totals of old and new are compared for
  every car and a random sample of builds** in a test.
- The career save and `GlobalB`-derived data are untouched; the save is read through the adapter and written back from it.
- **Risks:** this is the parity-critical stage: 136 slots (decals are 48 of them), special price rules for paint, vinyl
  recolours and HUD colours, "decals filled from the body kit", unlocks and the backroom markers. Mitigation: the adapter is
  generated from the same tables the garage reads today, and the old code stays the oracle until the round-trip and cart
  tests pass on a sample of cars.

## Stage 5: the content mods

**RIDE 6 (`mods/ride6-bikes`).**
1. *Inventory:* a report of every bindings table, rows per vehicle, which parts have more than the stock row. Everything
   below depends on it. If the aftermarket rows do not exist in the data, the slots hold stock only and the stage shrinks to
   the arrangement (tuning components, colours).
2. *Slots:* write `[[slot]]` per bike from the rows it has (the table in [examples.md](../specs/vehicle-components/examples.md#per-bike-availability--the-rows)).
3. *Meshes:* write swappable parts as their own solids (silencer, header, bar pad, hand guards, rims, discs; the chain only
   where the data has a mesh); tail pipe markers per exhaust component (from `exhaust.rs`); `[parts]` tables.
4. *Sound:* a `[car_sound]` set per exhaust (`exhaust.sound`); flames profile per exhaust.
5. *Performance:* the `TuningComponent` rows that upgrades use, as `bike.*` keys with their modifier types.
- **Risks:** the skinned chassis mesh (chain, calipers, levers cannot be hidden; the importer's front-end split is rigid);
  317 bikes times options means long conversions (the batch is incremental by stamp, so only changed bikes re-run); sound
  sets need recordings per exhaust (fall back to the stock set); the manifest size grows (shared catalog keeps it flat).

**2012 (`mods/nfsmw-2012-content`).** Decode `CarUpgrade` and the `PerformanceUpgrades` records first (the repository does not
know their contents, [chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md)); then write `components/upgrades.toml`
and one `slot_set` for the 93 cars. Until decoded, the mod can ship the brief's illustrative tiers behind a config switch, off by default, so no
invented numbers reach a player by accident.

## Stage 6: network

- The car catalogue message carries `(vehicle id, digest)`; the spawn message carries the encoded selection or `Change`s.
- `Selection::decode` is the only parser of peer bytes; add it to the netcode fuzz and limit tests; the server validates and
  may call `locked` with the sender's career.
- A digest mismatch shows the car stock with a warning; tests with two peers of different catalogs.
- **Risk:** the netcode is not built for gameplay yet ([ADR 0009](../decisions/0009-multiplayer-netcode-revisions.md)); the
  stage is small and waits for it.

## Stage 7: SDK and screens

`bb.components` 1.0 (read, write, event) in `libs/blackbox-modding`; `[[extend]]` for stock cars; the themed generic screen;
modding pages rewritten around components (the old tables documented as shorthands); `docs/modding/README.md` updated.

## Risks across stages

| Risk | Handling |
|---|---|
| Ids are save identity; a rename loses choices | ids are namespaced and documented as permanent; stage 3 keeps the old-hash map; an `aliases` field is an [open question](../decisions/0014-vehicle-components.md#open-questions) |
| Parameter key drift between a mod and the game | the key table is the one list; unknown keys refuse the component at load with the key named; ranges clamp and log |
| A modifier stack builds a physics the game cannot (extreme mass) | clamp to the base table's range after the stack; same ranges as `[stats]` |
| Physics rebuilt on every click in the garage | rebuild only when `params` or `behaviours` differ; cache by canonical selection |
| Mods extending stock cars conflict | additive only; first mod wins a slot id, the second is refused for that car |
| Two sources of truth during the move | the old reader is the oracle until the round-trip tests pass; one derived view at a time |
| Combinatorial testing | rules are data, validated per definition by `check`; tests sample selections and assert `normalize` idempotence and `resolve` determinism |
| Loss of the 2005 look and feel | the FEng pages stay; only their data source changes |
