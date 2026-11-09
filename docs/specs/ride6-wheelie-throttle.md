# RIDE 6 bikes: a wheelie from the throttle alone (a game-specific assist, not RIDE 6's)

The player's wish: "spamming the gas, or not, must lift and hold a wheelie easily". Continues [ride6-wheelie-hold.md](ride6-wheelie-hold.md) (section 1: with the weight
lever released no launch lifts the front wheel, 0 of 288 bikes; "if the players want the throttle alone to start a wheelie, the rider help's cut and the auto-rider's
forward weight under the throttle are the two things to open for a launch"). Tags: **[M]** measured on the model (the tests and the sweep below), **[R]** read in this
repository's code, **[C]** a choice of this project. **This is not the reference's behaviour**: the reference lifts a wheel with the weight lever and the throttle
([ride6-tricks.md](ride6-tricks.md), section 1). It is switchable and the reference's behaviour is the neutral value. **[Playtest round 2, section 6: the assist is slower, only in
the first three gears with caps of 40, 35 and 30 degrees, dips at a gear change, and is latched off after the front wheel is put down.]**

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

## 3. Measured, first version [M] (the numbers are those before the playtest round 2; the current behaviour is section 6)

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

## 6. Playtest round 2: slower, in three gears, a dip at the shift, and a latch [C]

The player's report: "bike wheelie too easily/autonomously. manual wheelies are ok currently, but if just press gas the bike wheelies automatically. ok but: it should happen slower, only in first (max 40
degrees), second (max 35 degrees) and third (max 30 degrees). max ignored if wheeling manually. also when the gear changes it should go down a little before going up (simulate clutch that stops gas).
also if I put it down it then shouldn't do it again automatically (unless I go back to first or stop)." Sections 1 to 5 describe the first version; this is what the throttle-only wheelie does now. The
group `ThrottleWheelie` gained the fields below (`REFERENCE` has them all at 0, so the reference behaviour is unchanged); the state lives in `wheelie_throttle.rs` (`State`, `update`).

**The allowed angle.** Besides the virtual lever the assist keeps one number, the angle it may lift to now (`State::reach`). It is the **ceiling of the pitch assist's target** (`PitchInput::reach_limit`) and
an **engine cut** when the nose is over it (`Lever::cut`, folded into the rider help's electronic cut), so it holds a nose that the assist alone cannot (a launch below 15 km/h, where the assist does not act). It
moves like this each step:

| what | rule | field, `GAME` |
|---|---|---|
| gears | the cap of the gear in use: first 40, second 35, third 30 degrees; in a higher gear **no wheelie is asked** (a wheelie that is up when the gear goes beyond the third is let down gently: the allowed angle falls to 0) | `gear_caps` 0.698, 0.611, 0.524 rad |
| the rider's own lever | the cap **does not apply**: a real lever (beyond `free_lever`) or a front brake hands the bike back to the rider and the allowed angle is infinite | `free_lever` 0.2 |
| the rise | the allowed angle rises at 8 degrees a second from the angle the nose has when the wheelie starts: 10 degrees take 1.25 s, the cap of first gear 5 s (before: 10 degrees in 1.3 s and the whole 38 in 3) | `rise_rate` 0.14 rad/s |
| the fall | it comes down at 34 degrees a second (to a dip, to a lower gear's cap, to nothing in a gear without one) | `fall_rate` 0.6 rad/s |
| the lead | it is never more than 6 degrees above the nose: a nose that lags (a bike that cannot lift, a wheelie just let down) is not run away from, so the rise always starts from the nose | `lead` 0.1 rad |
| the dip | a gear change (`Powertrain::shifting`, the clutch is out) lowers it by 8 degrees below where it was (or below the cap, if that is lower) for the change **and at least 0.3 s** (the bikes' own change takes 0.06 s, less than the nose needs to answer), then it rises again at the slow rate | `shift_dip` 0.14 rad, `shift_dip_time` 0.3 s |
| the engine cut | the drive is cut from 3 degrees above the allowed angle, to 70 percent at 12 degrees above | `cut_over` 0.052, 0.21 rad, `cut_most` 0.7 |

Why these numbers: the three caps are the player's. 8 degrees a second is slower than every lift the first version made (10 degrees in 1.3 s was the engine's own pace under the assist) and still reaches the cap of
first gear in 5 s, which a standing start spends in first and second; 34 degrees a second is the pace of the gentle lowering of the data (7 to 40 degrees a second, section 1); a dip of 8 degrees is the "a little" of
the player (the nose comes down 3 to 8 degrees in the 0.3 s); the cut is soft (70 percent, over 9 degrees) so that the launch is still quick. The unit test
`the_lift_is_slower_than_it_was` measures the time to 25 degrees against the first version's behaviour (`unlimited()`: the same assist without the rise, the caps, the dip and the cut) and wants at least 1.3 times it plus 0.2 s.

**The latch.** Once the front wheel has been put **down**, the throttle asks for no more wheelies. "Put down" is a wheelie of the throttle that had really gone up (the front wheel off the road and the pitch over 3 degrees)
and ended by: the front wheel touching the road (also with the gas still down), the nose under 3 degrees with the throttle shut, the rider's lever or the front brake, or a gear beyond the third. The latch
lets go when the bike **enters first gear** (from the second: a downshift, or the standing start's first change back) or **stops** (speed under 1 m/s). A wheelie that never went up (the gas lifted before the nose
rose) latches nothing, and a rider's own wheelie that the throttle never joined latches nothing. So: launch, wheelie, shut the gas, front down: pressing the gas again at the same gear does nothing; a stop, or first
gear again, arms it. (`rearm_speed` 1 m/s.) Spamming the throttle still keeps a wheelie that is up alive (the virtual lever holds while the nose is over 3 degrees) as in section 1.

**A wheelie can only start under its cap**: pitch under the cap of the gear, the lever at rest, throttle 0.9, speed under 40 m/s, lean under half the limit; the rest of the conditions of section 1 are unchanged.

**Tests.** `wheelie_throttle_tests.rs` (the rules, as pure functions of `State` and `Ask`): the cap of each gear is reached and never exceeded, no wheelie in the fourth gear and above, a wheelie up when the gear goes beyond the third
comes down and is put down, the cap does not touch the rider's lever, the rise is 8 degrees a second, the allowed angle does not run from a nose that stays low, the dip (from the cap and from a lower angle, never
deeper than `shift_dip`) and the rise after it, a lower gear's cap is reached at the fall rate, the engine cut's ramp, the latch (touchdown, lever, front brake) and its release (first gear, a stop; not by staying in first),
a wheelie that never went up latches nothing, blips keep it alive. `wheelie_throttle_sim_tests.rs` (the example supermoto and a superbike, the bike's own gearbox): the lift from a standstill, slower than before, never
more than 6 degrees over the cap of the gear (a second and a half after a change left out), no lever in the fourth gear, the rider's lever wheelie goes above the cap of the third gear where the assist's does not, the nose goes
down at a gear change and up again, and the front put down is not lifted again by the gas. The sweep `bike_wheelie_throttle` over the imported bikes also reports, per bike, how far over the cap of its gear the assist's
wheelie went; its "hold" column now reads a bang-bang throttle from a standstill (the pitch between 8 and 46 degrees from 2 s to 9 s) because the assist only works in the first three gears.

**Measured** [M]: to be filled in from the build slot: the 293-bike sweep with the assist on (lift in 3.5 s, falls, the highest pitch, bikes over a cap by 6 degrees) and the printed times of the lift test.
