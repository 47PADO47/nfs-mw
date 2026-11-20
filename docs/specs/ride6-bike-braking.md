# RIDE 6 bikes: braking, round 4 (this game's own tuning)

**This is a modification made exclusively for the NFS:MW rewrite. It is not the behaviour of RIDE 6.** It adds the **brake help**,
`blackbox_vehicle::bike::ArcadeBrake` (`libs/blackbox-vehicle/src/bike/arcade_brake.rs`, the force and the hold in `brake_help.rs`), as a field of
`ArcadeSteering` (`brake`; [ride6-arcade-steering.md](ride6-arcade-steering.md), lever 11). `bike_steering = reference` (`ArcadeBrake::REFERENCE`, every
field neutral) gives the original brakes exactly. Tags: **[R]** read in this repository's code, **[C]** a choice of this project, **[M]** measured in the
model, **[U]** not measured.

## 1. What the player reported

Playtest, round 4: "bikes should brake way more". The brief gave the numbers: a mean deceleration with the levers fully pulled of at least 1.3 g on
tarmac for a road bike (1.0 g for a supermoto or an offroad bike, scaled by the surface's grip; 0.9 g for a scooter), a stop from 100 km/h in about 30 m
and from 200 km/h in about 120 m, no fall and no uncontrolled stoppie, and the original behaviour kept for the anti-lock and the lock-up, the standing
hold ([ride6-bike-standstill-hold.md](ride6-bike-standstill-hold.md)) and the wheelie and burnout logic.

## 2. Before [M]

`bike_brake_table` (`libs/blackbox-modding/src/content/bike_ride_brake_table.rs`, an ignored test) rides 12 imported bikes (`RIDE6_FILTER`): a Yamaha
YZF-R1 2020, a Ducati Panigale V4 S 2025, a BMW S 1000 R, a KTM 1290 Super Duke R and an Indian Challenger RR (road: superbikes, nakeds, a bagger),
a KTM 690 SMC R and a Honda CRF 450 R SMT (motards), a BMW R 1300 GS and a KTM 890 Adventure R (offroad), a Honda X-ADV and two Italjet Dragster 200
(scooters), in the game's arcade steering, upright, a second at the speed, then both levers fully pulled with no throttle until the bike stands. Peak is
over 0.1 s; mean is the speed over the time to stop; **rear up** is the longest time of a bike with its rear wheel off the road, **nose** the lowest pitch.
"Before" is the same run with `RIDE6_ARCADE=brake=0` (the log `C:\r6work\fix3\brake-before.log` was taken before the help existed and is the same).

| kind (bikes) | km/h | peak g | mean g | stop m | rear up s | nose deg | falls |
|---|---|---|---|---|---|---|---|
| road (5) | 60 / 100 / 160 / 220 | 0.96 / 0.97 / 1.07 / 1.21 | 0.87 / 0.90 / 0.94 / 0.99 | 17.5 / 43.7 / 103 / 178 | 0.22 to 0.37 | -3.4 | 0 |
| motard (2) | 60 / 100 / 160 / 220 | 0.85 / 0.92 / 1.07 / 1.30 | 0.78 / 0.82 / 0.89 / 0.96 | 18.4 / 46.3 / 105 / 169 | 1.8 / 3.1 / 4.8 / 6.0 | -6.2 | 0 |
| offroad (2) | 60 / 100 / 160 / 220 | 0.84 / 0.86 / 1.00 / 1.19 | 0.78 / 0.80 / 0.85 / 0.92 | 19.1 / 48.5 / 111 / 184 | 0.2 to 0.37 | -5.3 | 0 |
| scooter (3) | 60 / 100 / 160 / 220 | 0.86 / 0.89 / 1.02 / 1.19 | 0.80 / 0.82 / 0.87 / 0.92 | 18.6 / 48.1 / 110 / 185 | 1.7 / 3.0 / 4.7 / 6.1 | -4.5 | 0 |

The reference bikes stop at 0.8 to 0.9 g (the peak only reaches 1.0 g and more at 160 km/h and over, with the air drag), 44 to 48 m from 100 km/h: no
bike gets near the brief's 30 m. The motards and the scooters ride a **stoppie for most of the stop** (the rear wheel is off the road for 1.8 s from
60 km/h, 6 s from 220), at 4.5 to 6 degrees nose-down: the front brake's torque and the short wheelbase lift the tail, the nose stays down and the
tyre's grip is used up on the front wheel alone.

## 3. The help

Two parts, both in the arcade steering so `bike_steering = reference` skips them.

**The force** (`ArcadeBrake::target`, `extra`; `Bike::apply_brake_help`). A deceleration force at the centre of gravity, along the road against the
velocity, with no torque, that makes up what the tyres do not give to the deceleration the levers aim for:

```text
mix     = 0.7 x front lever + 0.3 x rear lever                         the levers as the joined brakes make them (brakes::joined), less the lean relief
target  = max_decel_g[class] x g x mix x surface                       the deceleration aimed for in all, the tyres' and the help's
tyres   = - (front traction + rear traction) / mass                    what the two tyres put along the road in the last sub-step
extra   = clamp(target - tyres, 0, gain x g) x fade(speed, throttle)   the help
```

- **The tyres are left alone**: the force is not a brake torque, so the slip of each tyre, the anti-lock's release (`AntiLock`, `bike_slide.rs` reading the
  tyre's slip) and the squeal are what the tyres do. A bike that skids still skids; a wheel that locks locks. The help makes up whatever the tyres
  lack, so it **never takes away** (tyres that already give the target get none) and it never brakes a bike more than `target`.
- **The class** is `BikeSpec::terrain_class` (road, motard, offroad) with the scooter taking its own number whatever its class; the scooter test is the
  rear wheel's outer radius under `SCOOTER_REAR_WHEEL` = 0.30 m [R]: over the 293 imported bikes the Italjet Dragster 200 (0.249 m), the Dragster 700
  (0.286 m) and the Honda X-ADV (0.286 m) are under it and every other bike is over 0.306 m, a clean gap.
- **The surface**: `surface` is the lateral grip multiplier the bike believes in (`surface_factor`, `rough.rs`; the class's relief applied: an offroad
  bike keeps 98 percent of the grip on gravel, a road bike 80 percent), 1 on tarmac, so the target falls on loose ground exactly as the cornering
  assist does. It is **not** the raw surface multiplier: a road bike on gravel (0.6) still aims for 1.12 g.
- **The fade**: none under 2 m/s (the standing hold's, `hold.rs`, `LEAVE_SPEED` 1.2 m/s), whole over 6 m/s; gone with the throttle between 0.1 and 0.5 (a
  rider on the throttle does not brake); none in a burnout (brake and throttle), in a fall, with the rider off the bike, while the rider pushes the bike
  back, and with both wheels in the air. The levers are relieved in a lean like the brake torque (`choices::BRAKE_LEAN_RELIEF`): a bike at its lean limit
  gets 20 percent.

