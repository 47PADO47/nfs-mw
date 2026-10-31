# RIDE 6 rider weight lever at every speed, round 9 (clean-room spec)

What the rider's longitudinal weight lever (`Actions.WeightLongitudinal`, -1 back to 1 forward) does in the reference (build
`5.6.1-1697004`) in every state of the bike, why it feels dead at speed, what this game adds around it, and what a hard brake does
with it. It corrects nothing of [ride6-tricks.md](ride6-tricks.md); it adds the state-by-state reading, the tuck rule's lever term
and the pad's lever. Function addresses: [provenance](../provenance/ride6-weight.md). Tags: **[B]** binary, **[D]** data, **[I]**
inferred, **[C]** a named choice of this game. Code: `libs/blackbox-vehicle/src/bike/{trick,trick_spec}.rs`,
`crates/nfsmw/src/input/weight_axis.rs`; diagnostics: `libs/blackbox-modding/src/content/bike_weight_sweep.rs`.

Player report that started the round (pad, left stick Y = weight, X = steering): "at speed, or accelerating, I cannot pull the weight
forward to be tucked; slowing down I cannot pull it back to avoid a stoppie; and a really hard brake from speed should make the bike
do a very small stoppie by itself, about 20 degrees."

## 1. The lever in the reference

Two functions turn the lever into the rider's position `RiderAidPosition` (the position the rider component takes) every step.

**Which side owns the weight** [B: `0x145110af0`]. With `W` the raw lever (`bCombineWeightPositionInput`, off in every asset, would add the
rider-position input) and `D = WeightLongitudinalInputDeadzone` (0.75 for every bike, `Base_RiderHelp` [D]):

```
|W| <= D :  the auto-rider owns the weight                     (flag set)
|W| >  D :  the player owns it:  aim = W * ManualRiderForwardPosition  (W > 0)
                                       -(W * ManualRiderBackwardPosition)  (W <= 0)
```

`ManualRiderForwardPosition` is 1 and `ManualRiderBackwardPosition` -1 by constructor default (`0x50fda20`) and no asset sets them, so
**the player's aim is the lever itself**, on both sides. [B, D]

