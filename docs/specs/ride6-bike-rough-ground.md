# RIDE 6 bikes: loose and rough ground in the arcade steering (clean-room spec)

**This is a modification made exclusively for the NFS:MW rewrite. It is not the behaviour of RIDE 6.** It is part of
`blackbox_vehicle::bike::ArcadeSteering` (the field `rough`, `ArcadeRough`, `libs/blackbox-vehicle/src/bike/arcade_rough.rs`;
[ride6-arcade-steering.md](ride6-arcade-steering.md)); `ArcadeRough::REFERENCE` is neutral in every field and `bike_steering = reference` gives the
reference behaviour back exactly. Tags as in [ride6-bike-jumps.md](ride6-bike-jumps.md): **[R]** read in the code, **[D]** data, **[C]** a choice,
**[T]** covered by a test that passes, **[M]** measured (section 6).

## 1. What the player reported

"offroad bikes should be able to handle better offroad".

## 2. What happens on dirt now, read from the code [R]

The ground gives every tyre contact a `SurfaceGrip` (the host's `simsurface` table, `crates/nfsmw-data/src/car/physics/surface.rs`: a lateral and a
driving multiplier and the surface's tag; dirt, gravel, grass and sand are the loose ones). `bike/wheel_step.rs` multiplies the tyre's force by them and
nothing else in the bike knows about them:

1. **The lean controller leans for tarmac.** Its lean limit is the arctangent of the front tyre's grip at the rest load (`steer_handlebar`, `lean.rs`),
   a number of the tyre alone: 52 degrees on a surface that gives 0.65 of that, which asks the tyre for a force it cannot give.
2. **The arcade steering's cornering assist ignores the surface** (a known open item of [ride6-arcade-steering.md](ride6-arcade-steering.md),
   section 8): a side force of up to 1.1 g at the centre of gravity, whatever the tyres can hold. On dirt the bike is asked for 1.6 g and the tyres
   give 0.65 of 1.0 g: the front slides out, the heading angle and the lean pass their fall limits within a second, and the bike goes down at once,
   not progressively.
3. **The suspension's damper is the data's, on any ground.** A washboard's bumps arrive at 5 to 20 Hz and the bike's pitch and angular speed answer them
   inside the limits of the fall tests of a bike meant for tarmac.
4. **The fall tests are the same on loose ground**: a lean that stays beyond its limit for 0.01 s is a fall.

The code facts are read; what they add up to on a real dirt road (items 2 and 3 especially: how fast the bike goes down, what a bump's angular speed
is against the limits) is the reading of the code and **was not measured** (section 8).

## 3. What the help does (`ArcadeRough`) [C]

The tables are indexed by the class (`TerrainClass`: road, motard, offroad, [ride6-bike-jumps.md](ride6-bike-jumps.md), section 4). "Looseness" is
`ramp(surface lateral multiplier, [0.95, 0.75])`: 0 on tarmac and anything above 0.95, 1 at 0.75 and under, linear between; the bike follows it at
`surface_rate` 5 per second so a change of ground is met progressively.

