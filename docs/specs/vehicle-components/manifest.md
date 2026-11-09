# The mod manifest: components in TOML

Part of [vehicle components](README.md). This is the schema of the host loader (`libs/blackbox-modding`, `content/components.rs`,
planned) around the data model of [data-model.md](data-model.md). The model types deserialize directly from this TOML (the
prototype's tests parse their fixtures with it), unknown keys are an error, and a refused file names the key
([api-content.md](../../modding/api-content.md#cars)).

## Where things live

```
mods/example.pack/
  mod.toml
  components/exhausts.toml      [[component]] and [[slot_set]]: the catalog, any number of files, read in path order
  cars/fastcoupe.toml           the vehicle: base data as today, plus [[slot]] and [parts]
  files/CARS/FASTCOUPE/...      the solids the parts name
```

The catalog is **global**: the components of every mod's `components/` form one catalog, ids must be unique across the game
(prefix them with the mod id), and any vehicle of any mod may accept any component. A car does not own its components; it
owns only its list of slots.

## A component

```toml
# components/exhausts.toml
[[component]]
id = "example:exhaust.race"
name = "example.exhaust.race"            # translation key (ADR 0012); left out shows the id
tier = 1
price = 900
unlock = "career.heat3"                  # optional
when = [{ kind = "tier", slot = "engine", at_least = 1 }]
effects = [
  { kind = "part",      show = ["muffler.race"] },
  { kind = "param",     key = "bike.engine.torque_scale", op = "mul", value = 1.04 },
  { kind = "param",     key = "stats.mass_kg", op = "add", value = -1.5 },
  { kind = "behaviour", domain = "exhaust.sound",  select = "race" },
  { kind = "behaviour", domain = "exhaust.flames", select = "race" },
]
```

A `surface` effect: `{ kind = "surface", target = "body", finish = "metallic", colours = { base = "colour" } }`, where a
string colour names an `rgb` input of the slot and `[r, g, b]` is fixed.

## A vehicle's slots

```toml
# cars/panigale.toml   (the [bike] table and the rest of the manifest are unchanged)
slot_sets = ["example:common"]           # optional: slot lists defined once in components/ and shared

[[slot]]
id = "exhaust"
group = "exhaust"
accepts = ["example:exhaust.stock", { component = "example:exhaust.race", price = 1800 }]
default = "example:exhaust.stock"

[[slot]]
id = "chain"
accepts = ["example:chain.stock", { component = "example:chain.race", bundle = ["example:chaincolour.paint"] }]
default = "example:chain.stock"

[[slot]]
id = "chain_colour"
accepts = ["example:chaincolour.factory", "example:chaincolour.paint"]
default = "example:chaincolour.factory"
when = [{ kind = "is", slot = "chain", any_of = ["example:chain.race"] }]
inputs = [{ kind = "rgb", key = "colour", default = [212, 175, 55] }]
```

An offer's `bundle` lists components of other slots that are fitted with it (here the race chain comes painted;
[data model](data-model.md#offer)): the game installs them when the offer is picked and takes them off when it is replaced.

The slot order in the file is the screen order and the composition order. The bike has no `suspension` slot because it does
not list one; nothing else is needed to take an option away from one bike.

**Slot sets** are named lists of slots (`[[slot_set]] id = "example:common"` with `[[slot_set.slot]]` tables of the same
shape) so a mod with 300 cars that share five slots writes them once. A vehicle's own `[[slot]]` tables follow the set's
slots; a slot id that appears twice is an error. The importer of a mod that generates its manifests (RIDE 6) writes plain
`[[slot]]` tables; slot sets are for hand authors.

## Parts

`param`, `surface` and `behaviour` names are the game's keys. `part` ids need a binding to the vehicle's own files. A
vehicle's `[parts]` table gives it; a part with no entry takes the convention:

```toml
[parts."muffler.race"]
solid   = "PANIGALE_KIT00_MUFFLER_RACE"   # name without the level-of-detail letter; default: <CARID>_KIT00_<ID upper, '.' -> '_'>
role    = "body"                          # how the game poses it: body (default), steered, steered_slid, wheel_front, wheel_rear
markers = [{ name = "RIGHT_EXHAUST", position = [-0.9, -0.12, 0.31], direction = [-1, 0, 0] }]
```

- `markers` are the points the part carries. **Built: the tail pipes** (`EXHAUST`, `LEFT_EXHAUST`, `RIGHT_EXHAUST`; the loader takes
  any plain name, the game uses these three): `position` in metres and `direction` (the way the flame shoots, a unit vector) in the
  frame of the part's `role` **after** its `matrix` (for a body part, the car's frame), so the manifest says where the pipe is on this
  car whatever the pack. The game writes them to the car's marker list **when the part is drawn**, so an exhaust swap moves the flame
  with the muffler (`nfsmw-data` `components/markers.rs`). Today a car's body solid carries its `EXHAUST` markers
  ([car-customization.md](../../modding/car-customization.md#tail-pipes)); when a drawn body part declares a tail pipe, **those are
  taken off** and the part's replace them (a twin silencer: one part per can, one marker each); a part with none leaves the body's.
  Lamp points and the sound's origin are not built.
- A part whose solid is not in `GEOMETRY.BIN` is **not offered**: the loader removes every offer of a component that shows
  it, logs why, and the slot's default is used if it was the default. (The same rule as a rim whose model is missing today.)
- `role` is the game's: `steered` and `steered_slid` are the fork and the bars of a bike
  ([ride6-bikes.md](../../modding/ride6-bikes.md#the-steered-front-end)).
- `base = true` (default `false`) makes the part one of the **base vehicle's** (`VehicleDef::parts`): it is drawn whatever the slots
  hold, and no component shows or hides it by choice. It is for a model that is more than one solid at its finest level of detail
  (a bike's frame: `BODY`, `BODY2`, ...). A base part is drawn at the levels of detail its file has it at and at no other (the
  nearest-level fallback of a pack part would draw a finer solid in place of a coarser one).

## Parameter keys

A `param` key is the path of the number in the vehicle's own base data, so the author reads it off the manifest:

| Vehicle | Key | Base |
|---|---|---|
| a four-wheel mod car | `stats.<key>` | the `[stats]` table ([car-stats.md](../../modding/car-stats.md)); lists (`stats.torque_curve`, `stats.gear_ratios`) use `apply_list` |
| a bike | `bike.<path>` | the `[bike]` table ([bikes.md](../../modding/bikes.md)): `bike.front_suspension.spring_n_per_m` |
| a car of the 2005 install | `package.<category>` (a level), `package.<category>.junkman` (a flag), `aero.tuning_unlocked` | the original game's own tables ([examples.md](examples.md#the-2005-game)) |
| any | `wheels.front.radius_m`... | the `[wheels]` table, for a rim that changes the tyre size |

At load the game checks every key a catalog uses (`Catalog::used()`) against the keys it can bind for the vehicles that accept
the component, and refuses the component with the key named. After the modifiers apply, the game **clamps to the range the
base table accepts** (mass 200 to 40000 kg, grip 0.3 to 3...) and logs once, so a stack never builds a physics the game
would refuse in `[stats]`.

## Behaviour tables

A `behaviour` effect names `domain` and `select`; the game owns what the pair does ([resolution.md](resolution.md#behaviours)).
When it is a named parameter set, the numbers are the vehicle's:

```toml
[sets."tyre.compound".slick]            # in cars/<id>.toml, beside [stats] or [bike]
"stats.front_tire_grip" = 1.75
"stats.rear_tire_grip"  = 1.7
[sets."tyre.compound".offroad]
"stats.front_tire_grip" = 0.95
```

The game binds a resolved vehicle in this order: the manifest's base, then the `sets` of every selected behaviour (as base
overrides), then the components' modifiers. A component that names a `(domain, select)` the vehicle has neither a set nor a
handler for is reported as a mismatch when the car loads and the option is not offered for that car.

## Extending cars you do not own

```toml
# components/extra.toml
[[extend]]
vehicle = "BMWM3GTR"                     # an exact car id, or a pattern with * as the 2012 mod's [car.<pattern>] tables use
slot = "rims"                            # an existing slot of that vehicle
accepts = ["example:rims.five_spoke"]    # appended after its accepts

[[extend]]
vehicle = "*"
add_slot = { id = "nitrous_colour", accepts = ["example:nos.blue"], default = "example:nos.blue", group = "looks" }
after = "paint"                          # where the new slot goes in the order; left out: last
```

Extensions are **additive**: they append accepted components and add slots; they cannot change a default, remove a slot or an
option, or change another component. They apply after every vehicle has loaded, in mod load order, then slot order; a second
mod adding a slot id the vehicle already has is an error for that mod. This is how a mod adds rims to the stock cars of the 2005
game without copying their definitions.

## Migrating what exists

The tables of [car-customization.md](../../modding/car-customization.md), [car-stats.md](../../modding/car-stats.md) and
[bikes.md](../../modding/bikes.md) keep working. A manifest with **no `[[slot]]`, no `slot_sets` and no `components/`** is
read as today, and the loader builds its vehicle and catalog from the old tables, so every existing mod runs unchanged:

| Today | Becomes |
|---|---|
| `paints`, `default_paint` | slot `paint`: one component per paint (`surface` body: finish and fixed `base` colour), plus `paint.custom` with an `rgb` input `colour` for the custom body colour |
| `rim_paints` | slot `rim_paint`, the same shape on target `rim` |
| `tints` | slot `tint`: components with a `surface` on the glass target (colour, opacity as a `num` input or fixed) |
| `rims`, `spoilers`, `roof_scoops` | slots `rims`, `spoiler`, `roof`: a component per model that `show`s a part bound to that model; the default is the stock part |
| `vinyls`, `vinyl_colors` | slot `vinyl`: a component per vinyl (`surface` `body.vinyl`, `texture`, channel colours from `rgb` inputs `ch1`..`ch3`); the palette becomes the inputs' suggestions |
| `liveries`, `default_livery` | in slot `vinyl` after the vinyls (the shop lists them together today); the default livery is the slot default |
| `[customization.plate]` | slot `plate`: component `plate.text` with a `text` input of `max_len = slots`; `designs` is slot `plate_design` |
| the seven upgrade categories of `[stats]` | the built-in slot set `nfsmw:upgrades` (below), added to every four-wheel car with `[stats]` |
| `[wheels]`, `[stats]`, `[bike]`, `[engine_sound]`, `[car_sound]` | unchanged: they are the **base** the modifiers apply to |
| `[[damage_parts]]`, `[wing]`, `[[lamp_flare]]`, `[[light_bar]]` | unchanged; later they can be tied to parts (a damage part belongs to the part that is drawn) |

**Saved choices keep working.** The old identity of a choice is a hash of the car folder, the kind and the name. The loader
keeps a table from each old hash to the new component id, and the readers of `my-cars.toml` and the career garage accept
the old form and write ids from then on. A choice that cannot be mapped goes back to default, as today.

**`nfsmw:upgrades`** is the game's built-in slot set that reproduces `libs/blackbox-vehicle/src/stats/upgrades.rs`: seven slots
(`tires`, `brakes`, `chassis`, `transmission`, `engine`, `induction`, `nitrous`), each with a stock component and three tiers
whose effects are the documented per-level changes (`stats.front_tire_grip`/`rear_tire_grip` mul 1.05 per level, torque mul 1.06
per level, and so on). A mod that gives its own `[[slot]]` for any of those ids replaces that slot; a mod that gives
`upgrades = false` offers none. A bike has no `[stats]` and so none today; a bike manifest lists its own.

**`[performance]` of the 2012 mod** is not an upgrade: it is the user's conversion-time tuning of the imported base stats
([performance.rs](../../../mods/nfsmw-2012-content/src/performance.rs)) and stays as it is. The 2012 upgrade tiers become
components in that mod's `components/` once their contents are decoded.
