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
# detach_at = 0.9                     # optional: the part falls off from this damage on
# detach = false                      # optional: the part never leaves the car
mass_kg = 40.0                        # optional: its mass once it is a body of its own (default 25)

# A part that also moves with the car's speed, such as an active spoiler:
[damage_parts.active]
offset = [-0.07, 0.0, 0.02]           # metres at full deployment, in the solids' space
rotation_axis = [0.0, 1.0, 0.0]       # optional, with rotation_deg: a turn about the part's pivot
rotation_deg = 30.5                   # degrees at full deployment
# gas = true                         # optional: only with the gas pedal pressed; releasing it retracts the part
# open_after_seconds = 3.0            # optional vent: the speed must stay above above_mph this long
# close_after_seconds = 5.0           # ... and the part closes this long after the speed falls below below_mph
above_mph = 70.0                      # deploys above this speed
below_mph = 40.0                      # comes down below this speed (default: above_mph)
seconds = 0.6                         # the whole movement takes this long
```

| Key | Meaning |
|---|---|
| `name` | A label, 1 to 32 characters, no control character, unique in the car |
| `model` | The part's solid in the car's `GEOMETRY.BIN`, letters, digits, `_` and `-`, up to 56 bytes. The game looks for `<model>_A` to `<model>_E` and draws the part at the levels that have one. A part with none is left out and the log says so |
| `zones` | At least one of `front`, `rear`, `left`, `right`, `front_left`, `front_right`, `rear_left`, `rear_right` (the game's damage zones). The part follows the worst of them |
| `pivot`, `axis` | The hinge line, in the space of the solids (+x forward, +y left, +z up), metres. The axis is any non-zero direction, normalised by the game |
| `angle` | Degrees turned at the last stage, right-handed about `axis`, not 0, at most 180 |
| `stages` | Up to 8 damage fractions from 0 to 1, in ascending order. Left out: one stage at the first damage |
| `detach_at` | A damage fraction above 0 and at most 1: from there on the part falls off. Left out: the last of its `stages`, but not before 0.5 (a part comes off when the car is heavily damaged, not at the first dent) |
| `detach` | `false` keeps the part on the car whatever happens, a wreck included. Not with `detach_at` |
| `mass_kg` | 0.5 to 5000, default 25. The mass of the body the part becomes |
| `parent` | The name of an earlier part that has an `[damage_parts.active]` table, for a part that has one too: the part rides on its parent's speed movement (the Panamera's side flaps on its spoiler), at the same deployment, before its own movement |

### `[damage_parts.active]`

A part may move with the car's speed. With `gas = true` it also needs the gas pedal: it deploys above `above_mph` only while the
gas is pressed and retracts when it is released (`below_mph` is then usually 0). With `open_after_seconds` and
`close_after_seconds` (both or neither) it is an **air vent**: it opens once the speed has stayed above `above_mph` for the first,
plays out, and when the speed falls below `below_mph` it waits the second and then closes, without looking at the speed again
until it has closed.

A part may instead be moved by a **wing machine** (below). `offset` is the way it slides at full deployment (up to 5 m; it may be left out when
the part only turns), `rotation_axis` and `rotation_deg` (both or neither) the turn about the part's `pivot` at full
deployment (a non-zero axis, normalised by the game; up to 180 degrees, not 0), `above_mph` the speed it starts going out
above, `below_mph` the speed it starts coming back below (not above `above_mph`; between the two it goes on the way it was
going, so it does not flutter; when they are equal it goes out above the speed and back below it), `seconds` (0.05 to 10) how
long the whole movement takes. An active part moves or turns, not neither. A part with an `active` table may have `angle = 0`:
the damage then does not turn it, though it still comes off at its release level and in a wreck. Without a `stages` list it
comes off at the second dent of its zones (default release level 0.5).

### `[wing]` and `[[damage_parts.track]]`

A car with parts that several keyed animations move has a `[wing]` table, the machine that plays five animations (slot 0 to 4)
over the speed and the brake, and each such part has one `[[damage_parts.track]]` per animation that moves it (not together with
`[damage_parts.active]`; `parent` works as for active parts):

```toml
[wing]
open_mph = 140.0             # animation 0 plays above this speed
brake_mph = 125.0            # braking above this speed at rest plays animation 1
top_mph = 200.0              # above this speed animation 4 plays
brake_hold_seconds = 0.4     # the brake counts once it has been held this long
fade_seconds = 0.25          # cross-fades between animations
slot_seconds = [1.5, 3.0, 1.58, 3.0, 1.5]    # the length of each animation

