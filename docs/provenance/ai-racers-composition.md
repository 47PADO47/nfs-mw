# Racer composition and nitrous provenance

Sources for what a race opponent is built from and for its nitrous choice:
[specs/ai-racers.md](../specs/ai-racers.md) section 6 and
[specs/ai-driver-speed-skill.md](../specs/ai-driver-speed-skill.md) sections 2 and 7.1.

The inspected PC 1.3 executable has SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
Program identity was confirmed in Ghidra before every query. Live values come
from the unmodded install running the direct-boot quick race `16.1.1` (medium,
two laps, three opponents, no traffic, player on the game's own autopilot),
hidden and muted, with function hooks that only read memory (2026-11-18).

## Composition (2026-11-18)

Hooks on every behaviour's static constructor, two runs: each opponent is built
from `RBVehicle`, `PInput`, `EngineRacer`, `SuspensionSimple`, `DamageRacer`,
`DrawRaceCar`, `SoundRacer`, `AIVehicleRacecar`, `EffectsCar` and `ResetCar`; the
player's car from `RBVehicle`, `InputPlayer`, `EngineRacer`, `SuspensionRacer`,
`DamageRacer`, `DrawRaceCar`, `SoundRacer`, `AIVehicleHuman`, `EffectsPlayer` and
`ResetCar`. The intro scene replaces input, engine and suspension of every car
with `InputNIS`, `EngineSpline` and `SuspensionSpline`; the ordinary set is built
again when the scene hands over, about three seconds before the green light.
When the autopilot first takes the player's car, before the scene, a
`SuspensionSimple` is built for it; after the scene the player's car is back on
`SuspensionRacer` while the autopilot still drives it. This agrees with the
behaviour selector at `0x0067f0d0`, which gives `SuspensionSimple` to driver
classes 3, 4 and 6 only.

## Nitrous (2026-11-18)

The race action's think at `0x00428720` and its nitrous offer at `0x00407be0`
were read with their constants (`0x00890bf8` 10, `0x00890d68` 0.67,
`0x00890694` 0.33, `0x00890d64` -0.2, `0x00890634` 0.5, `0x00891640` -16,
`0x00890560` 20, `0x00890dac` 15, `0x00890d3c` 2, `0x00890da4` 5, `0x008910ec`
0.15). They give the rule of spec section 7.1 value for value. The action's
potentials at `0x00428450` hold the matched boost (`+0x8c`), the usable share
(`+0x90`), the full-tank seconds (`+0x94`), start and end grip and top speed; the
matching bias is zero unless the race status is in career mode.

Read live (two runs): every opponent's usable share is 0.375 with boost 0.75 and
a full tank of 5 s, the player's autopilot has share 1 and never presses the
button. The timer counts the off time from the moment the race action begins
(the scene hand-over); the first burn of each opponent starts 8.2 to 11.1 s after
the green light, when the timer has reached `-lerp(20, 4, skill)` (in one
instance 11.57 s off, skill about 0.53: base 0.4 plus catch-up). Opponents then
burn five to eleven times in the two laps, 0.4 to 13 s each. The rewrite's opponents had no nitrous choice.

No translated code is included in public sources.

## Second Razor race launch

The stock PC career frames archive records Razor at 1.78 m/s 0.217 s after
the 16.2.1 master timer hook. The first remake sample is at rest at timer
start. At 0.20 s the remake records 1.10 m/s, full throttle and no brake.
By about 5 s Razor is at 31.36 m/s in the stock sample and 26.23 m/s in the
remake. The stock archive does not sample the opponent's pedals, gear,
effective performance settings or target speed. The comparison brackets
an early pace difference without identifying which input caused it.

Across four stock PC career captures Razor trails by 0.43 to 0.93 s at
checkpoint 1. At checkpoint 8 he ranges from 5.45 s ahead to 5.40 s behind.
The recorded remake run trails by 2.83 s at checkpoint 1 and 28.32 s at
checkpoint 8. The stock and remake runs use different player controls and
traffic conditions. These times describe the recordings and do not set
racer tuning targets.

An additional PC career run identifies the Mustang by its start-grid position and racer role.
The stock engine reports 646.04 hp and the transmission has six forward gears. The installed
MustangGT ordinary maximum engine and induction records reproduce 646.04 hp. The base engine
produces 299.55 hp and has a five-speed transmission. The Razor appearance preset has no
physics record in the installed databases. The AI's native vehicle creation requests the full
ordinary performance match. The remake had skipped that match for this career event.

At the first stock sample, 0.017 s after the timer started, Razor was at 6,109 RPM. The earlier
remake capture has 1,467 RPM at the timer boundary. Those samples fall on different simulation
ticks. Matching the installed hardware does not establish staging RPM or checkpoint timing.
