# RIDE 6 burnout: the rear wheel spinning on the spot

How RIDE 6 lets a bike burn out its rear tyre while the front brake holds the bike, as far as the user's own game data and binary
say, why the bikes of this project could not do it, and how the model does it now. Sources: the input handler assets of the install
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
speed window starts at 0 and closes the cut to nothing at a standstill; and it is off in a burnout, section 3), the wheelie help.

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
| Traction control | the cut is 0 (`bTCSLowSpeedModulationForBurnout` false); with it true the cut follows `TCSSpeedThresholds` (0 to 5 km/h) instead. The flag's default was not found, but below 5 km/h the speed ramp is nearly the same | `0x14512bd20`, `0x145110450`, `0x14513f3b0` |
| Automatic gearbox | no over-limit change up ([ride6-gearbox-auto](ride6-gearbox-auto.md)) | `0x1450d7000` |
| Lean controller | lean limit `BurnoutLeanLimit` (18 degrees, supermoto 10), lean help factor 1, balance 0, stability help 0 | `0x1450c3ae0` and the help functions ([ride6-chassis-round6](ride6-chassis-round6.md)) |
| Burnout help force | a yaw-rate controller at the rear contact, see section 6 | `0x1450bfa00` |

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
| `BurnoutHelpReduction`, `...Duration`, `...RaiseRate` | absent | 1, 0.1 s, 10 |
| `BurnoutMinSpeed`, `bEnableBurnoutInNeutral` | absent | 5 km/h, false |

## 5. The model [M where marked]

`libs/blackbox-vehicle/src/bike/burnout.rs` holds the flag and the numbers (`BurnoutSpec`: `grip`, `slip_correction`, `speed_limit`);
the manifest has them in `[bike.burnout]` (`docs/modding/bikes.md`), the importer writes the table from the handler
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
- **Traction control** (`mod.rs`): the throttle cut of the control is 0; the rider help's engine cut stays.
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

## 6. Not modelled

- **The burnout help force** (`EnableBurnoutHelpForce`, on for the 436 road handlers, off for the supermotos). It is a yaw-rate
  controller: a force at the rear wheel's contact, along the wheel's axle, that drives the bike's yaw rate to `BurnoutDesiredAngularSpeed`
  (45 degrees per second) times the steering input filtered over `BurnoutSteerFilterTime`, scaled by the throttle and the clutch, by
  `BurnoutHelpForceFactor` and by the engine's rpm share (`BurnoutHelpForceRPMPercentageRange`). So a held stick swings the rear round
  the held front wheel (a donut), and a centred stick damps any yaw. The decompiled body is a long vector expression and was not turned into
  an algorithm. In the model the stick alone swings the rear by the rear's 5 % of lateral grip: with the stick over, 5 of 288 bikes move
  more than a metre (1.9 m at most) and none falls.
- The burnout lean limit and the help terms of the lean controller (section 3): the lean files are another round's. The bike stands
  upright on the front tyre, the rider keeps it up, and the sweep shows no fall.
- The low-speed brake (the model holds a stopped bike by tyre friction and the lever), and engine stall (the model's engine has none).

## 7. Tests and the sweep

Unit tests (`bike/burnout_tests.rs`, `powertrain_tests.rs`, `bike_burnout.rs` of the manifest, `physics/burnout.rs` of the importer, and
`sim/bike_tests.rs` of the host): the flag's terms; the pedal and the throttle spin the example bike's rear wheel past a slip ratio of 1
within a second, it stays within a metre, upright, unfallen; the front lever alone does the same, the handbrake or no brake does not;
the rear tyre's force is under a tenth of its peak (and five times more with a grip of 1); the release launches it past 15 m/s in 4 s
without a pitch of 1 rad; the clutch is engaged and the over-limit change up is off; an engine far over its limit stays cut; the host
shows skid and smoke on the rear wheel only and none after the launch.

The sweep (`libs/blackbox-modding/src/content/bike_ride_burnout.rs`, `bike_burnout`; the 288 bikes of the install; the throttle and the
hold for 3 s, then the brake let go for 4 s; pass = rear slip ratio over 1 within 1 s, within 1 m of the start, no fall, no pitch past
1.2 rad, over 5 m/s at the end):

| Hold | Pass | Latest spin | Farthest | Worst pitch |
|---|---|---|---|---|
| brake pedal (both levers) and full throttle | 288 of 288 | 0.62 s | 0.17 m | 0.59 rad |
| front lever alone and full throttle | 288 of 288 | 0.55 s | 0.17 m | 0.58 rad |
| pedal at 0.3 and full throttle | 288 of 288 | 0.62 s | 0.23 m | 0.57 rad |
| pedal, half throttle (spin not required) | 288 of 288 | 1.37 s | 0.18 m | 0.51 rad |
| pedal and the stick over | 283 of 288 | 0.47 s | 1.86 m | 0.74 rad |

Before: the pedal never spun the wheel (slip ratio 0.1 to 0.4 on the three bikes tried). The five that move over a metre with the stick over
are the BMW R 1250 R, Honda X-ADV, Italjet Dragster 700, Kawasaki KZ1000R and KTM 890 Duke R: they swing round the front wheel
(section 6). Run it with `RIDE6_CARS=<cars folder> cargo test -p blackbox-modding --features host --lib bike_burnout -- --ignored --nocapture`
(`RIDE6_VERBOSE=1` for one line a bike, `bike_burnout_trace` with `RIDE6_FILTER=<bike>` for the time series round the release).

## 8. Open

- The burnout help force (section 6) and the burnout lean limit.
- The default of `bTCSLowSpeedModulationForBurnout` (not found; the effect is small under 5 km/h).
- Whether the reference's rear brake is joined to the front lever below `BurnoutMinSpeed` for a rider with one brake axis; the model
  releases it, a rider with two levers needs no such rule.
- The tyre smoke's colour and density for a bike are the car's.

## 8. Checked in the game

Offscreen on the final build (`--muted`, close camera): the KTM 450 held at a standstill with `throttle=1,brake=1` spins the rear wheel in a cloud of tyre smoke, stays upright and shows 0 km/h at a high rpm. The smoke is the cars' own rear-tyre effect.
