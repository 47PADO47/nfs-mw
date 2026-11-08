# RIDE 6 bikes: a wheelie from the throttle alone (a game-specific assist, not RIDE 6's)

The player's wish: "spamming the gas, or not, must lift and hold a wheelie easily". Continues [ride6-wheelie-hold.md](ride6-wheelie-hold.md) (section 1: with the weight
lever released no launch lifts the front wheel, 0 of 288 bikes; "if the players want the throttle alone to start a wheelie, the rider help's cut and the auto-rider's
forward weight under the throttle are the two things to open for a launch"). Tags: **[M]** measured on the model (the tests and the sweep below), **[R]** read in this
repository's code, **[C]** a choice of this project. **This is not the reference's behaviour**: the reference lifts a wheel with the weight lever and the throttle
([ride6-tricks.md](ride6-tricks.md), section 1). It is switchable and the reference's behaviour is the neutral value.

## 1. The mechanism: a virtual lever [C]

Nothing new lifts the front wheel. `libs/blackbox-vehicle/src/bike/wheelie_throttle.rs` decides, each step, what **weight lever the help works with**: the player's own, or the
lever fully back (-1) while the throttle asks for a wheelie. Every consumer of the lever then acts as it does with a real lever, and its measured behaviour carries over:

| Consumer (`trick.rs`) | With the virtual lever |
|---|---|
| the pitch assist's target (`pitch_assist::torque`) | `lever x throttle x MaxAngle` (35 degrees for the KTM) while the rear wheel is down and the front brake free |
| the forced-wheelie cut of the rider help (`interventions`) | the throttle is cut only from 33 to 45 degrees, not from 4 degrees up (the cut that kept every launch down) |
| the auto-rider (`rider_position`) | the rider's weight goes to the back (the player "has the weight") instead of forward under the throttle |
| the gentle lowering (`pitch_assist::lowering_torque`) | a throttle that is shut lets the nose down at 7 to 40 degrees a second, not 110 to 125 |

The engine and the tyres still do the lifting; the assist's torque is a fraction of the weight's (ride6-tricks.md section 0). A bike whose power-to-weight cannot lift its
front wheel stays down: the two Italjet Dragster 200 scooters (3 degrees at best).

**When it asks** (`ThrottleWheelie::GAME`, in `wheelie_throttle.rs`; the group of constants and the only place to tune it):

| Field | Value | Meaning |
|---|---|---|
| `throttle` | 0.9 | at or above this a wheelie is asked: a gas that is spammed or held flat out, not one that is rolled on |
| `lever` | -1 | the virtual lever (back); the target angle scales with it and with the throttle |
| `free_lever` | 0.2 | the real lever must be within this of its rest; a rider who moves it (wheelie or stoppie side) is in charge |
| `max_speed` | 40 m/s | no wheelie is started above 144 km/h; one that is up is held at any speed |
| `max_lean` | 0.5 | no wheelie is started with the lean past half its limit (a corner exit keeps the front down); the pitch assist's own lean fade acts as before |
| `hold_pitch` | 3 degrees | once asked, the lever stays back until the front wheel is down (pitch under 3 degrees) so a shut throttle lowers the nose gently and a blip lifts it again |
| front brake | under 0.1 | a front brake ends it (the stoppie side is the rider's) |

A bike with no wheelie control in its data (none of the 293) asks for nothing.

## 2. The switch and the neutral value [C]

- **Library**: `Bike::set_throttle_wheelie(ThrottleWheelie::GAME | ThrottleWheelie::REFERENCE)`. The library's default is `REFERENCE` (enabled false: the lever is the
  player's, exactly as before; `the_reference_values_give_the_reference_behaviour`), so every sweep and test of the lever keeps its numbers.
- **Game**: the setting `easy_wheelie` (default **on**; `config.toml`, `NFSMW_EASY_WHEELIE=off`, the console `set easy_wheelie off`, `crates/nfsmw/src/settings`), read at every step
  (`scenes/world/bike_assist.rs`), so a change reaches a bike on the road. It applies to the bike the player rides: a computer driver's bike (`collision_role.player`
  false) keeps the reference's behaviour. There is **no menu row**: the options menu is being reorganised by another change; the key is the switch.

## 3. Measured [M]

