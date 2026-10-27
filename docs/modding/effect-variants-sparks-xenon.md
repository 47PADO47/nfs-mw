# Effect variants for collision sparks and the restored (Xenon) sparks

Two more [effect variants](effect-variants.md): the **collision sparks** (`SPARKS/<ID>/`) and the line sparks of the **restored
(Xenon) spark style** (`XENON/<ID>/`). A profile replaces the numbers the game throws a vehicle's sparks with, the way a nitrous
profile replaces the numbers of the nitrous flames. The 2012 mod writes both
(`mods/nfsmw-2012-content/src/spark_assets*.rs`; where its numbers come from is in
[chameleon-collision-effects.md](../formats/chameleon-collision-effects.md)). The parsers are
`blackbox_modding::content::{parse_sparks_profile, parse_xenon_profile}`; the game plays them in
`scenes/world/impact_effects/variant*.rs` (collision sparks) and `scenes/world/vehicle_effects/sparks/variant.rs` (restored sparks).

| Kind | Option id and label | Saved as | Folder in `files/` | Replaces |
|---|---|---|---|---|
| Collision sparks | `collision_spark_variant`, "Collision spark variant (Original PC)" | `[mod_values.nfsmw] option_collision_spark_variant` | `SPARKS/<ID>/` | the particle groups of a vehicle's hits and scrapes in the **Original PC** spark style |
| Restored sparks | `xenon_spark_variant`, "Restored spark variant (sparks only)" | `[mod_values.nfsmw] option_xenon_spark_variant` | `XENON/<ID>/` | the two line sparks (`emsprk_line1`, `emsprk_line2`) of the **Restored (Experimental)** style |

Both options are hidden while they have only **Original**; the 2012 mod adds the value **NFS Most Wanted 2012** (id `mw2012`) to each.
A profile that is missing or refused is logged and the game's own sparks play. The **Collision Sparks** setting must be on (Options
> Video) for either to show, and the **style** decides which variant acts: a `SPARKS` value acts in the Original PC style, a
`XENON` value in the Restored style, and the other style keeps the install's data. **The restored variant is the sparks only:** the
Restored style's wind contrail and the view streaks of the speed trails have no 2012 equivalent, so there is no variant of them
(a `[trail]` or `[streaks]` table is refused), and they keep the game's own look and its `[xenon_effects]` tuning.

## What stays the game's

The variants replace what is thrown and how strong it is. What the install keeps is whether a surface sparks at all and where the
camera must be, and for the ground and the car's underside also its own acceptance of the contact.

**A car against a wall, a barrier, a guard rail or a prop** (the world's collision mesh and the scenery; a car against a car
too) is played by the profile's own gates:

- an **impact** plays when the contact closes (a closing speed above 1 m/s) and the wall reacted to the car; the strength is the
  profile's curve of the closing speed. The install's own ramp (a reaction of 1 m/s or more) is not asked, so a tap of 3 m/s plays;
- a **scrape** plays whenever the car slides along the surface; the profile's gate (the 2012 one: nothing below 0.73 m/s, full from
  1.37) says how much. The install's scrape would start only at 5 m/s of sliding speed, which would leave out the slow slides the
  2012 game sparks;
- the surface must have a spark in the install for the style: in the **Original PC** style a particle group (every surface of the
  installed cars has one, wood included), in the **Restored** style a restored spark, **unless the profile has sparks for that
  channel**, which then plays on any surface (the install has no restored spark for wood, a fence of wood sparks in 2012 all the same);
- the group's draw distance applies to the Original PC style (100 m for the installed cars).

A wall reacts to a car in some steps only (a car sliding along it is pushed back and comes in again), so a scrape goes on from the
car's last contact for 0.1 s when a step brings none (the install keeps its scrape for 0.2 s the same way).

The **ground** (a landing, the underside) keeps the install's acceptance of the contact: it closes or slides, with the link's
minimum. A channel the profile has no layer or spark for (`[hit]` or `[scrape]` absent, or with none) is the install's, exactly
as it was. With the variant **Original** nothing here changes.

## Common to both profiles

```toml
format = 1                     # required

[hit]                          # the burst of an impact
closing_speed_full = 22.08     # m/s: the closing speed at which the burst is at full strength
intensity_floor = 0.153        # 0 to 1: the strength at no closing speed

[scrape]                       # the continuing sparks of a slide
speed_start = 0.73             # m/s of sliding speed below which nothing is thrown
speed_full = 1.37              # m/s from which the rate is the full one
```

