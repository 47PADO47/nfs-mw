# RIDE 6 burnout: the rear wheel spinning on the spot

How RIDE 6 lets a bike burn out its rear tyre while the front brake holds the bike, and swing round it with the stick over (a donut), as far
as the user's own game data and binary say, why the bikes of this project could not do it, and how the model does it now. Sources: the input handler assets of the install
(eight classes, 445 handler objects over 319 vehicles, read with the physics dump), the native input handler class (build
`5.6.1-1697004`, decompiled in a private headless copy), and the model's own sweeps. Evidence tags as in [README](../README.md):
**[B]** read from the binary, **[D]** read from data, **[I]** inferred, **[M]** a modelling choice of this project. Provenance:
[ride6-burnout](../provenance/ride6-burnout.md). Earlier rounds: [ride6-feel-round10](ride6-feel-round10.md), [ride6-chassis-round6](ride6-chassis-round6.md).

## 1. Why the player could not burn out

The user's report: "I can't do burnouts". Three causes, in the order they bite:

1. **The brake key pulls both brakes.** The game's input layer has one brake axis; `command()` of the bike (`crates/nfsmw/.../sim/bike.rs`)
   sends it to the front lever and the rear lever alike, and the handbrake to the rear one alone. With throttle and brake held at a
   standstill the rear wheel was braked as hard as the front one, so it never spun (the three bikes tried: R1 2014, Panigale V4, Hypermotard
   698: rear slip ratio 0.1 to 0.4 after 3 s). The reference has two levers, and its burnout needs the **front** one alone (section 2).
2. **The model had no burnout.** Even with the rear lever free, the rear tyre kept all its grip and the front brake could not hold a
   bike that is pushed by the full grip of the rear: the bikes crept 0.5 m in 3 s and the wheel spun only because the launch clutch
   slipped. What lets a real bike stay while its rear spins is the reference's burnout state: the rear tyre has a twentieth of its grip.
3. **A wheel that spins free ran away.** With the clutch engaged (section 3) the engine follows the free wheel; the rev limiter alternates
   the cut and the torque curve is held flat past its end, so the engine climbed to 31000 rpm on a 15500 rpm bike, and the bike flew off
   at the release (47 of 288 bikes flipped). Section 5 has the fix.

Not the cause, though suspected: the reverse (the walk back starts only with *no* throttle, `reverse.rs`), the traction control (its
speed window starts at 0 and closes the cut to nothing at a standstill, section 3), the wheelie help.

## 2. The burnout flag [B]

The input handler (`UBikeInputHandler`) computes the flags `IsAccelerating`, `IsBraking`, `IsUsingHandbrake`, `IsPressingClutch`,
`IsInReverse`, `IsAtLowSpeed` and **`IsDoingBurnout`** from the vehicle's inputs once per step (`0x1450d7fb0`):

```
accelerating = throttle > 0.1 and the engine runs
braking      = front brake lever > 0.1          # "Brake" input
handbrake    = rear brake lever  > 0.1          # "Handbrake" input
clutch       = clutch lever      > 0.1
speed        = |forward speed| in km/h          # VehicleForwardSpeed [cm/s] x 0.036
burnout      = speed < BurnoutMinSpeed and accelerating and braking and not handbrake and not clutch
               and (gear > 0, or gear >= 0 when bEnableBurnoutInNeutral)
```

`BurnoutMinSpeed` is a limit despite its name (a burnout needs a speed *under* it); the constructor sets 5 (km/h) and no handler asset
overrides it [B: `0x1450bd6d0`, store of `0x40a00000` at `+0x258`]. `bEnableBurnoutInNeutral` is false. There is no hysteresis and no timer.

## 3. What the flag does [B]