[[damage_parts.track]]
slot = 0
translation = [[0.0, 0.0, 0.0], [0.002, 0.184, -0.208]]      # metres at each key, evenly spaced over the animation
rotation = [[0.0, 0.0, 0.0, 1.0], [0.0, 0.18, 0.0, 0.98]]    # unit quaternion (x, y, z, w) at each key, about the pivot
```

The machine: slot 0 opens above `open_mph` (and closes below it); braking above `brake_mph` at rest plays slot 1; above `top_mph`
slot 4 plays; braking with the wing out cross-fades to slot 2 (from the open wing) or slot 3 (from the top-speed wing). A part's
pose is the blend of the keys of the animations playing (the first replaces it, the second moves it toward its own pose by the
fade weight; a part with no track in an animation stays at rest there). Keys are absolute poses, 2 to 64 per track.

A wrong value, or a key the table does not know, refuses the car and the log names the key. At most 16 parts per car. The
schema is checked by `libs/blackbox-modding/src/content/damage_parts.rs`; whether the model exists is checked by the game
when it assembles the car.

## What the game does

Each tick the car's speed (the length of its velocity, in mph) moves every active part one step: out above `above_mph`, back
below `below_mph`, at a constant rate of 1 over `seconds` for the whole way (the game's own animation clock does not ease
either). The part is turned about its `pivot` by `rotation_deg` times that, moved by `offset` times that, and then turned by
the damage as below. The deployment travels to the car draw in the set of revealed
slots, like the zone levels. A part that falls off keeps the place it had. The downforce of the car's physics is not changed.

The damage model keeps a panel level from 0 to 3 for each of the eight zones
([vehicle-damage.md](../specs/vehicle-damage.md)). A part's **damage** is the worst panel level of its `zones` over 3: 0, 0.33,
0.67 or 1. With no damage the part is at rest. Otherwise it has turned by `angle` times the share of its `stages` that the
damage has reached, about the line through `pivot` along `axis`. A repair (the `repair` console command) puts it back on the
car. The turn is a pose of the solid on the car: the part keeps its mesh and its materials,
and every car drawn with its damage events (the player's car, and the cars that pass them to the rig) shows it.

The slot numbers 256 and up of a car's placements are its hinged parts (`car::mods::hinged`); the parts database of the 2005
slots does not see them.

### Falling off

A part **leaves the car** when its damage reaches its release level (`detach_at`, or the default above) or when the car is
wrecked: destroyed by its health, wrecked as a cop car, or busted by a breaker. A wreck sheds every part that can detach,
whatever the damage of its zones. The part keeps the turn it had, and becomes a fragment of the same prop solver that the
installed vehicle parts use (a cop car's hood, `docs/specs/world-sequencers.md` "Fragments"):

- a rigid body of the part's `mass_kg` whose box is the bounding box of its own solid, centred on the box, with the car's
  orientation;
- it starts with the velocity of the point of the car it sat on (the car's velocity plus its spin about the car), thrown 2 m/s
  outwards along the ground and 1 m/s up, and spinning at 3 rad/s about its hinge the way it had turned;
- gravity, ground and world collisions, and collisions with other bodies, as any prop; it does not damage the cars it hits;
- it comes to rest where it lands, and is released a while after: 5 s out of the camera's sight, then it sinks for 2 s and is
  removed; it goes at once with the car that owns it (when the car is removed); the game keeps at most 121 vehicle fragments;
- the car stops drawing the part (its slot joins the car's hidden slots), and the part does not come back, except through
  `repair`, which puts it on the car again and leaves the fallen body where it lies.

The sound is the one the game already plays for a detached part that hits the road or a car: the prop contact sound of the
metal `smackable` record `largemetalpost`. Nothing else plays when a part comes off.

Not done: a part does not flap as the car moves (it holds the pose of its stage); its box is the bounding box, not its
shape; the fallen part is not a bounds node with joints (`JOINT_MALE` and `JOINT_FEMALE` of the bounds are the trailer hitch
of the 2005 game and play no part here); no sparks or smoke are made when it breaks off.

## The 2012 cars

The cars whose spoiler is a plain speed state (the 911, the 918 Spyder and its Concept Study, the LFA, the SL65, the SLS, the LP700-4 and the Panamera: 10 bundles,
[chameleon-active-aero.md](../formats/chameleon-active-aero.md)) get one more part, `Spoiler`, with an `active` table whose
speeds, time, slide and turn are decoded from the 2012 state machine and animation, and no damage turn; the Panamera's two side
flaps are parts of their own that hang from it. The Agera's flaps follow the gas (`gas = true`), the air vents of four cars wait
(`open_after_seconds`, `close_after_seconds`) and the Veyrons' wing is a `[wing]` machine with keyed `track`s. The McLaren, the
Venom and the Huayra keep their parts at rest: what the data does not state is not guessed.

The [2012 mod](nfsmw-2012-content.md#hinged-parts) writes these tables for its cars from the 2012 game's own damage data.
A 2012 part turns through a few degrees at most, so its stages are small steps. The mod writes the part's own mass and no
`detach_at`: the 2012 data names no release level, so the default applies, which comes from the 2012 ladder of the part (its
last step, and not before half the damage): the 911's bonnet (last step 0.5) and mirrors (0.2, raised to 0.5) come off with
the second dent of their zones, its bumpers (0.75) and doors (1.0) with the third. A wreck takes them all.

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