**Where the position goes** [B: `0x1450ff610`]. The position approaches its aim with the square-root profile (`approach`, the one of
[ride6-tricks.md](ride6-tricks.md) section 2) at `ManualPositionEngageSpeed` (5 per second for the player's aim) or
`AutoRiderPositionEngageSpeed` (5 for the auto-rider's); the input channel behind it (`WeightLongitudinal`, 6 per second for every bike
[D]) then moves the rider's target place. A swing from one end to the other takes about 0.3 s.

**The auto-rider's aim** (inside the dead zone) is section 5 of [ride6-tricks.md](ride6-tricks.md):

```
rest = throttle_curve(throttle) + brake_curve(front_brake)    # (0,0)-(0.1, 1) and (0,0)-(0.1, -1) in Base_RiderHelp [D]
aim  = rest, pushed to +-1 by a lift of 3.6 to 4 degrees (forward against a wheelie, back against a stoppie)
```

So the throttle past 10 percent puts the rider fully forward and the front brake past 10 percent fully back, whatever the lever says
inside +-0.75. Nothing in the data makes the auto-rider weaker at speed.

## 2. What that means in each state (the lever reads dead because it mostly is)

| State | Who owns the weight | What the lever can do |
|---|---|---|
| Accelerating (throttle over 0.1), lever -0.75..0.75 | auto-rider: **forward** (+1) | nothing: the weight is already forward. A lever at -0.5 does not move it back, it is overridden to +1 |
| Accelerating, lever below -0.75 | player | weight back; with the throttle it asks a wheelie (`pos = -ramp(-W; 0.3 .. 1)`, [ride6-tricks.md](ride6-tricks.md) section 1) |
| Accelerating, lever above 0.75 | player (weight forward, as the auto-rider has it) | nothing physical; **the reference cancels the tuck** (section 3) |
| Braking (front brake over 0.1), lever -0.75..0.75 | auto-rider: **back** (-1) | nothing: the weight is already back; a lever at +0.5 is overridden to -1 |
| Braking, lever below -0.75 | player, back | the same as the auto-rider: **pulling back cannot reduce a stoppie any further** |
| Braking, lever above 0.75 | player, forward ("asked") | the rider help's brake relief starts at 30 degrees instead of at the first degrees of lift (section 5) |
| Braking, lever above 0.95 | player, forward | and the stoppie help holds the angle `ramp(W; 0.95 .. 1) x 45 degrees` |
| Coasting, lever inside +-0.75 | auto-rider: rest 0 | the weight is 0 unless a lift pushes it |

The tests `inside_the_dead_zone_the_auto_rider_owns_the_weight_forward_under_the_throttle_and_back_under_the_brake` and
`pulling_the_lever_back_ends_the_stoppie_within_half_a_second` (`tests_tricks.rs`) pin the rows; the diagnostics
`bike_weight_accel` / `bike_weight_brake` print them for the installed bikes.

**What the weight moves** [D]: the rider's place is `BaseOffset + (BackwardOffset x -W  or  ForwardOffset x W)` (centimetres in the
bike frame, [ride6-rider-body-aero.md](ride6-rider-body-aero.md)). Over the 288 bikes: `ForwardOffset` is 0 for every handler (the
constructor default; no asset sets it) and `BackwardOffset` is 22 cm back for the default handler (268 bikes), 10 cm for the scooters
and 0 for the KTM's Motard and the Maxi Enduro handlers (18 bikes). So **pulling the weight forward moves no mass on any bike**, and
back moves 70 kg by 22 cm (the combined centre of gravity by 8 cm, which raises the load transfer at which the rear unloads by a tenth
of a g). The lever's real work is the three things above: who the rest position belongs to, the pitch help's request (wheelie,
stoppie) and the tuck.

## 3. Tuck-in and the lever

The tuck action (`Actions.TuckIn`, 1 or 0) [B: `0x14512b870`] is held above 0.5 by a tuck input, or by the rule `bAutoTuckIn`: faster
than `AutoTuckInMinSpeed` (90 km/h), the normalised lean under 1, the raw throttle over `AutoTuckInMinThrottle` (0), the raw front brake
under `AutoTuckInMaxBrake` (0.1) **and the raw lever inside the dead zone** (`|W| < D`; the compare is `D <= |W|` to bail out). The tuck
level the animation reads rises at 2 per second while the action is 1 and falls at 2 per second otherwise (`ride6-rider-round6.md`,
section 2). The tucked rider's weight is not moved: `TuckInPosition` is 2 (outside -1..1, so `TuckInWeightLongitudinalRange` is used)
and that range is its constructor default (-1, 1), which the function returns on without writing [B, D].

So **in the reference a lever pushed forward past 0.75 at speed stands the rider up**, on either side of the lever the tuck is
cancelled, and the only way to be tucked is a lever inside +-0.75 (typically released) or a tuck input, which no device here has. A
pad player who pushes the stick forward to get down behind the screen gets the opposite.