| Consumer | Effect in a burnout | Function |
|---|---|---|
| Wheel set-up of the handler | the rear wheel's two runtime grip scales (`+0x460` longitudinal, `+0x464` lateral; the setters `0x145168a50`, `0x145168a30` are one `movss` each) both get `BurnoutGripMultiplier`, and its `SlipCorrectionMultiplier` (`+0x308`) gets `BurnoutSlipCorrectionMultiplier`; out of a burnout they are 1, 1 | `0x1450d7ed0` |
| Starting sequence | the launch clutch (the feathering controller, the plain launch ramp) is not worked: the clutch stays where the rider's lever leaves it, engaged | `0x1450d7cd0` |
| Traction control | `bTCSLowSpeedModulationForBurnout` false: the cut is 0; true: the cut is the usual one shaped by `TCSSpeedThresholds` (0 to 5 km/h in the data). **The constructor sets it true** (section 4) and no asset clears it, so the cut stays | `0x14512bd20`, `0x145110450`, `0x14513f3b0`, `0x1450fe630` |
| Automatic gearbox | no over-limit change up ([ride6-gearbox-auto](ride6-gearbox-auto.md)) | `0x1450d7000` |
| Lean controller | the lean limit **is** `BurnoutLeanLimit` (18 degrees, supermoto 10) and nothing else of the limit applies; lean help factor 1, balance 0, stability help 0 | `0x14510b180`, `0x1450c3ae0` and the help functions ([ride6-chassis-round6](ride6-chassis-round6.md)) |
| Burnout help force | a yaw-rate controller at the rear contact, section 6 | `0x1450d5050`, `0x1450bfa00` |
| Collision | a collision sets the help force's cut (`BurnoutHelpReduction`) for `BurnoutHelpReductionDuration`, then gives it back at `BurnoutHelpRaiseRate` | `0x1450dd9c0`, `0x1450e0480` |

The low-speed brake (`LowSpeedBrakeTuning`, which holds a stopped bike with both brakes) is **off** while the throttle is pressed
(`0x1450d76b0` needs `IsAccelerating` false), so it does not fight a burnout. The reverse needs no throttle.

## 4. The data [D]

Over the 319 vehicles of the dump (445 handler objects, counted per object):

| Key | Values | Constructor default |
|---|---|---|
| `BurnoutGripMultiplier` | 0.05 on 436 (all road, naked, adventure, scooter, bagger classes), 0.15 on 9 (the supermoto handler: Hypermotard 698, CRF450R-SMT x2, FS450, 450 SMR, 690 SMC-R, TM SMX x2) | 1 |
| `BurnoutSlipCorrectionMultiplier` | 0.9 on the 9 supermotos, absent elsewhere | **0** (`0x62c` is stored as a quad word, 1.0f then 0) |
| `BurnoutLeanLimit` | 18 on 436, 10 on the 9 supermotos | |
| `EnableBurnoutHelpForce` | false on the 9 supermotos, absent elsewhere | true |
| `BurnoutHelpForceFactor`, `BurnoutDesiredAngularSpeed`, `BurnoutSteerFilterTime` | absent | 1, 45 (degrees per second), 1 s |
| `BurnoutHelpForceRPMPercentageRange` | absent | 0.75 to 0.95 (doubles at `+0x610`, `+0x618`: `0x3fe8...`, `0x3fee666660000000`) |
| `BurnoutHelpReduction`, `...Duration`, `...RaiseRate` | absent | 1, 0.1 s, 10 |
| `bTCSLowSpeedModulationForBurnout` (traction control element) | absent from all 319 vehicles | **true** (`0x1450fe630`: the byte at `+0x48` is stored 1; `TCSSpeedThresholds` there is 15 to 50 and the assets set it) |
| `BurnoutMinSpeed`, `bEnableBurnoutInNeutral` | absent | 5 km/h, false |

## 5. The model [M where marked]

`libs/blackbox-vehicle/src/bike/burnout.rs` holds the flag and the numbers (`BurnoutSpec`: `grip`, `slip_correction`, `speed_limit`,
`lean_limit`, `tcs_modulation` and the help force's `HelpSpec`); the manifest has them in `[bike.burnout]` and `[bike.burnout.help]`
(`docs/modding/bikes.md`), the importer writes the tables from the handler and the traction control element
(`mods/ride6-bikes/src/physics/burnout.rs`) and a bike without the table gets the road bikes' handler.

- **The flag** (section 2) from the rider's held throttle and levers and the speed along the bike, every sub-step. The clutch is
  the automatic gearbox's and never the rider's, and the gear is never neutral or reverse, so those terms are always true.
- **The joined pedal [M].** The host has one brake axis, so a pedal pull arrives as equal front and rear levers. The model reads a rear
  lever that is within 0.02 of the front one as the pedal's own (not the "handbrake"), and in a burnout the rear lever is released:
  *brake plus throttle at a standstill is the front brake alone*. A rear lever above the front one (the handbrake held beyond the
  pedal) is the hand's and no burnout. With separate levers (a host with a pad's two triggers) the reference's rule holds as it is:
  any pressed rear lever stops the burnout. The cost: a rider cannot hold both brakes equally with the throttle open on the spot.