**The pitch hold** (`ArcadeBrake::pitch_hold`). The tyres' longitudinal force acts at the road, below the centre of gravity, and so pitches the bike nose
down; that is the weight transfer and, on a short bike, the lifted tail. The hold applies `pitch_hold` (0.5) of that moment, `sum((contact - cog) x
(heading x traction))` about the road's axis across the heading, the other way: a bounded torque (at most half the tyres' own), proportional to the braking,
that keeps the rear tyre loaded and shortens the dive. It is **let go as the rider shifts his weight forward** (`weight_release` 0.2 to 0.5 of the
lever's travel, `trick.weight`), so a stoppie of his own is possible: the example bike with the weight forward still goes to -15 degrees with the rear wheel
up for two seconds (`a_rider_who_shifts_his_weight_forward_still_lifts_the_rear_wheel`). The hold fades out with the lean (`1 - lean / limit`): measured
without it, a leaned slow turn with the brake touching turned 4 percent wider.

## 4. Constants [C]

| constant | value | what and why |
|---|---|---|
| `ROAD_DECEL_G` | 1.4 g | the *steady* deceleration aimed for; the lever's rise (a third of a second) and the last metres cost the mean 0.1 g, so 1.4 gives the mean of 1.3 g and 30 m from 100 km/h that the brief asks (1.3 aimed gave 1.24 g and 31.7 m) |
| `MOTARD_DECEL_G`, `OFFROAD_DECEL_G` | 1.0 g | the brief's 1.0 g, scaled by the surface |
| `SCOOTER_DECEL_G` | 0.9 g | the brief's 0.9 g |
| `GAIN` | 0.7 g | the most extra: the worst shortfall measured is 0.55 g (a road bike at 0.85 g aiming for 1.4 g) |
| `PITCH_HOLD` | 0.5 | with 0.0 the motards and the scooters lift the tail for 2 to 5 s; with 0.5 for 0.35 s at most (the lever's first instant), 0.8 gave no more (the first 0.2 s is the suspension's rebound) |
| `FRONT_SHARE` | 0.7 | the front brake does most of the braking; a front lever alone aims for 0.7 of a full brake (the joined brakes add the rear lever of the bikes that have them: the measurement with the front lever alone equals the full one) |
| `speed` | 2 to 6 m/s | the force is the standing hold's under 2 m/s and the tyres' alone at the very end of a stop |
| `throttle` | 0.1 to 0.5 | the force goes with the throttle |

## 5. After [M]

The same 12 bikes, both levers fully pulled (log `C:\r6work\fix3\brake-final.log`):

| kind (bikes) | km/h | peak g | mean g | stop m | rear up s | nose deg | falls |
|---|---|---|---|---|---|---|---|
| road (5) | 60 / 100 / 160 / 220 | 1.44 / 1.48 / 1.58 / 1.73 | **1.20 / 1.30 / 1.38 / 1.46** | **12.3 / 29.9 / 70 / 122** | 0.20 | -3.3 | 0 |
| motard (2) | 60 / 100 / 160 / 220 | 1.06 / 1.14 / 1.29 / 1.53 | **0.95 / 1.02 / 1.09 / 1.17** | 14.8 / 37.0 / 85 / 139 | 0.35 | -5.8 | 0 |
| offroad (2) | 60 / 100 / 160 / 220 | 1.05 / 1.13 / 1.25 / 1.46 | **0.95 / 1.00 / 1.07 / 1.15** | 15.2 / 38.1 / 88 / 149 | 0.0 | -5.1 | 0 |
| scooter (3) | 60 / 100 / 160 / 220 | 0.96 / 1.00 / 1.13 / 1.31 | **0.88 / 0.92 / 0.97 / 1.04** | 16.7 / 42.7 / 98 / 165 | 0.25 | -3.7 | 0 |

- A road bike stops from 100 km/h in **29.9 m** (before 43.7 m) with a mean of **1.30 g**, and from 200 km/h in about 100 m (122 m from 220 km/h, the
  brief's 120 m at 200 km/h is met with room: the help stops a little shorter, the air drag does the rest). At 60 km/h the mean is 1.20 g: the lever's
  rise is a larger share of a short stop. The supermotos and the offroad bikes stop at 1.0 g, the scooters at 0.9 g.
- **No falls** on any of the 293 imported bikes, from 60 and from 220 km/h (`RIDE6_EVERY=1 RIDE6_KMH=60,220`, log `brake-all.log`): rear wheel up at most 0.38 s
  (the first instant), nose at most -5.9 degrees. The long stoppies of the motards and the scooters (2 to 6 s) are gone.
- On gravel (`RIDE6_SURFACE=0.6`, grip 0.6 on both axes, log `brake-final-gravel.log`): road 1.08 g mean at 100 km/h (35.8 m), motard 0.94 g, offroad
  0.99 g (the class's relief keeps their grip), scooter 0.80 g; no falls.
- With the front lever alone the stop is the same (the joined brakes), with the rear lever alone 0.46 g at 100 km/h.
- With the help switched off (`RIDE6_ARCADE=brake=0`) the table of section 2 comes back bit for bit.

## 6. Tests

`blackbox-vehicle` (`brake_help_tests.rs`, `arcade_brake.rs`, `class.rs`): the reference adds nothing; the target follows the levers, the kind of bike and
the surface; the help makes up the shortfall to the gain and never takes away; the force comes in with the speed and goes with the throttle; the hold
is let go with the weight forward; a full brake stops a road bike in about 30 m from 100 km/h where the reference needs 45 m; 200 km/h in 125 m at most;
the rear wheel stays down (under 0.5 s) and the nose under 8 degrees at 60, 100, 160 and 220 km/h; the hold keeps the rear wheel down longer than the
force alone; a rider with his weight forward still lifts the rear wheel; the tyres give what they gave (within -10 and +30 percent: the hold keeps the
rear tyre loaded, so it brakes a little harder); a scooter aims lower, the throttle takes the force away; a bike that stands is left alone; no lever,
no help. The older suites cover the standing hold, the burnout, the ABS and the wheelie logic and pass unchanged.

## 7. How to switch it off

- `bike_steering = reference` (config, `NFSMW_BIKE_STEERING=reference`, console `set bike_steering reference`): `brake: ArcadeBrake::REFERENCE`.
- In the library: `ArcadeSteering { brake: ArcadeBrake { gain: 0.0, ..ArcadeBrake::default() }, .. }` keeps the hold only; `pitch_hold: 0.0` keeps the force only.
- In the harness: `RIDE6_ARCADE=brake=0|brakegain=|brakeg=|brakegmoto=|brakegoff=|brakegscooter=|hold=`.

```text
RIDE6_CARS=<folder> RIDE6_FILTER=yzf-r1-2020,ktm_690,... [RIDE6_KMH=60,100,160,220] [RIDE6_SURFACE=0.6] [RIDE6_LEVERS=front|rear] [RIDE6_ARCADE=...] \
  cargo test --release -p blackbox-modding --features host --lib bike_brake_table -- --ignored --nocapture
```

## 8. What is not verified

- Only the model: the sound (a longer squeal at 1.4 g), the rider's animation under a harder brake and the camera's pitch were not looked at; the
  force does not pitch the bike (it acts at the centre of gravity), so the dive is the tyres' and the hold's.
- A bike on a slope brakes against the slope's component as before (the force is along the road); a downhill stop is longer than a flat one.
- The scooter test is the rear wheel's radius, so the Honda X-ADV is a scooter and a hypothetical modded scooter with a big rear wheel is a road bike.
- The 60 km/h mean (1.2 g) is under the brief's 1.3 g: the lever's rise (the data's input rate) costs a larger share of a short stop; aiming higher would
  make a 100 km/h stop under 28 m.