The gate keys are needed when the channel has layers (`[[hit.layer]]`/`[[hit.spark]]`) and optional otherwise. **Strength** `s` of an
impact is `floor + (1 - floor) * clamp(v / closing_speed_full, 0, 1)` for the contact's closing speed `v` (the component of its
velocity into the surface); of a scrape it is `clamp((u - speed_start) / (speed_full - speed_start), 0, 1)` for the sliding speed
`u`. It replaces the install's intensity curve (the linkage's minimum and maximum and the wrapper polynomial). `closing_speed_full`
is above 0 and `speed_start` below `speed_full`, both up to 1000 m/s. A scrape of strength 0 throws nothing (and the install's
group does not play either). **Strength scales the count and the speed of an impact**: it throws `s` times the count of sparks
`s` times as fast. A scrape scales the **rate** only.

**Counts** are per contact patch: the 2012 game counts particles per square metre of the mesh near the contact; the mod multiplies by
the area of half a disc of the patch's radius (`pi r^2 / 2`, 0.39 m^2 for the impact's 0.5 m) and writes the result.

## `SPARKS`: one layer is one emitter

```toml
[[hit.layer]]                  # up to 8 in a channel
kind = "streak"                # "streak" or "sprite" (the default)
# texture = "SPARKS/MW2012/E140280_0.PNG"   # an RGBA PNG in the mod's files; needed by a sprite
grid = 0                       # 0 (the whole picture), 2, 4 or 8 cells a side
fps = 0                        # 0 to 60: a sheet plays at this many frames a second ...
random_frame = false           # ... or, with fps = 0, shows a random cell for each particle
blend = "additive"             # or "alpha"
burst = 79.0                   # particles at once at full strength (hit)
life = 0.85                    # seconds; life_variance 0 to 1 (a life between life * (1 - v) and life)
life_variance = 0.7059
speed = 8.0                    # m/s along the surface normal at full strength; speed_variance 0 to 1
speed_variance = 0.5
spread = 10.0                  # degrees: the half angle of the cone around the normal
gravity = 9.8                  # m/s2, down
drag = 0.0851                  # a particle loses drag * speed of its speed each second, per m/s
bounce = [0.8, 0.9]            # parsed, not used by the Original PC style (it has no bounce)
friction = [0.01, 0.05]        # parsed, not used
inherit = 1.0                  # share of the car's velocity taken at birth, 0 to 1; inherit_variance too
inherit_variance = 1.0
stretch = 0.125                # seconds of travel a streak shows
keys = [0.0, 0.33, 0.66, 0.99] # ages (0 to 1) of the next lists: 2 to 16, strictly rising; straight lines between
size = [0.08, 0.0184, 0.0118, 0.0052]       # full width in metres at each key
colors = [[255, 104, 39, 255], [255, 104, 39, 255], [255, 112, 36, 230], [255, 125, 28, 0]]   # red, green, blue, alpha bytes
brightness = [12.0, 6.0, 6.0, 5.3]          # an overbright multiplier of the colour at each key (default 1)

[[scrape.layer]]
rate = 196.0                   # in a scrape: particles a second at full strength, instead of burst (not both)
# the other keys as above
```