1. **The bike knows the surface** (`follow_surface`, `rough.rs`). Each step the two wheels on the ground report their lateral multiplier (after the
   class's relief, below) and looseness; the bike keeps the mean, lagged at 5 /s, in `RoughState` (and keeps it in the air). The lean controller's grip is
   the tyre's **times that multiplier** (`steer_handlebar`: `lateral_grip: grip * surface_factor()`): on dirt the lean limit is the arctangent of a smaller
   grip, the bike leans less and turns wider, which is what a surface that gives less asks. The cornering assist is **scaled by the same multiplier**
   (`cornering_acceleration`). The fall lean limit still follows the tyre's own grip (unscaled, `follow_grip`), so a bike that the surface lets slide is
   not one the fall test calls fallen at the smaller lean: it slides first. That is the progressive breakaway: the tyre saturates, the heading angle
   grows over a second or more, and the next two helps give the rider that second ([T] `the_bike_follows_the_surface_and_turns_wider_on_dirt_than_on_tarmac`).
2. **Less penalty for the bikes made for it** (`grip_relief` [0.5, 0.75, 0.95], `rough::feel`). The share of a surface's grip loss that is taken away,
   lateral and driving: `1 - (1 - grip) x (1 - relief)`. Dirt at 0.65 is 0.825 for a road bike, 0.91 for a motard and 0.98 for an offroad bike; a surface
   that gives 1 or more is untouched and a relief of 0 returns the grip bit for bit. The relief is applied before the bike's belief, so an offroad
   bike also leans more and turns harder on dirt ([T] `an_offroad_bike_corners_harder_on_dirt_than_a_road_bike`, `an_offroad_bike_keeps_more_of_a_loose_surface_than_a_road_bike`).
3. **The suspension damps more on loose ground** (`damping` [1.0, 1.15, 1.40] at looseness 1, `Suspension::force_scaled`): the damper's force and
   slope are multiplied by `1 + (factor - 1) x looseness`, with the looseness the bike believes in (smoothed, so the damper does not change with every
   bump). Only the compression phase of a wheel on the ground; the wheel in the air is the data's ([T] `the_suspension_damps_more_on_loose_ground_for_the_offroad_bike_and_not_on_tarmac`).
4. **The fall tests forgive loose ground** (`tolerance` [1.0, 1.30, 1.80] at looseness 1, `lean_hold` [0, 0.15, 0.30] s): the limits of the lean, the
   pitch, the heading angle and the angular speed are wider by the factor (the same `widen` as a landing, the wider of the two applies, never past 85,
   80 and 86 degrees), and a lean must stay beyond its limit for `lean_hold` times the looseness more than the data's 0.01 s before it counts. A bump
   that jolts an offroad bike for a tenth of a second is not a fall; a lean that stays is.

### Why these values [C]

- `loose_range` [0.95, 0.75]: the host's dirt and gravel are the surfaces the audio table marks `secondary` (`surface.rs`); their lateral multipliers
  are in the installed table and **were not read for this spec** (section 8). 0.95 is chosen so that tarmac and the ordinary road surfaces, which sit at or
  above it, are never loose, and 0.75 so that a surface at a quarter under the road's grip counts as wholly loose. [C, assumption]
- `grip_relief` [0.5, 0.75, 0.95]: first tried [0, 0.3, 0.6]. [M] That took the offroad bike's dirt corner from 1.04 g (what it did before, when the assist
  ignored the surface) to 0.89 g, which would feel like a loss; the values were raised until the offroad bike keeps its 1.0 g (0.98 of the grip on 0.65
  dirt) and the road bike slides earlier (0.86 g, half the penalty). [C]
- `damping` 1.4 at most: the damper's slow slope is the one that controls a washboard; a much larger factor makes the wheel follow the bumps with a heavier
  rebound than the spring clears before the next bump (a pogoing bike). 1.4 is a first choice, to be tuned against the washboard of section 6. [C]
- `tolerance` and `lean_hold` follow the landing help's numbers ([ride6-bike-jumps.md](ride6-bike-jumps.md), section 3.4): the same widening, scaled by
  looseness; the hold is 0.3 s at most because an offroad bike's lean controller needs about that to bring a bike back from a slide (the flip of the lean
  takes 0.65 s, [ride6-arcade-steering.md](ride6-arcade-steering.md), section 4).

## 4. Road bikes get the surface too

The road bike's relief is 0, but it benefits from the first help: it also stops asking the tyres for a lean the surface cannot give, so the road bikes
that fell on dirt now turn wide and stay up ([T] `no_class_of_bike_falls_in_a_dirt_corner_with_the_help`). That is the help working as intended: dirt is dirt
for everyone; the offroad bikes just lose less to it.

## 5. Reference mode is unchanged

`follow_surface` false, a relief of 0 (which returns the grip untouched), a damping factor of 1 (a product with 1) and a tolerance of 1 and a hold of 0
(`widen(limit, 1.0, cap) == limit`; `limit_duration + 0.0`): a bike with `ArcadeSteering::REFERENCE` takes exactly the steps it took. The surface's looseness
is still recorded, which changes nothing ([T] `the_reference_steering_rides_dirt_with_the_tyres_alone`).