- **Rear grip** (`wheel_step.rs`, `tyre.rs`): the two grip scales of the surface are multiplied by `grip`, and the combined-slip
  normalisation takes `slip_correction` instead of the fixed 1 (the wheel's `SlipCorrectionMultiplier`).
- **Clutch** (`powertrain.rs`): `launch()` returns 0 and ends the start in a burnout; the engine is joined to the wheel. The over-limit
  change up does not come.
- **Traction control** (`mod.rs`, `burnout_traction_cut`): the control's cut stands in a burnout, shaped by its speed window (0 to 5 km/h,
  so it is weak at a standstill and 0.7 at 1 m/s); `[bike.burnout] tcs_low_speed_modulation = false` would make it 0. Round 11 had it 0 by
  assumption; with the constructor's true the sweep's latest rear spin moved from 0.62 s to 0.73 s and nothing else changed.
- **Lean** (`lean.rs`, `handling.rs`): the lean limit is `[bike.burnout] lean_limit_rad` (18 degrees, 10 for the supermotos) while the flag stands
  and it replaces every other term (case 1 of the effective limit, [ride6-lean-controller](ride6-lean-controller.md)); 0 leaves the limit
  alone. The lean help factor is 1, its balance 0 and the stability help 0 in a burnout. A bike with the stick over therefore leans 18 degrees
  into its donut (the KTM's standing limit is 1 degree).
- **Help force** (`burnout.rs`, section 6): the stick-over donut.
- **The rear brake** is released while the flag stands (above); the front brake is the rider's.
- **Over-rev [M]** (`choices::OVERREV_SHARE`, 1.1): past 1.1 times the rev limit the fuel stays cut instead of alternating. The reference
  alternates the cut at the limiter [B, `ride6-engine-drivetrain.md`], which averages a positive torque where the torque curve is held
  flat past its end; with the clutch engaged and a rear wheel free to spin, the engine of the 15500 rpm R6 climbed to 31000 rpm in 3 s
  and the release threw the bike (a flip on 47 of 288 bikes). A real engine is dragged back; at 1.1 the alternation near the limit is
  unchanged (tested at 1.05). After the fix the burnout holds the engine at about 16200 rpm.
- **Smoke and marks** (`sim/bike.rs`): the wheel readout took the slide along the wheel as the slip ratio times the speed along the
  bike (at least 1 m/s), and the ratio is scaled down below 1 m/s, so a rear wheel spinning on the spot read 0.9 m/s of slide and made
  no smoke. The bike's `WheelState` now carries the tyre's own `slip_speed` (rolling speed of the wheel less the speed of the patch), and
  the host takes it: a burnout reads 30 m/s or more, `skid` and `smoke` 1 on the rear wheel and 0 on the front, the car's own smoke
  and skid-mark effects (`effects/smoke.rs`, `marks.rs`) take it from there.

### Controls

Hold the **brake** and the **throttle** together with the bike under 5 km/h (keyboard: both keys; pad: both triggers); let the brake go
and the bike launches with the rear still spinning. The controls screen says so under "Brake"; `docs/modding/bikes.md` has the table.

## 6. The burnout help force: the donut [B, with [I] and [M] marked]

`EnableBurnoutHelpForce` (true by the constructor, false on the 9 supermoto handler objects) switches on a yaw-rate controller that runs
every step in a burnout (`0x1450d5050` calls `0x1450bfa00`; out of a burnout it zeroes its state). Read from the decompiled body, with the
vector arithmetic turned into the operations it performs (a quaternion rotation of vectors into the body frame and back):

```
rear wheel in contact (wheel +0x28), up = the road's normal                         # 0x1450cd860: front normal + blended rear normal, up if flat
r      = rear contact (wheel centre less the chassis' up x radius, 0x145157eb0) - vehicle origin
r_perp = r - up (up . r);   L = |r_perp|;   act only if L >= 0.1 cm                # 0x148fbaccc
f      = filtered stick:  gap = f - stick;  m = sqrt|gap| - dt x BurnoutSteerFilterTime;  f = (m > 0) ? stick + sign(gap) m^2 : stick
want   = BurnoutDesiredAngularSpeed (rad/s) x f           # stored; the yaw rate now is up . angular velocity
tau    = (up . I_world up) (want - yaw rate) / max(dt, 0.001)        # I_world = R diag(inertia) R^T; the torque that closes the gap in one step
side   = unit( right - up (up . right) )                  # the chassis' local Y, `0x148f950b0` = (0, 1), without its part along up
need   = (tau / L) x Actions.Throttle x (Actions.Clutch - 1) x BurnoutHelpForceFactor
share  = clamp01( (rpm - 0.75 M) / (0.95 M - 0.75 M) ),  M = EngineData.MaxRPM x limiter MaxRPMScale    # 0x1450ba6a0; 1 when the range is unset
force  = side x (need - ContactForce_rear . side) x share x (1 - CurrentHelpReduction)
apply `force` at r_perp (the road-plane part of r, so it adds a yaw torque only)         # AddForceAtLocation, 0x1450ff5a0
```

`Actions.Throttle` is the throttle after the traction control and the rider help, `Actions.Clutch` the lever (0 released, so the factor is
-1: the force is minus the torque over the arm). The steer is `OriginalActions.Steer`, the rider's own stick before the lean controller
replaces it. The force makes the *total* side force on the rear contact the controller's (the tyre's own is subtracted), so the rear swings
round the front wheel, which the front brake holds, at the data's yaw rate. There is **no limit** on the force: it is a one-step deadbeat
controller, scaled by the throttle, the rpm share and the collision's cut only. `CurrentHelpReduction` is 0 until a collision (`0x1450dd9c0`)
sets it to `BurnoutHelpReduction` (1) for `BurnoutHelpReductionDuration` (0.1 s); it then falls at `BurnoutHelpRaiseRate` (10 per second,
`0x1450e0480`). The same variable is the `(1 - CurrentHelpReduction)` of the lean help formula ([ride6-chassis-round6](ride6-chassis-round6.md),
section 2), which the model does not apply (its collision help is the addition `A` alone).

