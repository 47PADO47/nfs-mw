# Provenance: the rider weight lever, round 9

Build `5.6.1-1697004` of the reference executable, read in a headless Ghidra project of the scratch folder (decompiler output stays
outside the repository). Addresses are image-relative to `0x140000000`. Spec: [ride6-weight-round9.md](../specs/ride6-weight-round9.md).

| Fact | Function | How it was read |
|---|---|---|
| The lever's side of the dead zone, the auto-rider flag, the manual aim (`W * ManualRiderForwardPosition` for `W > 0`, `-(W * ManualRiderBackwardPosition)` otherwise), the optional sum with the rider-position input | `URiderHelpVehicleElement` helper `0x145110af0` | compare of `\|W\|` with the field at `+0x6c` (`WeightLongitudinalInputDeadzone`); the products read at `+0x84` and `+0x88` |
| The aid position: the manual path approaches its aim at `ManualPositionEngageSpeed` (`+0x8c`), the auto path at `AutoRiderPositionEngageSpeed` (`+0x74`); the result is written to `RiderAidPosition` (`+0x9f4`) and to the actions (`+0x17f4`) | `0x1450ff610` | the tail of the function (square-root approach), the head (`0x145110af0` call) |
| `ManualRiderForwardPosition` 1, `ManualRiderBackwardPosition` -1, `AutoTuckIn` off, `TuckInPosition` 0.8, `TuckInWeightLongitudinalRange` (-1, 1) as doubles | constructor `0x50fda20` | constant stores at `+0x84`, `+0x88`, `+0x398`, `+0x3a0`/`+0x3a8` |
| The tuck action: held input above one half; otherwise `bAutoTuckIn` and the five bail-outs (speed, lean, **raw lever against the dead zone**, throttle, brake); `TuckInPosition` outside -1..1 with the default range returns without writing | tick `0x14512b870` | compares at `+0x3b4`, `+0x3b8` with `\|+0x1838\|`, `+0x6c` with `\|+0x17b4\|`, `+0x3bc`, `+0x3c0`; the range compare at `+0x3a0`/`+0x3a8` |
| The ABS release: `(1 - brake reduction) * (1 - ABS)` per wheel, each through a rate limiter (rise 10000, fall `ABSFallRate`); the ABS reads the wheel's normalised slip ratio at `+0x2e8` through the translator `NormalizedSlipTuning` | `0x1450e0240`, `0x1450cac70` | re-read in round 9 to confirm the equilibrium of section 5 |
| Data: `Base_RiderHelp` (dead zone 0.75, auto-tuck 90 km/h, stoppie weight control 3.6 to 4), `Motard_RiderHelp` (relief 0 to 4 degrees), `Motard_ABSVehicleElement` (front -0.3 to -0.5, rear -0.5 to -1, speed 10 to 25 km/h), the five input handlers' `Rider` offsets (`ForwardOffset` unset everywhere, `BackwardOffset` -22, -10 or 0 cm) | the physics assets under `PhysicsRide6/Physics/VehicleElements` | read with CUE4Parse |

Not read: the player input component's `EvaluateRiderWeightLongitudinal` and the helper configuration it uses (blueprint data).