(The numbers are of the 2012 impact's first spark, abridged.) How the game draws a layer:

| Key | Meaning in the game |
|---|---|
| `kind = "streak"` | a quad along the particle's velocity, `size` wide and `speed * stretch` metres long (never shorter than it is wide: no `stretch` is a round dot), with the head at the particle. With no `texture` it uses the game's own streak picture (white, bright at the head, fading to the tail, soft at the sides); a `texture` replaces it (`v = 0` is the head) |
| `kind = "sprite"` | a camera-facing square `size` wide with the `texture`, or one cell of its `grid` |
| `brightness` | multiplies the colour in the textured effects pass (`EffectVertex::detail`), so a value above 1 blooms under the additive blend |
| `burst` / `rate` | `count * s` particles at once, or `count * s * dt` in each physics step, kept in a per-car, per-layer carry so the rate is right at any step |
| `speed`, `spread` | the direction is inside the cone, spread evenly over its cap; an impact flies at `speed * s * (1 - speed_variance * u)`, a scrape at `speed * (1 - speed_variance * u)` |
| `inherit` | adds the car's velocity times `inherit * (1 - inherit_variance * u)` |

A car scrapes **once a step**, from its strongest contact; a car that does not scrape in a step starts again with no carry. At most
64 cars scrape at once and 1024 sparks are alive. Ranges the parser checks: counts 0 to 1024, life 0.01 to 10 s, `speed` 0 to 200,
`spread` 0 to 180, `gravity` -50 to 50, `drag` 0 to 10, shares 0 to 1, `size` 0 to 10 m, `stretch` 0 to 5 s, `brightness` 0 to 64,
bytes 0 to 255.

## `XENON`: one spark is one `EmitterStyle`

```toml
[[hit.spark]]                  # up to 2 in a channel (the Restored style has two line sparks)
color = [1.0, 0.407, 0.153, 1.0]   # RGBA, 0 to 1
life = 0.55                    # seconds; a spark has one fixed life: life * (1 - life_variance)
life_variance = 0.0
count = 79.0                   # 0 to 256: at once at full strength (hit); particles a second at full strength (scrape)
count_variance = 0.0           # 0 to 0.01
center = [0.0, 0.0, 0.0]       # the birth volume, local frame (z along the surface normal), metres
extent = [0.5, 0.5, 0.0]       # the full width
velocity = [0.53, 0.53, 6.0]   # local start velocity, m/s (an impact scales it by the strength)
variation = [1.0, 1.0, 0.333] # multiplicative: each component times 1 + variation * (2u - 1)
inherit = [0.55, 0.55, 0.0]    # share of the incoming world velocity
gravity = -4.9                 # the coefficient of the parabola (half the acceleration), start ...
gravity_delta = 0.0            # ... and delta
length = 255.0                 # 0 to 255: time offset of the segment's tail, over 2048 seconds
length_delta = 0.0
height = 59.0                  # 0 to 255: full width, over 2048 metres
elasticity = 217.0             # 0 to 255, over 255: the share of the speed kept at a swept bounce (the style's own are 160 and 120)
```

The keys are the restoration's `EmitterStyle` fields as `EmitterStyle::read` names them in the attribute data (`Colour1`, `Life`,
`LifeVariance`, `NumParticles`, ...) plus `elasticity`, the byte the host takes from the INI
([xenon-particle-motion.md](../specs/xenon-particle-motion.md)). The parser applies the limits of `EmitterStyle::validated`
(colour 0 to 1, life above 0 and up to 10 s, vectors within 1000, extent and variation not negative) and 0 to 255 for `length`,
`length_delta`, `height` and `elasticity`. The mod converts the 2012 numbers this way: `velocity.z` the mean speed,
`velocity.x/y` `mean * tan(spread / 2)`, `variation.z` `(max - min) / (max + min)`, `gravity` half the 2012 acceleration, `length`
the streak's seconds times 2048, `height` the mean width times 2048, `elasticity` the mean restitution times 255, `life` the mean
of the 2012 life range.

In the game: the strength picks the strongest contact and drives the flash as before; an impact throws `round(count * s)` sparks
with the velocity scaled by `s`, a scrape throws `count * s * dt` sparks in each step (so `count` is **per second** there, and the
`[xenon_effects]` `spark_rate` dispatch clock is not used). The tuning still applies: `spark_intensity` (above 1 more particles,
below 1 dimmer ones) and `max_sparks`. The contact flash takes the colour of the profile's first spark.

## Not verified

The contact patch area (a nominal 0.39 m^2), the segment length factor (the executable's own constant is taken as 1), the speed
scaling of the scrape (taken as none), that a scrape's strength follows the sliding speed and an impact's the closing speed (the
inputs are named `ImpactClosingVelocity` and the effect's `Velocity`; the code that fills them was not found), and which of the 8
surface slots the install's default is (the mod takes the effect the objects list most). That the 2012 game sparks a wall in every
state of the car (its trigger code was not found; the host plays any wall contact that closes above 1 m/s or slides). The mod writes the
2012 dust (3 m radius, growing to 1.85 times) at most 10 m wide, which is the most a layer may be, and draws a sprite that is not
overbright (the dust) with alpha blending instead of adding it; the 2012 blend of the dust is not decoded. The look of the variants in
the renderer (the streak picture, the overbright, the flash that a scrape throws 196 times a second) has not been compared with the
2012 game. The tests cover the parsers, the strength curves, the counts and speeds, the gating and the wall contacts (a fake wall
contact at 20 m/s, a slide at 0.5 and at 2 m/s); no screenshot of the variant has been taken.