**In the model** (`burnout.rs`: `filter_stick`, `rpm_share`, `Bike::apply_burnout_help`; `[bike.burnout.help]` of the manifest, the importer
writes every value from the handler with the constructor's where the asset is silent):

- The steps above, per sub-step (1/480 s at most), after the tyres have done theirs. The stick filter, the yaw gap, the arm and the rpm
  share are as read. The throttle is the one the engine got (`throttle x (1 - traction cut)`); the clutch term is 1 (the lever is never held).
- **[M] The arm and the point are taken from the centre of gravity**, about which the model's forces act, where the reference takes the vehicle's
  origin: with the origin the torque of the force would not be the one the controller asked for. **[M] `up`** is the sum of the two contacts' normals
  (as the lean limit's bank uses), not the reference's blend by `TerrainNormalBlendFactor` (not in the data) and its minimum-normal test.
  **[M]** the engine's `MaxRPMScale` is 1. **[I]** the vehicle's `+0x1d20` is the (diagonal) inertia, the body-frame axes are those of the
  rigid body, and the angular velocity read at `+0x1c88` is the body's.
- **The reference's own position quirk is not copied.** `0x1450ff5a0` subtracts the vehicle's position from the point it is given, and
  `0x1450bfa00` hands it a point that is already relative to that position; whether the physics space is centred on the vehicle, so that the
  two agree, was not found. The model applies the force at the intended point.
- Out of a burnout, with the rear wheel in the air, with `enabled` false (the supermotos), or with the rider off the bike, nothing is applied
  and the stick filter is back at rest.
- `Bike::burnout_yaw_target()` reads the yaw rate asked for now (rad/s, positive to the right); a host or a test can compare it with the
  body's. `Bike::notify_collision` also sets the cut.

The supermoto handler has no help force: its bikes swing round the steered front wheel by the rear tyre's own grip (0.15 and the slip
correction 0.9) and the 10 degree lean limit, at 0.44 to 0.50 rad/s in the sweep (25 to 29 degrees per second), which is the model's own result.

### Not modelled

- The low-speed brake (the model holds a stopped bike by tyre friction and the lever), and engine stall (the model's engine has none).
- The animation blueprint's burnout pose, the tyre smoke's colour and density (the car's).

## 7. Tests and the sweep

Unit tests (`bike/burnout_tests.rs`, `powertrain_tests.rs`, `bike_burnout.rs` of the manifest, `physics/burnout.rs` of the importer, and
`sim/bike_tests.rs` of the host): the flag's terms; the pedal and the throttle spin the example bike's rear wheel past a slip ratio of 1
within a second, it stays within a metre, upright, unfallen; the front lever alone does the same, the handbrake or no brake does not;
the rear tyre's force is under a tenth of its peak (and five times more with a grip of 1); the release launches it past 15 m/s in 4 s
without a pitch of 1 rad; the clutch is engaged and the over-limit change up is off; an engine far over its limit stays cut; the host
shows skid and smoke on the rear wheel only and none after the launch. Round 12 adds: the defaults (18 degrees, the TCS flag, the help
force's constructor values); the stick filter (1 - (1 - t)^2 for a full stick over a filter time of 1 s, never past the stick, symmetric,
a time of 0 never moves); the rpm share; the donut (the example bike with the pedal and the stick over: the mean yaw rate of the last second
within 10 % of 45 degrees per second, the filter arrived, the front contact within 0.3 m, the rear still spinning, upright, turned past
1.5 rad in 4 s; the same to the left and at half the stick; zero with a centred stick; not the data's without the help force; none and a
restarted filter after the brake is let go); a collision's cut (held 0.1 s, then 10 per second, and no yaw while it is held); the lean limit
(18 degrees, 10 for the supermoto's table, the usual one with none); and the traction control's cut (kept by default, 0 with the flag cleared).

The sweep (`libs/blackbox-modding/src/content/bike_ride_burnout.rs`, `bike_burnout`; the 288 bikes of the install; the hold for 3 s, then the
brake let go for 4 s; pass = rear slip ratio over 1 within 1 s, within 1 m of the start (**the front contact**, for a donut, which turns
the bike round it), no fall, no pitch past 1.2 rad, over 5 m/s at the end, and with the stick over the yaw rate of the last second within 15 % of the
data's for each bike whose help force asked for one):

| Hold | Pass | Latest spin | Farthest | Worst pitch |
|---|---|---|---|---|
| brake pedal (both levers) and full throttle | 288 of 288 | 0.73 s | 0.17 m | 0.57 rad |
| front lever alone and full throttle | 288 of 288 | 0.67 s | 0.17 m | 0.58 rad |
| pedal at 0.3 and full throttle | 288 of 288 | 0.73 s | 0.23 m | 0.56 rad |
| pedal, half throttle (spin not required) | 288 of 288 | 1.40 s | 0.18 m | 0.50 rad |
| pedal and the stick over (right) | 288 of 288 | 0.52 s | 0.34 m (front) | 0.58 rad |
| pedal and the stick over to the left | 288 of 288 | 0.52 s | 0.31 m (front) | 0.58 rad |
| pedal and the stick half over | 288 of 288 | 0.52 s | 0.18 m (front) | 0.61 rad |

Donut: 281 bikes ask for a yaw rate, **281 reach it** (the measured rate is 1.006 to 1.015 times the data's, at the stick's 1, -1 and 0.5);
the 7 with the help force off (Hypermotard 698, CRF450R-SMT x2, KTM 450 SMR, 690 SMC-R, TM SMX x2) swing at 0.44 to 0.50 rad/s on their own, front
contact within 0.28 m, none falls. Before round 12 the stick over moved 5 of the 288 bikes more than a metre (1.9 m at most, BMW R 1250 R, Honda
X-ADV, Italjet Dragster 700, Kawasaki KZ1000R, KTM 890 Duke R) and the yaw rate was whatever the rear's 5 % of grip gave. Run it with
`RIDE6_CARS=<cars folder> cargo test -p blackbox-modding --features host --lib bike_burnout -- --ignored --nocapture` (`RIDE6_VERBOSE=1` for one
line a bike with its yaw rate and the data's, `bike_burnout_trace` with `RIDE6_FILTER=<bike>` for the time series round the release).

## 8. Open

- Whether the reference's rear brake is joined to the front lever below `BurnoutMinSpeed` for a rider with one brake axis; the model
  releases it, a rider with two levers needs no such rule.
- The reference's position quirk in the help force (section 6), the `TerrainNormalBlendFactor` and the lean help's
  `(1 - CurrentHelpReduction)` after a collision.
- No statement about the help force was tested in the running game (it must not be launched): the yaw rate the model reaches is the data's
  by construction of the controller; whether the real bike keeps its front wheel as still as the model's is unknown.

## 9. Checked in the game

Offscreen on the final build (`--muted`, close camera): the KTM 450 held at a standstill with `throttle=1,brake=1` spins the rear wheel in a cloud of tyre smoke, stays upright and shows 0 km/h at a high rpm. The smoke is the cars' own rear-tyre effect.

## 10. Exhaust pops during a burnout (the game's addition) [C]

Playtest: "burnouts should produce some small popoff flames (the same one that happen when changing gear) every 3s." [C]

While a bike is in a burnout (the flag of section 2, `Bike::burnout()`, read by the host through `CarSim::bike_burnout`) the exhaust makes one **small pop** at fixed intervals, the same flame a gear change makes:
`crates/nfsmw/src/scenes/world/exhaust/burnout.rs` asks `ExhaustFlames::pop()` (the trigger of the console's `exhaust-flames pop`), which a bike draws as its shift flame, a quarter larger
(`trigger::BIKE_BACKFIRE_SCALE_OVER_SHIFT`, "a pop is the shift flame a quarter larger"; the flame's size, length and burst are the existing ones, nothing new is drawn).

| constant | value | why |
|---|---|---|
| `BURNOUT_POP_INTERVAL` | 3.0 s | the player's "every 3s" |
| `BURNOUT_FIRST_POP` | 1.0 s | the first pop comes a second into the burnout, so a burnout of a few seconds gets one (a choice; the rest follow every 3 s from it) |

**The pop is heard** (playtest: "during burnouts when the flame exits the exhaust i dont hear pops"): the pop used to be a flame only. A gear change's
flame comes with the shift's own recording, a lift-off's backfire flame with the car's `backfire`/`pops` recordings (`audio/car/own_sfx.rs`, the sound decides and the flame follows); the burnout's
pop is the other way round, the host lights it, so the host tells the sound. `ExhaustFlames::note_burnout` returns whether a pop is due (whatever the `exhaust_flames` setting: a sound
is not a flame), the drive notes it on its `SoundFeed` (`note_exhaust_pop`), the next tick carries it as `CarInput::exhaust_pop`, and `OwnSfx::exhaust_pop` plays **the bang a lift-off backfire plays**:
a random variant of the car's `backfire` recordings, else its `pops`, else its down-shift, at `logic::backfire_level(rpm_pct)` (0.5 at idle to 1.0 at the limiter, the lift-off backfire's own level;
a burnout is on the limiter) times the shared `SHOT_LEVEL`, in the same bank and the same group (`Sfx`) as every other one-shot. No flame is queued by the sound (the host's is already lit, so it does not flame twice);
a car with none of the three lists stays silent. **Not placed at the exhaust:** a mod car's one-shots are played flat (the car's sound is heard from the camera, as the shift and the lift-off backfire are), so the burnout
pop is not moved to the pipe either; positioning all of them is a separate change. Tests: `exhaust/burnout.rs` (the due flag whatever the setting), `logic/tests.rs` (the level), `drive/sound.rs` (the note is carried once and cleared).

Out of a burnout the clock stands at zero and the next burnout starts again with its first pop. The pop respects the `exhaust_flames` setting because `pop()` asks nothing while the flames are off (and the
flames of a car with no flame data stay quiet as before). Tests: `exhaust/burnout.rs` (the pops come at 1, 4, 7 and 10 s of an 11 s burnout; none shorter than the first; a new burnout starts over; the pop sets the
same `forced` flag as the console's and nothing while the setting is off).