`cargo test -p blackbox-vehicle wheelie_throttle` (13 tests: the lever's rules, and the example supermoto and a superbike with the lever never touched): with `REFERENCE` a full
throttle launch keeps the front wheel down (under 10 degrees in 4 s); with `GAME` it lifts 10 degrees in under 1.5 s; the throttle held flat out for 12 s from 0, 10, 20 and 30
m/s never passes 55 degrees and never falls; a bang-bang throttle (on below 38 degrees) holds 25 to 48 degrees for 5 s from 10, 20 and 30 m/s; a shut throttle drops the nose
at under 46 degrees a second and the front wheel touches; a lever forward and a front brake are not overridden.

The sweep `bike_wheelie_throttle` (`libs/blackbox-modding/src/content/bike_ride_wheelie_throttle.rs`) over the **293 imported bikes**, the weight lever never touched, with the
assist off (`REFERENCE`) and on (`GAME`): a standstill launch at full throttle (8 s), the throttle flat out from 20 m/s (10 s), a 0.4 s on and 0.4 s off tap from 12 m/s, a spam of
0.7 s on and 0.4 s off from 8 m/s (12 s), a bang-bang throttle from 20 m/s (on below 37 degrees; "held" is 25 to 50 degrees over the last 5 s), and flat out at 15 m/s with
the stick a third over for 3 s, back, a third the other way (the steered wheelie).

| 293 bikes | assist off (reference) | assist on |
|---|---|---|
| front wheel up within 1.5 s of a flat-out launch | **0** | **276** (all but 2 within 4 s; the two Italjet Dragster 200 scooters never) |
| a wheelie held with a bang-bang throttle for 5 s | **0** | **289** (not: Honda X-ADV, the two Dragster 200, Suzuki Turbo Martek at 22 degrees) |
| falls (any of the six rides) | 0 | **0** |
| highest pitch of any ride | 6 degrees | **42** (flat out held for 10 s: 36 to 40 degrees, no loop-out) |
| steered wheelie (stick a third over) | 1 to 3 degrees | 36 to 42 degrees, no fall |

Twelve of them: flat-out launch (time to 10 degrees, highest pitch in degrees), flat out from 20 m/s (highest pitch), spam 0.7 s on and 0.4 s off from 8 m/s (pitch range of the
last 5 s), bang-bang:

| Bike | launch | flat out | spam | bang-bang |
|---|---|---|---|---|
| KTM 450 SMR | 1.3 s, 40 | 39 | 5 to 15 | 30 to 37 held |
| Ducati Panigale V4 S 2025 | 1.4 s, 40 | 39 | 7 to 17 | 30 to 38 held |
| BMW S 1000 RR 2019 | 1.4 s, 40 | 39 | 7 to 17 | 31 to 37 held |
| KTM RC 390 2023 | 1.3 s, 39 | 36 | 5 to 13 | 35 to 36 held |
| Kawasaki Z900 2021 | 1.3 s, 39 | 37 | 6 to 16 | 35 to 36 held |
| Suzuki Hayabusa 2021 | 1.3 s, 39 | 38 | 5 to 16 | 35 to 37 held |
| Honda CRF 450 R SMT 2024 | 1.3 s, 41 | 39 | 6 to 18 | 29 to 38 held |
| Yamaha YZF R1 2020 | 1.4 s, 40 | 39 | 6 to 15 | 30 to 38 held |
| KTM 690 SMC R 2026 | 1.4 s, 39 | 38 | 7 to 21 | 35 to 36 held |
| Honda X-ADV 2024 (a scooter) | 1.5 s, 36 | 4 | 0 to 3 | 1 to 3 lost |
| Suzuki Turbo Martek | 1.2 s, 41 | 40 | 0 to 3 | 22 to 38 lost |
| Italjet Dragster 200 | never, 3 | 3 | 0 to 2 | 3 lost |

All with the assist off: launch never 10 degrees (1 to 3 at best), flat out 1 to 3, spam -1 to 2, bang-bang 0 to 2 lost. Falls before and after: 0 and 0 of 293 (the sweep has the
stick in it).

**What it does not do** [M]: a short tap does not lift much because the thrust of a few tenths of a second does not (the 0.4 s tap at 12 m/s reaches 2 to 3 degrees on every bike,
as a blip of the lever-back wheelie did in [ride6-wheelie-hold.md](ride6-wheelie-hold.md), section 4); a spam of 0.7 s on and 0.4 s off from 8 m/s keeps the nose between 5 and
21 degrees; the throttle held gives 36 to 42. A taller spammed wheelie needs more torque than the assist's (a physical limit the assist does not cheat).

**What it costs** [M]: a bike held in a wheelie is slower than one kept down, as with the real lever: the rider help cuts the throttle from 33 to 45 degrees to hold the nose at about 38,
so a flat-out launch from a standstill reaches **80 km/h in 8 s on the KTM 450 SMR instead of 137** (113 instead of 196 on the Panigale V4 S, 120 instead of 176 on the Z900), and
in the world (`view-world --drive KTM450 --at 985,4430`, 6 s of full throttle) 35 km/h in first gear instead of 73 in second. A player who wants to accelerate eases the throttle under
0.9 (a pad's trigger) or taps it off, or switches the setting off. The lever's strength is not a way to lower this: with the virtual lever at -0.5 or -0.65 (`RIDE6_TW_LEVER`) the bikes
do not lift at all (the help's forced-wheelie rule needs the weight beyond its dead zone, 0.75), and -0.8 or -0.9 lift to the same 38 degrees.

## 4. Choices that remain

- Flat out (0.9) is the trigger, so a launch with a pad's trigger fully pressed lifts the front: that is the request. Rolling the gas on does not. `ThrottleWheelie::throttle`
  is the knob; 1.0 asks for a throttle at the stop.
- The tuck-in rule (`tuck_requested`) sees the virtual lever like the real one: a rider whose weight is back does not tuck.
- The rider's animation follows the weight (he sits back) because the auto-rider's weight follows the virtual lever.
- Not changed: the rider help's cut and the auto-rider for any other case, the reference's lever behaviour, `choices.rs`, the lean and steering files.

## 5. To check

```
cargo test -p blackbox-vehicle wheelie_throttle
RIDE6_CARS=<folder of manifests> [RIDE6_FILTER=a,b] [RIDE6_TW_LEVER=-1 RIDE6_TW_THROTTLE=0.9] cargo test --release -p blackbox-modding --features host --lib bike_wheelie_throttle -- --ignored --nocapture
nfsmw view-world --drive KTM450 --muted --camera-mode chase --drive-script "3:throttle=1;5:throttle=0;2:throttle=1;1:throttle=0" --screenshot out/wheelie.png --screenshot-delay 3 --screenshot-interval 0.5 --screenshot-count 20
```
