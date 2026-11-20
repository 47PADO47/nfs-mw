# 0014 — Vehicle components: a base vehicle plus installed components

- **Status:** proposed (2026-10-29). Designed; the model and resolver are built and tested as `libs/blackbox-components`,
  first integrated for the RIDE 6 bikes ([ride6-bike-upgrades.md](../modding/ride6-bike-upgrades.md)): loader, bike binder and the importer; the 2005 garage and the 2012 mod are not moved.
- **Developer tools (2026-11-08):** the F11 vehicle panel, the `vehicle` console command and the F6 garage's Parts tab list a vehicle's
  slots from its components, or from the category model for a car that has none ([devtools-vehicle.md](../devtools-vehicle.md)).
- **Shop (2026-11-08):** the auto shop, performance shop, Junkman and My Cars screens list a vehicle's slots from its components, price
  and refuse by them, and save a slot no category holds with the car ([specs/garage-shop-components.md](../specs/garage-shop-components.md));
  the 2005 cars keep their categories, which are the model's fallback.
- **Specs:** [specs/vehicle-components/](../specs/vehicle-components/README.md) (data model, resolution, manifest, runtime,
  worked examples). **Plan:** [plans/vehicle-components.md](../plans/vehicle-components.md) (seven stages, risks).
- **Builds on:** [ADR 0005](0005-native-mod-plugins.md) (mods add files and registry entries; content mods need no code),
  [ADR 0008](0008-ui-themes.md) (one component system for screens), [ADR 0012](0012-localization.md) (names are keys).

## Question

A vehicle's customization is described three different ways in the repository, and none of them can describe the others.

How should a vehicle and its upgrades be modelled so that **one system** serves the 2005 game's garage, the 2012 content
mod (body and tyre tiers that change numbers and behaviour together) and the RIDE 6 bikes mod (each bike offers different
parts: a KTM 450 can change its bar pad but has no chain to change; a Panigale V4 S can change chain and chain colour but not
its suspension; an exhaust changes the look, the power, the sound and the flames), for the original content and for mods,
with saves, the network and the shop?

## Facts checked on 2026-10-29

What is already generic, and what is hard-wired:

