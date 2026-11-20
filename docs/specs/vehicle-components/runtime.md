# Runtime: the library, the game's side, saves, network, SDK, screens

Part of [vehicle components](README.md).

## The library: `libs/blackbox-components`

Pure `std` + `serde` (`serde_json` for the digest); no Bevy, no game names, no I/O ([ADR 0004](../../decisions/0004-swappable-renderers.md)
rules for `libs/` hold). Built and tested; nothing else depends on it yet.

| Module | Holds |
|---|---|
| `effect` | `Value`, `Input`, `ColourSource`, `Op`, `Effect` |
| `model` | `Catalog`, `Component`, `Condition`, `InputDef`, `Offer`, `Slot`, `VehicleDef`, `Used` |
| `selection` | `Selection`, `Pick`, `Change`; `canonical`, `diff`, `apply` |
| `state` | (private) the picks in effect and slot activity; rule evaluation |
| `validate` | `Problem`, `Gate`; `validate`, `normalize`, `locked` |
| `check` | `VehicleDef::check(catalog)`: the author's checks of [data-model.md](data-model.md#invariants-checked-by-vehicledefcheck) |
| `resolve` | `ResolvedVehicle`, `ParamTable`, `Modifier`, `Note`; `VehicleDef::resolve` |
| `view` | `SlotView`, `OptionView`, `OptionState`, `Quote`; `slot_views`, `quote` |
| `codec` | `Selection::encode/decode` (the wire form), `VehicleDef::digest` |

The calls a game makes:

```rust
vehicle.check(&catalog)?;                                    // at load, once per vehicle: Vec<String> of mistakes
let used = catalog.used();                                   // at load: parts, surfaces, params, behaviours to bind
let resolved = vehicle.resolve(&catalog, &selection)?;       // on every change; Err(Vec<Problem>) if invalid
let selection = vehicle.normalize(&catalog, &selection);     // after a player edit: repair, canonical
let views = vehicle.slot_views(&catalog, &selection, &gate); // for the screens
let quote = vehicle.quote(&catalog, &installed, &draft);     // for the cart
let locked = vehicle.locked(&catalog, &installed, &draft, &gate);
let changes = installed.canonical(&vehicle).diff(&draft.canonical(&vehicle));  // for the network
```

The library is `Send + Sync` data; resolving is cheap (a few hundred ops) and allocation is per call. The game caches
`ResolvedVehicle` by the canonical selection and rebuilds only the parts of the car that changed (below).

## What the game binds

A game (an **adapter**, in `crates/`) turns a `ResolvedVehicle` into a car. It owns five small maps, each registered once:

| Resolved | The game does | Rebuilt when |
|---|---|---|
| `parts` | looks up the solid of each part through the vehicle's `[parts]` table (or the convention) and draws it; collects the markers of drawn parts | the part set changed |
| `surfaces` | feeds the paint, vinyl, livery and decal pipelines (`target` names a pipeline: `body`, `rim`, `glass`, `body.vinyl`, `chain`) | a surface changed |
| `params` | builds the base from the manifest (`Stats`, `Bike`, the 2005 package), applies the behaviour `sets`, then `ParamTable::apply` per key, clamps to the base table's range, and rebuilds the physics | a param changed |
| `behaviours` | maps each `(domain, name)` to a handler or a parameter set; unknown names were rejected at load | a behaviour changed |
| `installed` | exposes the tiers to the shop's bars and the stats screens | always |