**This game [C]:** the lever forward does not cancel the tuck. `AutoTuckSpec::forward_weight_tucks` (manifest
`[bike.wheelie.help.auto_tuck] forward_weight_tucks`, default true; false is the reference's rule) makes only a lever pulled back past
the dead zone cancel it, because that is the wheelie side where the player takes the weight from the rider help. The other terms
(speed, lean, throttle, brake) are unchanged, so a lever forward under the front brake is still a stoppie, not a tuck. The change is
cheap to undo and has no effect on the physics except the rider's air drag and the animation's `TuckedIn`.

| KTM 450 SMR, 30 m/s, full throttle for 4 s | lever -1 | -0.5 | 0 | 0.5 | 0.8 | 1.0 |
|---|---|---|---|---|---|---|
| tuck level, the reference's rule | 0 | 1 | 1 | 1 | **0** | **0** |
| tuck level, this game | 0 | 1 | 1 | 1 | **1** | **1** |
| weight after 4 s | -1.0 | +1.0 | +1.0 | +1.0 | +0.8 | +1.0 |

(`bike_weight_accel`; the lever -0.5 row is the auto-rider overriding the player, section 2. At 30 m/s the lever -1 row is a wheelie,
pitch 37 degrees, the help's 35 degrees plus the front lifting.)

## 4. The pad's lever [C]

The reference's physics reads the lever as it is; how a device reaches it is the game's input layer (the `IgnitionPlayerAction-
HelperEvaluator` and its `WeightLongitudinalConfiguration` curves are blueprint data that were not read). The help tests the lever
against 0.75 and 0.95, so this game's pad lever (`input/weight_axis.rs`) is:

1. the stick's circular gate opened to a square (a stick at the rim on a diagonal gives the lever its end);
2. the 15 percent dead zone of the pad taken off and the rest rescaled;
3. **`REACH` 0.9: the lever reads 1 once the stick is 0.9 of the way past the dead zone**, so the lever's 0.95 is at 0.88 of the raw
   travel (it was at 0.96) and its 0.75 at 0.72 (it was 0.79). A thumb that holds the stick against the gate at 0.9 now does the
   stoppie help's full lever.

`REACH` is the largest share a steady thumb holds on the pads in use; it is a named choice and one constant.

## 5. Hard braking from speed

**What limits the front brake.** The ABS ([ride6-rider-help.md](ride6-rider-help.md), [ride6-tricks.md](ride6-tricks.md) section 4)
releases the front brake as a ramp of the tyre's normalised slip `nx` (-1 is the force peak) from -0.3 to -0.5 [D:
`Motard_ABSVehicleElement`, no wet term on tarmac; the same for every bike read], and the tyre's force at `nx` = -0.3 to -0.35 is about
0.7 of its peak (KTM front tyre at 1.8 kN: 1570 N of 2220 N). The brake settles where the
released brake force equals the tyre's force there: **about 0.9 to 1.0 g for the bikes of the install**, whatever the brake's force
(the KTM's 1880 N front brake is not the limit). A bike lifts its rear wheel when the deceleration `a` passes `g b / h`, the distance
`b` of the combined centre of gravity from the rear patch over its height `h`: 0.9 g for the KTM, about 1.05 g for the sport bikes of the table. So at
the ABS's level 1 **a sport bike's rear wheel stays down in the hardest stop**, and the KTM's lifts about one degree.

**What the rider help does when it lifts.** The brake relief ([ride6-tricks.md](ride6-tricks.md) section 4) cuts the front brake pedal by
`ramp(lift; AutoRiderStoppieBrakeControl)` (0 to 4 degrees for the Motard bike, 1 to 8 degrees for the other 270 [D]) unless the rider
"asks" (lever over 0.75 and the brake over 0.1: then `AutoRiderForcedStoppieBrakeControl`, 30 to 90 degrees). A bike that could brake
harder than it lifts settles where the relief takes the excess away, so **the automatic stoppie is bounded by the relief range**: at most
the end of it, in practice `start + r (end - start)` with `r` the excess share (a bike 10 percent past the lift threshold: 1.7 degrees
on the base range). The ABS level also multiplies the relief's start (1, 0.75, 0.5, 0, 0 at the levels 0 to 1, [D]) and the front ABS
(0.1, 0.2, 0.45, 1, 1).

**Measured on 21 of the 288 bikes** (every 14th of the install; `bike_weight_brake`, a stop from 30 m/s with both brakes fully on, flat
road, 2.5 s; the rear lift is the angle of the line through the hubs against its rest value while the rear tyre is off the road; the
dive is the body's nose-down pitch with the fork; "auto" is the lever at 0 and also the lever at -1, which gives the same numbers
because the auto-rider is already fully back under the brake; 0.85 is the lever in the asked band, 1.0 the stoppie help's full lever):

| bike | dive (deg) | decel (m/s^2) | ABS 1: auto, 0.85, 1.0 | ABS 0.5: auto, 0.85 | ABS 0: auto, 0.85, 1.0 |
|---|---|---|---|---|---|
| aprilia_rs-250-2001 | 3.4 | 9.8 | 0.0, 0.2, 23.2 | 0.0, 0.2 | 0.0, 17.4, 23.2 |
| aprilia_rsv4-rf-2018 | 3.2 | 9.2 | 0.0, 0.0, 23.4 | 0.0, 0.2 | 0.5, 19.7, 23.4 |
| bmw_m-1000-rr-rm-2021 | 2.0 | 9.7 | 0.0, 0.0, 26.1 | 0.0, 0.0 | 2.0, 25.3, 26.2 |
| ducati_1098r-em-2008 | 2.1 | 9.6 | 0.0, 0.0, 25.8 | 0.0, 0.0 | 1.9, 25.2, 25.9 |
| ducati_desertx-rally-2024 | 4.4 | 8.7 | 0.0, 0.0, 19.9 | 0.0, 0.0 | 0.0, 0.0, 19.9 |
| ducati_superleggera-v4-2021 | 2.2 | 9.5 | 0.0, 0.0, 25.4 | 0.0, 0.0 | 1.6, 23.6, 25.4 |
| honda_cbr-1000rr-fir-89e7cd | 2.7 | 9.3 | 0.0, 0.0, 23.7 | 0.0, 0.0 | 0.0, 16.9, 23.7 |
| honda_cbr600rr-rm-2005 | 2.4 | 9.7 | 0.0, 0.0, 25.9 | 0.0, 0.0 | 2.1, 25.7, 25.9 |
| honda_rvf750-rc45-1994 | 2.1 | 9.3 | 0.0, 0.0, 23.8 | 0.0, 0.0 | 0.0, 0.0, 23.8 |
| kawasaki_ninja-h2-2019 | 2.6 | 9.0 | 0.0, 0.0, 23.3 | 0.0, 0.0 | 0.0, 0.0, 23.3 |
| kawasaki_ninja-zx-6r-d950a7 | 2.9 | 9.3 | 0.0, 0.0, 23.8 | 0.0, 0.0 | 0.3, 19.4, 23.8 |
| kawasaki_zx-r-750-em-1989 | 2.1 | 9.5 | 0.0, 0.0, 25.5 | 0.0, 0.0 | 1.9, 24.5, 25.6 |
| ktm_rc-390-2023 | 3.8 | 9.6 | 0.0, 0.2, 24.0 | 0.0, 18.2 | 1.6, 23.3, 24.0 |
| mv-agusta_f4-1000s-2006 | 2.5 | 9.2 | 0.0, 0.0, 23.8 | 0.0, 0.0 | 0.0, 16.5, 23.8 |
| paton_s1-r-lightweig-88d18e | 2.8 | 9.8 | 0.0, 15.8, 25.0 | 0.2, 21.7 | 1.5, 23.0, 25.0 |
| suzuki_gsx-r-600-rm-2017 | 2.1 | 9.8 | 0.0, 0.0, 26.7 | 0.0, 0.3 | 2.2, 26.9, 26.7 |
| suzuki_hayabusa-rm-2021 | 1.8 | 9.1 | 0.0, 0.0, 24.2 | 0.0, 0.0 | 0.0, 0.3, 24.2 |
| triumph_daytona-765-d846c3 | 2.2 | 9.3 | 0.0, 0.0, 25.0 | 0.0, 0.0 | 1.5, 22.8, 25.0 |
| yamaha_fzr-750-r-ow-439b32 | 2.3 | 9.6 | 0.0, 0.0, 25.1 | 0.0, 0.0 | 1.5, 22.7, 25.1 |
| yamaha_yzf-r1-2008 | 2.7 | 9.2 | 0.0, 0.0, 23.9 | 0.0, 0.0 | 0.0, 19.5, 23.9 |
| yamaha_yzf-r6-2019 | 1.9 | 9.2 | 0.0, 0.0, 25.6 | 0.0, 0.0 | 1.6, 24.2, 25.6 |
| mean of 21 | | | 0.0, 0.8, 24.4 | 0.0, 1.9 | 1.0, 17.9, 24.4 |

The columns show three things. **(a)** At the default level 1 no bike of the table lifts the rear by itself (0.0 degrees; the KTM, not in
the table, 0.9), whatever the lever up to 0.95, and at 0.5 only the two lightest bikes lift in the asked band. **(b)** At level 0 the ABS
no longer limits and the relief holds the rear at 0 to 2.2 degrees (1.0 on average, the lift threshold of these bikes is about 10 to
10.5 m/s^2): this is the automatic stoppie the reference allows, and it is small by design. **(c)** At level 0 with the lever in the
asked band (0.85) the relief starts at 30 degrees, so the nose goes on dropping: 17 of the 21 bikes are at 16.5 to 27 degrees after
2.5 s, and with the lever at 1.0 the help takes it to its 45 degrees (all 21 are at 20 to 27 degrees after 2.5 s on the way up).
**That free stoppie of 15 to 25 degrees in the asked band is the nearest thing in the data to the report's "about 20 degrees"**; no
bike does it by itself.

**The lever back** takes the weight to -1, which the auto-rider has already put there under the brake (the rest curve), so the
columns for the lever at 0 and at -1 are equal for all 21 bikes. What does end a stoppie is the lever leaving the asked band: from a
stoppie of 11 degrees on the KTM at lever 1.0, a lever at 0 or -1 hands the weight back and the relief is the unasked one (full at
4 degrees): the nose is up again within 0.3 s (`pulling_the_lever_back_ends_the_stoppie_within_half_a_second`).

**Not changed, and why.** The brake's force, the ABS's thresholds and ramp, the tyre's grip and the relief are data or recovered
code ([ride6-tricks.md](ride6-tricks.md) section 4, [ride6-tyre.md](ride6-tyre.md)); none of them is a guess that could be moved to give
a 20 degree stoppie, and a stoppie of that size without the lever in the asked band would break the relief rule (the relief is full at
4 degrees on the Motard bike and at 8 on the others). The front fork's travel and the load transfer follow the suspension and the
mass distribution of the other specs; the numbers above are what they give.

## 6. Choices and what stays open

| | | |
|---|---|---|
| The lever forward does not cancel the tuck | [C] | `forward_weight_tucks`, default true, in the manifest; the reference's rule is `false` |
| The pad's lever reads 1 at 0.9 of the travel past the dead zone | [C] | `weight_axis::REACH` |
| No key or button for a held tuck input | [I] | the reference has an action `TuckIn` whose source device is not in the physics module; `Bike::set_tuck` takes it |
| The input layer's own shaping of the lever (`WeightLongitudinalConfiguration`) | open | blueprint data of the player input component; the lever here is linear past the dead zone |
| The importer does not write `[bike.abs_level]` | open | the ABS level's curves (front 0.1, 0.2, 0.45, 1, 1; relief 1, 0.75, 0.5, 0, 0) are in the specs and in `abs_level.rs` but not in the 288 manifests, so the level is the plain factor there (level 0: no front ABS at all instead of a tenth); `bike_weight_brake` at `RIDE6_ABS=0` shows the plain factor |
| The stoppie's overshoot at the full lever at 30 m/s | open | the help holds 45 degrees, but the pitch passes 41 degrees (the balance angle of the KTM) and the rider falls at 60; the reference's relief for the asked stoppie (30 to 90 degrees) is as implemented |