| Fact | Where | Consequence |
|---|---|---|
| The parts database is generic: parts by type, slot and level, with wide in-memory indices | `libs/blackbox-carparts`, [car-parts-capacity.md](../specs/car-parts-capacity.md) | it stays the source of the 2005 parts; a mod vehicle need not push every option into it |
| The 2005 slot list is a layout table (136 `CAR_SLOT_ID`s) | `libs/blackbox-carparts/src/layout/` | data already; the garage does not use it as a list |
| Nine fixed categories and fixed arrays: `Category`, `Selection { parts: [Option<u32>; 9], vinyl_colors: [_; 3], decals: [[_; 8]; 6] }` | `crates/nfsmw-data/src/car/customization.rs` | a tenth category, or a car with one fewer, cannot be said |
| Seven fixed performance categories and `PerformancePackage { levels: [u8; 7], junkman: u8 }`; the ladder length is per car (`<category>_upgrades`), levels interpolate two attribute collections | `car/physics/performance/`, [performance-upgrades.md](../specs/performance-upgrades.md) | per-car ladders exist; categories do not |
| A mod car's customization is eight fixed lists (`paints`, `rims`, `spoilers`, `roof_scoops`, `vinyls`, ...), at most 64 each; no body kits or hoods; every choice is priced at tier one | `libs/blackbox-modding/src/content/customization.rs`, [car-customization.md](../modding/car-customization.md) | a mod cannot add a slot, a rule or a price |
| A mod car's upgrades are seven fixed categories of three levels with hard-coded percentages, on `[stats]`; **a bike gets none** | `libs/blackbox-vehicle/src/stats/upgrades.rs`, [car-stats.md](../modding/car-stats.md), [bikes.md](../modding/bikes.md) | no per-vehicle upgrade, no bike upgrade |
| The 2012 mod's `[performance]` and `[car.<id>]` are conversion-time multipliers, not upgrades | `mods/nfsmw-2012-content/src/performance.rs` | the 2012 tiers do not exist in the game |
| The garage screens loop over `Category::MODELS` and the seven performance categories; the saved selection is a string of arrays | `crates/nfsmw/src/garage/{parts,performance}.rs`, `my_cars/store.rs` | the UI cannot list "the slots this vehicle has" |
| A vehicle's parts: three for a car, six for a bike (body, wheel, driver, rear wheel, fork, bars); an exhaust is part of the body solid | `car/mods/parts.rs`, [ride6-bikes.md](../modding/ride6-bikes.md) | swappable parts need to be separate solids |
| Plate text and design cannot go in the original save and are kept beside it | [car-customization.md](../modding/car-customization.md#where-the-text-and-the-design-are-kept) | a side store beside the save has precedent |
| RIDE 6 keeps one row per part per vehicle in its bindings tables (`MUFFLER`, `BARPADS`, `CHAIN`, ...), and tuning rows that name a member path with a multiplier or additive modifier | [ride6-bikes.md](../modding/ride6-bikes.md), [ride6-physics-elements.md](../formats/ride6-physics-elements.md) | per-bike availability is the presence of a row; performance changes are named-field modifiers |
| The 2012 game has `PowerTrain`, `Nitrous`, `Body`, `Chassis`, `Tyre` upgrade lists; their contents are not decoded in this repository | [chameleon-customization.md](../formats/chameleon-customization.md) | the 2012 tiers of the brief are **assumed**, with invented numbers, until decoded |
| The brief's task named a `[performance]` table of car manifests; that table is the 2012 mod's configuration, not a manifest table | `mods/nfsmw-2012-content/src/performance.rs`, [api-content.md](../modding/api-content.md#cars) | no manifest `[performance]` exists to migrate |

## Options

1. **Keep growing the fixed tables:** add keys to `[customization]` and the shop (`exhausts`, `forks`, `chains`), add a
   bike upgrade table, add tiers to `[stats]`. Cheapest now; every new part kind is a schema change in three crates, every
   dependency between parts is code, and a vehicle that lacks a slot is a special case in the garage.
2. **Per-game systems behind one screen:** the garage asks a trait `Customizable` per vehicle kind. Keeps each game's logic,
   but nothing is shared with mods, saves and the network need three encodings, and a mod cannot add a slot to a car.
3. **One data model: vehicle = base + slots; slots accept components; components carry effects** (chosen). Per-vehicle
   availability, rules, prices and the three kinds of change are data; games differ only in how they bind a resolved vehicle.

## Decision

1. **A new engine-generic crate `libs/blackbox-components`** (pure `std` + `serde`, no game names). A `VehicleDef` lists
   **slots**; each slot lists the **components** it accepts (`Offer`), a default (stock) and optional rules; a shared
   **`Catalog`** defines components once; a **`Selection`** is the player's picks. `resolve(vehicle, catalog, selection)`
   returns a `ResolvedVehicle`. [data-model.md](../specs/vehicle-components/data-model.md).
2. **Per-vehicle availability is the vehicle's slot list.** There is no global category list. The KTM 450 declares a bar pad
   slot and no chain slot; the V4 S declares chain and chain colour and no suspension; every 2005 car gets the same slot
   set generated from its tables. A selection naming a slot or component the vehicle does not accept is **refused**, never
   ignored.
3. **Four effect kinds cover the three kinds of change.** `part` (show or hide model parts: swap a muffler), `surface`
   (materials, textures, colours: paint, vinyl, chain colour), `param` (set, add, multiply or limit a number the game reads,
   or set a flag/text/list: mass, power, grip, brakes, springs) and `behaviour` (pick a named behaviour of a domain, which
   the game maps to a handler or a named parameter set: `tyre.compound = slick`, `handling.model = drift`,
   `exhaust.sound = race`). Visual changes are `part` and `surface`, performance is `param`, behaviour is `behaviour`
   plus flags; one component may have any of them, so a tyre changes numbers and behaviour **together**.
4. **Composition is fixed and local.** Slots fold in the vehicle's order, effects in listed order. A number is
   `clamp(((set or base) + adds) * muls)` with a floor and a cap; two `set`s keep the later slot; parts hidden by anyone
   stay hidden; behaviours keep the later slot; each conflict produces a `Note` for the author, not a failure.
   [resolution.md](../specs/vehicle-components/resolution.md#order).
5. **Dependencies and exclusions are rules** (`is`, `is_not`, `filled`, `empty`, `tier`) on a slot (it exists only while they
   hold, naming earlier slots only), on a component, or on one vehicle's offer. A default has no rules, so stock is always
   valid and `normalize` (reset every offender to default) always succeeds. A screen sees why an option is `Blocked`.
6. **The library holds pricing and unlock hooks, not the economy.** A component has a `price` (an offer can override it per
   vehicle) and an `unlock` name; `quote` lists what a change installs and removes; `locked` asks a `Gate`. **Resolving
   never asks the gate**, so a saved car keeps what it was bought with. Trade-in and refunds stay the game's shop rules.
7. **Behaviour is expressed as names, not code.** The library carries `(domain, select)` and knows no physics. The game
   registers the names per domain and maps each to a handler or to a table row of the vehicle (`[sets]`). A name the game
   cannot map is reported when the catalog loads and the option is not offered for that vehicle.
8. **Serialization is canonical and carries ids, not numbers.** Saves and the network store the canonical sparse
   `Selection` (picks that differ from stock), as TOML/JSON in saves and as a compact bounded byte form on the wire
   (`encode`/`decode`, strict, 256 picks at most). A peer cannot inject a stat: values come from the receiver's own catalog.
   `VehicleDef::digest` lets peers detect different definitions before comparing selections. Resolution is pure and
   deterministic (ordered maps, fixed float order).
9. **Mods write TOML first.** `components/*.toml` holds the global catalog (`[[component]]`, `[[slot_set]]`); a car manifest
   gets `[[slot]]`, `[parts]` (part id to solid, markers and role) and `[sets]`; `[[extend]]` adds accepted components or slots to
   cars a mod does not own, additively. A native `bb.components` interface comes later and carries the wire bytes.
   [manifest.md](../specs/vehicle-components/manifest.md).
10. **Compatibility.** Every existing manifest keeps working: with no `[[slot]]` the loader desugars `[customization]`
    (and, for four-wheel cars, the built-in `nfsmw:upgrades` slot set that reproduces `stats/upgrades.rs`) into the new model,
    keeping a table from old choice hashes to ids; `[stats]`, `[bike]` and `[wheels]` stay as the **base** the modifiers apply
    to. The original game's stock cars get definitions **generated by an adapter** from their tables, and the original save
    stays the file format of the 2005 garage; anything the save cannot hold goes in a side file beside it.
11. **Where code lives.** Model and resolution: `libs/blackbox-components` (nothing in it names a game). Adapters and
    binders (parts to solids, params to `Stats`/`Bike`/package, behaviours to handlers): `crates/nfsmw-data/src/car/`.
    Manifest loading and the SDK: `libs/blackbox-modding`. Screens: `crates/nfsmw/src/garage/` and the UI themes.
12. **Staged, with the old code as the oracle.** Seven stages ([plan](../plans/vehicle-components.md)); the 2005 garage moves
    last and only after round-trip tests pass on a real install.

## Consequences

- **Good.** One description of "what can this vehicle change" for three games and for mods, readable by the screens
  (`slot_views`), the shop (`quote`), saves and the network. A mod adds a slot, a rule, a price or a body kit without
  touching the game. A bike can have upgrades. Backwards compatible. The core is small (about 1,400 lines plus 1,000 lines of tests, among
  them the three games' examples) and pure, so it can move to the shared monorepo.
- **Cost.** An adapter per game and a key table per vehicle kind (the list of numbers a mod may change) must be maintained;
  the 2005 garage migration is the largest, parity-critical change; swappable meshes must be separate solids (the RIDE 6
  importer changes); ids become permanent save identity.
- **Not solved by the model.** Whether the RIDE 6 data has alternative parts per bike and what the 2012 upgrades contain
  are facts the repository has not read; the model is ready for them, the content stages need them (stage 5 starts with an
  inventory). Skinned meshes (the chain in the KTM's chassis mesh) cannot be hidden by a `part` effect.
- **Out of the model, deliberately.** Continuous player tuning (suspension sliders, gearing), a licence plate's stored text
  and the paint's palette UI stay where they are; components can unlock them.

## Alternatives rejected inside the decision

- *Components as full vehicle clones* (a bike variant per exhaust): the number of cars multiplies (317 bikes), and the 2012 mod
  already makes the point that "a body variant is another car" is a limit ([car-customization.md](../modding/car-customization.md#limits)).
- *Raw stat values in the selection or on the wire:* simplest and unsafe; the selection carries ids so a peer or a save edit
  cannot build an impossible car.
- *Physics deltas in the library as typed fields:* it would name tyres, brakes and engines in `libs/`. String keys mapped by
  the game keep `libs/` free of game and physics names ([AGENTS.md](../../AGENTS.md)).
- *Priorities on effects or components for ordering:* more data to get wrong; slot order is already the author's choice
  and is shown on the screens.
- *A scripting language for conditions or effects:* rules and four effect kinds cover every case found; a script engine
  would add a sandbox and a security surface for nothing seen yet.

## Open questions

Defaults are chosen so work is not blocked; each is flagged where the answer would change code.

1. **Sparse selections follow the default.** A pick equal to the default is not stored, so a mod that changes a vehicle's
   default changes every car that never touched that slot. Default: yes (that is what "stock" means). Alternative: store
   every pick of an installed car (dense), at the cost of larger saves and diffs.
2. **Ids versus hashes in saves.** The 2005 save keeps part hashes; the new side stores keep string ids. Default: strings
   (readable, mod-safe), with the 2005 adapter mapping hashes. Needed: whether to add an `aliases = ["old.id"]` field to a
   component so a mod can rename without losing saves (default: not yet).
3. **Trade-in and ownership.** The 2005 shop gives a quarter back for eligible model parts and has no inventory of owned
   parts. Default: the same rule for every game, no inventory (buying installs). Do the RIDE 6 and 2012 modes want owned-parts
   inventories or free swapping in My Cars?
4. **Percentages compound.** `mul 1.1` twice is `1.21`. Default: yes, matching the way the 2005 and RIDE 6 modifiers stack.
5. **`f64` in the library, cast by the game.** Default: yes; the bike and car physics are `f32`.
6. **Named parameter sets live in the vehicle (`[sets]`), not in the catalog.** A catalog-level `behaviour` table
   (domain plus name to a bundle of `param` effects, shared by many vehicles) would let a mod define "slick" once. Default: not
   now; the game or the vehicle maps it. Add if mods repeat the same tables.
7. **Who may extend a stock car.** `[[extend]]` is additive and first-come-first-served. Do you want mods to be able to
   *remove or replace* stock options (a total conversion)? Default: no.
8. **The brief's 2012 tiers and RIDE 6 aftermarket rows are assumed.** Default: the model is built for them; the content
   stages start with an inventory and decoding, and ship nothing invented. Do you want the illustrative 2012 tiers shipped
   as an opt-in anyway?
9. **Lobby compatibility.** A digest mismatch shows the car stock with a warning. Alternative: refuse the player from the
   session. Default: show stock; the lobby's mod-manifest check (MP-63) is the strict gate.
10. **Decals.** The 48 positional decal slots of the 2005 game become 48 slots in a group. Default: yes, one group on the
    screen. Alternative: one slot with several picks (multi-slot components).
11. **A second owner for one slot.** Two mods cannot both add a slot with the same id to one stock car. Default: first wins,
    the second is refused for that car with a log line.
12. **Which unlock names exist.** The model has the hook (`unlock = "career.heat3"`); the 2005 career, My Cars and the mods
    each need names. Default: the career provides `career.*` names, My Cars unlocks everything, mods may use `mod:<id>.*`.
13. **Names are translation keys** ([ADR 0012](0012-localization.md)); a component without one shows its id. Default: yes.