## 6. The sweep and its numbers

`libs/blackbox-modding/src/content/bike_ride_air.rs`, `bike_rough_sweep` (ignored; `RIDE6_CARS=<folder>`): five offroad bikes (the first five of the class in
the folder) and three road bikes over a rough strip, a washboard of four 5 cm bumps 0.5 m apart (2 m) and one of 24 (12 m), and in a dirt corner (a stick of
0.7 for 6 s on the dirt of the table), at 40 and 80 km/h: the falls, the speed kept (the speed at the end over the speed at the start) and, for the corner,
the lateral g of the last two seconds. **Before** is `RIDE6_ARCADE=air_before` (the arcade steering as it was), **after** the default.

| bikes, speed | 2 m strip: falls / speed kept | 12 m strip: falls / speed kept | dirt corner: falls / speed kept / lateral g |
|---|---|---|---|
| offroad x5, 40 km/h, before | 0 / 1.02 | 0 / 1.02 | 0 / 1.02 / 1.04 g |
| offroad x5, 40 km/h, after | 0 / 1.02 | 0 / 1.02 | 0 / 1.02 / **1.02 g** |
| offroad x5, 80 km/h, before | 0 / 0.98 | 0 / 0.98 | 0 / 0.98 / 0.89 g |
| offroad x5, 80 km/h, after | 0 / 0.98 | 0 / 0.98 | 0 / 0.98 / **0.87 g** |
| road x3, 40 km/h, before | 0 / 1.01 | 0 / 1.01 | 0 / 1.01 / 1.04 g |
| road x3, 40 km/h, after | 0 / 1.01 | 0 / 1.01 | 0 / 1.01 / 0.86 g |
| road x3, 80 km/h, before | 0 / 0.99 | 0 / 0.98 | 0 / 0.99 / 0.88 g |
| road x3, 80 km/h, after | 0 / 0.99 | 0 / 0.99 | 0 / 0.99 / 0.73 g |

[M] `bike_rough_sweep` on the installed bikes (the first five offroad and three road bikes of the folder), release build, dirt of 0.65 lateral and 0.7 driving
grip. **The harness does not reproduce falls**: no bike fell on the washboards or in the corner (stick 0.7), before or after. Before, the assist ignored the
surface and every bike cornered at the road's g on dirt, which no tyre can give; the falls of the play test are not seen here. What the numbers show is the
change of feel: the road bikes now slide wider on dirt (0.86 g at 40 km/h and 0.73 at 80, from 1.04 and 0.88) and the offroad bikes keep their g (1.02 and
0.87), so on dirt an offroad bike corners 19 percent harder than a road bike at 40 km/h and 19 at 80. The strips (4 and 24 bumps of 5 cm) threw nothing and cost
no speed with the damper at 1.4.

## 7. Tests

`bike/rough_tests.rs` (a dirt corner at 40 and 80 km/h for each class, the offroad bike corners harder, the bike follows the surface, a washboard of 2 m
and of 12 m for each class and the offroad bike's speed, the damping on dirt and on tarmac, the reference on dirt), `bike/arcade_rough.rs`,
`bike/rough.rs`.

## 8. Not verified, open

- The harness has no fall to cure on dirt (section 6): the damper factor and the tolerances are first choices, untested against a fall.
- The host's surface table gives dirt and gravel their lateral multipliers from the original game's cars; those values are not read here (the tests use
  0.65); the real ones, and a sand and a grass surface, are checked at the build slot with the installed table.
- The tyre model's post-peak fall-off is the data's; the progressive breakaway of section 3.1 comes from the lean limit and the fall limit, not from a
  change of the tyre curve (a surface-specific curve would need the tyre's slip peaks scaled with the surface, which the data does not state).
- No in-scene check on a real dirt road of the city or of the RIDE 6 tracks.
