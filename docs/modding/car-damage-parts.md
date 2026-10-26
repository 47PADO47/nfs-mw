# Hinged damage parts of a mod car

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) can have parts that turn
on a hinge, or fall off, when the zones around them are damaged: a bonnet that lifts, a mirror that folds, a bumper that
hangs. The parts are solids of the car's own `GEOMETRY.BIN` and the `[[damage_parts]]` tables of its manifest say how they
move. Nothing about them comes from a car of the 2005 game, and a car with no such table is damaged as before (the
[shared zone model](../specs/vehicle-damage.md)).

## The solids

A part is a solid named like the car's other solids, `<model>_<letter>`, in the car's own folder
([car-customization.md](car-customization.md#the-files-of-a-car)). **The body solid must not contain it**: the game draws
the part in addition to the body, so a body that still holds the part would show it twice. While the car is intact the part is
drawn where it was modelled, and the car looks as if the part were still in the body.

A part is drawn at the levels of detail it has a solid for, and a level without one draws the part as the body does. So a car
cuts its parts out of its finest levels (`A`, `B`) and leaves the coarser levels whole: at a distance a damaged car
keeps its part in place. The 2012 mod does this because only the finest level of a 2012 body carries the data to cut it by.

## The manifest: `[[damage_parts]]`

```toml
# cars/fastcoupe.toml
[[damage_parts]]
name = "Bonnet"                       # 1 to 32 characters, unique in the car
model = "FASTCOUPE_KIT00_BONNET"      # the solid, without the level letter
zones = ["front", "front_left", "front_right"]
pivot = [0.74, -0.70, 0.88]           # a point of the hinge line, in the solids' space
axis = [0.0, 1.0, 0.0]                # the direction of the hinge line
angle = -15.0                         # degrees turned at the last stage
stages = [0.04, 0.05, 0.5]            # damage fractions at which each stage starts
# detach_at = 0.9                     # optional: the part is gone from this damage on
```

| Key | Meaning |
|---|---|
| `name` | A label, 1 to 32 characters, no control character, unique in the car |
| `model` | The part's solid in the car's `GEOMETRY.BIN`, letters, digits, `_` and `-`, up to 56 bytes. The game looks for `<model>_A` to `<model>_E` and draws the part at the levels that have one. A part with none is left out and the log says so |
| `zones` | At least one of `front`, `rear`, `left`, `right`, `front_left`, `front_right`, `rear_left`, `rear_right` (the game's damage zones). The part follows the worst of them |
| `pivot`, `axis` | The hinge line, in the space of the solids (+x forward, +y left, +z up), metres. The axis is any non-zero direction, normalised by the game |
| `angle` | Degrees turned at the last stage, right-handed about `axis`, not 0, at most 180 |
| `stages` | Up to 8 damage fractions from 0 to 1, in ascending order. Left out: one stage at the first damage |
| `detach_at` | A damage fraction above 0 and at most 1. From there on the part is not drawn |

A wrong value, or a key the table does not know, refuses the car and the log names the key. At most 16 parts per car. The
schema is checked by `libs/blackbox-modding/src/content/damage_parts.rs`; whether the model exists is checked by the game
when it assembles the car.

## What the game does

The damage model keeps a panel level from 0 to 3 for each of the eight zones
([vehicle-damage.md](../specs/vehicle-damage.md)). A part's **damage** is the worst panel level of its `zones` over 3: 0, 0.33,
0.67 or 1. With no damage the part is at rest. Otherwise it has turned by `angle` times the share of its `stages` that the
damage has reached, about the line through `pivot` along `axis`, and from `detach_at` on it is not drawn. A repair (the
`repair` console command) puts it back. The turn is a pose of the solid on the car: the part keeps its mesh and its materials,
and every car drawn with its damage events (the player's car, and the cars that pass them to the rig) shows it.

The slot numbers 256 and up of a car's placements are its hinged parts (`car::mods::hinged`); the parts database and the
`hidden` set of the 2005 slots do not see them.

Not done: the part does not collide as its own body, a detached part does not fall to the road, and it does not flap as the
car moves (it holds the pose of its stage). The 2012 data has no fall and no flap either
([chameleon-hinged-parts.md](../formats/chameleon-hinged-parts.md)).

## The 2012 cars

The [2012 mod](nfsmw-2012-content.md#hinged-parts) writes these tables for its cars from the 2012 game's own damage data.
A 2012 part turns through a few degrees at most, so its stages are small steps and `detach_at` is not set.

## Using it

1. Cut the part out of the body at the levels it should hang on, and name it `<model>_A` and so on.
2. Add a `[[damage_parts]]` entry. Test the pose with the `damage` console command in a world with the car
   (`damage DZ_FRONT 6`), and clear it with `repair`.

## Code

- Schema: `libs/blackbox-modding/src/content/damage_parts.rs` (`CarManifest::damage_parts`).
- Registry and pose: `crates/nfsmw-data/src/car/mods/hinged.rs` (`publish`, `of`, `Hinge::pose`, `Hinge::turn`, `add`), published by
  `crates/nfsmw/src/modding/content/cars.rs`; the parts are added to the assembled car in `car::load_with`.
- Drawing: `CarRig::instances_detail` (`scenes/world/drive/rig.rs`); the zone levels reach it in the set of revealed slots
  (`VehicleEvents::revealed_slots`).