The physics are rebuilt only when `params` or `behaviours` differ from the previous resolution (compare the two tables);
a repaint or a rim change touches no physics. `physics_from_stats(stats, package, bounds)` stays the one function the garage
preview, the world, split screen and the AI use ([car-stats.md](../../modding/car-stats.md#upgrades)).

## Saves

| Where | What is stored | Compatibility |
|---|---|---|
| My Cars (`my-cars.toml`) | the canonical `Selection` as TOML next to the existing record (`selection` string today, `store.rs` version 2); the new field is `components` | the old `selection` string is still read; a record with `components` wins |
| The career garage record of the original game | only what the original save can hold: the 2005 slots as part hashes and the performance package, written by the 2005 adapter exactly as today | unchanged; saves stay compatible both ways |
| A mod vehicle's career record | the car type (hash of its folder) as today, and its `Selection` in a side file beside the save, `components.toml`, keyed by profile and car, like `plates.toml` | a save copied without the side file loads stock |

The side-file rule is the existing precedent: the original save has no room for a plate text, so it lives beside the save
([car-customization.md](../../modding/car-customization.md#where-the-text-and-the-design-are-kept)). A selection is read back
through `normalize`: ids a mod removed, slots a mod dropped and rules a mod added reset to default instead of failing the
load, and the log lists what was reset.

## Networking

The wire carries **selections, never numbers**. A peer sends `(vehicle id, definition digest, encoded selection)` (or
`Change`s after the first full selection). The receiver:

1. looks the vehicle up; refuses if it does not have it (the lobby's mod-manifest check, MP-63, [security-gameplay.md](../multiplayer/security-gameplay.md); designed, not built);
2. compares `VehicleDef::digest(catalog)`; a mismatch means the peers have different mods or versions, and the car is shown
   stock with a warning rather than guessed at;
3. decodes with `Selection::decode` (bounded: at most 256 picks, 128-byte strings, 8 inputs per pick, canonical ordering,
   finite numbers, no trailing bytes) and runs `validate`; an invalid selection is refused as a whole;
4. resolves locally.

A peer cannot inject a stat: the only numbers in a selection are bounded inputs, and every `param` value comes from the
receiver's own catalog. Unlocks are not checked on the wire (they are the sender's economy); a server that wants to may call
`locked` with the sender's gate. The digest is a 64-bit FNV-1a of the definition and every accepted component, which detects
accidents (a different mod version) but is **not** a security measure; the lobby mod manifest (a list of sim-affecting mods with hashes) is the compatibility check.

## The mod SDK

Content first. A mod with components is a content mod ([ADR 0005](../../decisions/0005-native-mod-plugins.md)): TOML only,
no `[library]`. A code mod gets a versioned interface later:

| Entry | Sketch | Stage |
|---|---|---|
| `bb.components` 1.0, read | `vehicles()`, `slots(vehicle) -> [SlotInfo]`, `selection(car) -> bytes`, `resolved(car) -> ResolvedInfo` (parts, behaviours, param modifiers as key/value) | 7 |
| `bb.components` 1.0, write | `set_selection(car, bytes) -> Result`; honoured only in `Load`/world phases that allow vehicle writes (disabled in multiplayer like the `bb.vehicles` writes) | 7 |
| event `ComponentsChanged` | `{ car, canonical selection }` after any change, for HUD and sound mods | 7 |
| `register_component` / `register_slot_set` | a mod that generates its catalog in `Mod::load` (the 2012 and RIDE 6 mods write their files instead, and call `bb.content.rescan`) | 7, optional |
| `bb.vehicles` | unchanged. It reads the driven vehicle's resolved numbers, not components | - |

Selections cross the ABI as the wire bytes, so the C interface needs no new structure types. The file-based route
(`components/*.toml`, `cars/<id>.toml`) is the supported one for the 2012 and RIDE 6 mods: they already generate files and
rescan.

## The screens

`slot_views` is the whole model of a customization screen: a list of slots in vehicle order, each with `group`, whether it
is active, the selected component, whether it can be empty, and its options with `name`, `tier`, `price` and a state.

- **A generic screen** (the F6 garage panel and a themed screen for mod vehicles): sections by `group`; per slot a list;
  tier ladders (a slot whose options have increasing tiers) as a stepper with bars; an option in state `Blocked(rules)` greyed
  with the rule named (the component's name from the rule's `any_of`); `Locked(unlock)` greyed with its unlock text; inputs as
  a colour picker, a slider or a text field; a cart from `quote`. Slots that are not active are not shown.
- **The 2005 garage keeps its screens** ([garage-frontend.md](../garage-frontend.md)). The adapter gives each of its menu rows
  a slot id (the rows are a fixed set) so the existing FEng pages read and write the selection through the same calls; the
  rows do not move.
- **Per-vehicle availability is visible for free**: the KTM's screen has a bar pad row and no chain row because its definition
  has the one and not the other.
