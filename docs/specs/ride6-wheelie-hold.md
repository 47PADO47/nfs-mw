# RIDE 6 bikes: a lighter bike, and a wheelie held with the throttle (clean-room spec)

The report of the player: "make the bikes a little lighter so that it is easier to wheelie, and to hold a wheelie, spamming the gas or not". Continues
[ride6-tricks.md](ride6-tricks.md) (the wheelie control and the rider help) and [ride6-feel-round11.md](ride6-feel-round11.md) (the lean help of a bike on
its rear wheel). Tags: **[M]** measured on the model (the tests below and the sweep `bike_wheelie_blip` over the 288 imported bikes, run this round),
**[D]** data, **[R]** read in this repository's code, **[C]** a choice of this project.

## 1. What a wheelie is made of here [R, D]

The control is the weight lever back (V, the pad's left stick down) with the throttle (`ride6-tricks.md`, sections 1 to 3): the throttle is the strength
of the request, so the assist asks for `throttle x MaxAngle` (35 degrees for the KTM) and a throttle that is shut asks for level. Its torque is a
fraction of the weight's; the thrust lifts. With the lever **released** the rider help cuts the throttle from 4 degrees up (`throttle_pitch`), the auto-rider
puts the weight forward under the throttle, and the assist is off: the front wheel does not lift. Measured on the 288 bikes, a full-throttle launch from a
standstill with the lever released lifts **none** past 10 degrees in 4 s (the KTM: a front load of 120 N of 1800 at its best and 1.4 degrees of pitch; with the
bike's mass cut to 0.4 of the data's it is 3.5 degrees), so the lever stays the way to start a wheelie [M]. That is the reference's design and was not changed.

## 2. A bike a sixth lighter [C]

`choices::BIKE_MASS_SCALE` = 0.85, applied once in `Bike::new` to the bike's own mass (`chassis.mass`, wheels included); the rider's 70 kg is not scaled and
the 288 manifests are not regenerated. The inertia follows (it is the radius of gyration squared times the total mass), as do the rest loads, the combined
centre of gravity (the rider's share goes up) and the pitch term of the assist. 1.0 gives the data's masses back. For the KTM 450 the total goes from 183 to 166 kg.
It helps by itself a little (launch of the 288 with the lever back, lift under 1.5 s: 262 bikes before, 273 after), but a lighter bike alone **loses** the held
wheelie to the bang-bang throttle (253 of 288 hold it before, 218 after: its nose drops faster when the throttle is shut and the assist's torque, scaled by
the mass, is smaller), which is why section 3 comes with it.

`a_bike_with_no_wheelie_control_has_nothing_to_hold_it` (tests_tricks.rs) was inverted by the change: with `wheelie = None` (no assist, no cut) the lighter bike
goes past the 40 degrees the control holds it at; every one of the 288 manifests has the control.

## 3. A held wheelie is lowered gently [C]

The nose drops fast when the throttle is shut: the torque of the assist is `Mass x InertiaTensorScale.pitch x Acceleration` = 45 kg m^2 x up to 10 rad/s^2 for the
KTM, and a wheelie turns about the rear contact with an inertia of 170 kg m^2 and a weight torque of 1.2 kN m (7 rad/s^2): a throttle shut at 30 to 37 degrees
took the front down at **110 to 125 degrees a second** and it landed at 20 or more (before this round, measured on the example and on a superbike) [M].

`pitch_assist::lowering_torque`, added to the wheelie's torque while the assist acts (the lever back, the rear wheel on the road, the front brake free, above the
minimum speed; not to a stoppie): the nose may drop at `lowering_rate(pitch)` = 1.2 per second times its height, held between 0.12 rad/s (7 degrees a second at
touchdown) and 0.7 rad/s (40 degrees a second from 35 degrees); the torque holds back the excess over the rate in 0.02 s, as an angular acceleration of the
whole body about the rear contact (`Bike::pivot_inertia`: its own inertia plus its mass times the squared distance of its centre of gravity from the contact in
the plane of the pitch) held to 25 rad/s^2. Constants `choices::WHEELIE_LOWER_*`. A nose that rises or is level gets nothing, so a blip of the throttle raises it as before.
A response of 0.05 s instead of 0.02 left a soft limit (the excess settles at the weight's acceleration over the gain: 0.6 rad/s) and a landing at 22 degrees a second.

Result [M]: from a held 35 degrees at 20 m/s with the throttle shut, the nose goes down at most 38 degrees a second on the supermoto and 45 on the superbike (110
to 125 before), takes 1.0 to 1.1 s to touch (0.4 before) and touches at 13 to 14 degrees a second (20 or more before).

## 4. Held by blips [M]

`bike/wheelie_hold_tests.rs`: the supermoto (the example) and a superbike (195 kg, three times the torque, 5.0 final ratio), the lever back, 20 m/s (also 10 and 30), a
0.6 s burst and then a bang-bang throttle, on below 38 degrees and off above, 9 s:

| | pitch over the last 5 s | front wheel | fell |
|---|---|---|---|
| supermoto 10, 20, 30 m/s | 30.1 to 38.1, 30.0 to 37.4, 32.5 to 36.4 degrees | up all the time | no |
| superbike 10, 20, 30 m/s | 35.9 to 38.6, 34.8 to 38.4, 35.9 to 38.3 degrees | up all the time | no |

(the test asks for 28 to 46 degrees: an upshift cuts the thrust and costs the supermoto up to 8 degrees for a moment, 29.4 degrees at a target of 34 at 30 m/s.)
A launch from a standstill in first gear at full throttle with the lever back: 10 degrees at 1.07 s on the supermoto (the front wheel up, still first gear), 0.7 s on the
superbike. A blip of 0.6 s of full throttle from the ground raises the nose to 12 degrees (supermoto) and 16 (superbike), and a second one does the same after it was let down. The throttle held at
full with the lever back from 0, 10 and 25 m/s for 10 s never takes the pitch within 0.1 rad of the fall limit (no loop-out), as before.

Over the 288 imported bikes (`bike_wheelie_blip`, 20 m/s, lever back, target 37, "held" = the pitch stays between 25 and 50 degrees for the last 5 s): the data's masses
without the lowering 253 hold; a sixth lighter without it 218; a sixth lighter with it **286**; none falls; the two that do not hold are the Honda X-ADV (a scooter that does not lift
at 20 m/s) and the Suzuki Turbo Martek (it dips to 21 degrees). Lift within 1.5 s from a standstill with the lever back: 262 before, 273 after.

## 5. Not changed, open

- The lever released: no launch lifts the front (section 1). **Done since, as a game-specific switchable assist: [ride6-wheelie-throttle.md](ride6-wheelie-throttle.md).**
  Original note: If the players want the throttle alone to start a wheelie, the rider help's cut (`throttle_pitch`) and the
  auto-rider's forward weight under the throttle are the two things to open for a launch; neither was touched.
- The second-gear and upshift dips of a held wheelie are the gearbox's (the clutch cut).
- The lowering is for the assist's wheelie only; a wheelie without the lever (lifted by a kerb, a jump) lands as before.
- The scooters and the lightest bikes do not lift at 20 m/s with the data's torque; the mass scale is the only knob the report asked for.

## 6. To check

```
cargo test -p blackbox-vehicle wheelie_hold
RIDE6_CARS=<folder of manifests> [RIDE6_FILTER=ktm450] [RIDE6_START=20] [RIDE6_TARGET=37] cargo test -p blackbox-modding --features host --lib bike_wheelie_blip -- --ignored --nocapture
nfsmw view-world --drive ducati_panigale-v4-2018 --muted --camera-mode chase --drive-script "3:throttle=1,weight=-1;6:throttle=1,weight=-1;1.5:throttle=0,weight=-1;1:throttle=1,weight=-1;2:throttle=0,weight=-1" --screenshot out/wheelie.png --screenshot-delay 3 --screenshot-interval 0.5 --screenshot-count 20
```
