# Normal-play race follow-up

Spec: [race-play-path-parity](../specs/race-play-path-parity.md).

The user supplied three modded PC race captures and reported normal-play failures.
The third uses a stock Carrera GT, Hard, catch-up On, six opponents, no traffic
and police disabled. Installed localization identifies Rockridge & Union as
event `14.2.3`, 8264.796 metres. The player finishes second in 121.865 seconds;
the winner finishes in 120.977 seconds. These are observations of that setup,
not acceptance thresholds for every car, route or difficulty.

Source inspection finds that `Pausable` dropped NIS audio ownership and feedback,
despite bare-world audio tests passing. Independent vgmstream r2168 decoding
matches every channel and PCM sample of the inspected countdown stream. That
rules out the decoder for this stream; it does not establish audible playback
or complete A/V synchronization. Raw streams and recordings remain private.
Installed IntroNis02 and IntroNis04 rev files contain 202 and 323 points on
channel zero respectively; all fifteen other channels are empty. The extended
custom-grid primary-cue fallback is a remake policy, not proof of native actor
binding for every starting position.

The winning capture opens `InGamePhotoMaster.fng` about 0.050 seconds after its
finish callback with race simulation frozen. The recorded camera moves before
holding. Installed footer structure supplies the anchors for contextual prompts.
Host controller glyphs are a remake integration with the existing binding system.

The stock tuning refusal was reproduced with installed pause menus: after Accept
on the disabled entry, Back could not resume. The final pause flow opens the page
for a supported mode even when all categories are locked. It snapshots the driven
car without restoring the loaded career's selected car. Installed tests cover
entry, locked categories, Back/Resume, unchanged unrelated career data and owned-car
tuning persistence. See the [tuning contract](../specs/tuning-ui-lifecycle.md).

PC static analysis uses executable SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
`GRacerInfo::CreateVehicle` at `0x005e8e60` supplies normalized `[1, 1, 1]`
through `VehicleParams::matched` at offset `0x28`. The constructor at
`0x00689820` passes that target to `Physics::Upgrades::MatchPeformance` at
`0x0067c310`. The matcher uses stock/max normalized endpoints and ordinary
package counts. This establishes maximum ordinary race hardware for valid
endpoints, independently of the action planner's later player matching.
`CreateVehicle` takes the vehicle key from the character's front-end preset
ride when one resolves, otherwise from its car type, and builds the `[1, 1, 1]`
target after that choice for both. At the constructor's call site
(`0x00689967`) the matcher call is guarded only by a non-null target pointer;
no preset or customization test was found there. The installed `presetride`
class has 25 records and none for the `OPM_*` presets that `16.1.1` names.
Unattended runs of `16.1.1` in the unmodded original (2026-11-18) put the
`gto`, `imprezawrx` and `rx8` opponents at 52 to 60 m/s ten seconds after the
green light, where the remake's stock-hardware opponents reached 40 to 44.
The quick action's default zero bias follows the existing `Physics::Info`
and `AIActionRace` reference declarations. No translated code is included.

The PC behavior selector at `0x0067f0d0` overrides suspension to Simple for
racer driver class 3, despite the stored `pvehicle` suspension field. This
corrects the earlier claim that Simple aerodynamic catch-up was inactive.
The engine and Simple suspension reference interfaces describe the existing
positive-torque, drag and downforce scales in the speed/skill spec. These are
per-instance modifiers, not changes to installed car data.
The installed full-course regression also exposed an integration omission:
the spawned `CarSim` never set its existing racer collision classification.
Stalled cars had zero supported wheels despite a road ray beneath them. The
terrain-mask contract already distinguishes racers (zero) from traffic (`0x40`).
Set that classification in the common racer spawn path, retaining police and
ordinary traffic classifications.
That classification correction alone did not clear the installed start failure.
The next diagnostic confirmed that the stalled cars were overturned, while their
cached road normals and ray hits agreed. The existing reference `RBVehicle` and
`Chassis` staging contracts apply to any staging vehicle, including AI racers.
The host had wired only brake pedals for held rivals, and the shared staging
flag incorrectly required a player launch configuration. The follow-up wires
physical staging and reuses spring-rest placement for race cars. Pitch and roll
remain physically simulated, and no player launch boost is granted to rivals.

The race candidate also reuses the project's spring-rest placement correction
from commit `0de4221094fe98beec458e80ee86640a2e71a1b3`, including its spec and
regressions. This removes the initial static-sag drop when a car is placed on
the road. It is independent of authored NIS body animation.

The initial capture did not decode opaque vehicle source keys into opponent
models. Later stock-race captures resolve model parents, as recorded in
[race-stock-pace](race-stock-pace.md). Native car-selection distribution, exact
grid randomization and remaining pace differences need separate verification.
The remake's controlled player-trajectory probe measures the production settings,
physical rivals and catch-up without claiming a complete behavioral match.

## Start grid of `16.1.1` (2026-11-18)

Install data (unmodded reference install): `16_1_1_circuit/16_1_1_startgrid` holds
only `Position` and `Rotation`. `16_1_1_circuit/opponent` names
`16_1_1_circuit/opponent/spawn` (a `carmarker`, 109 m ahead of the start) as its
`ForceStartPosition`; the remake used it as that racer's grid slot. The race's
`IntroNIS` is `IntroNis10`, a type 4 scene, whose `car1` to `car4` channels end
at (1634.50, 382.94), (1634.92, 380.12), (1635.14, 377.51) and (1634.18, 385.92).
An unattended observation of the running original, supplied with the task, put
the player and the three opponents at those points at the green light and had a
car at the forced-start marker only before the cars were placed. The other
standing-start intros end in rows about 3 m behind the start; the Blacklist boss
intros and the rolling D-Day intro do not end in a grid. No executable code was
read for this change.

## Opponent launch of `16.1.1` (2026-11-18)

Observation of the running unmodded original, given with the task (three
unattended runs: medium, no traffic, two laps, three opponents, standing start):
one second after the green light the `gto` is at 7.8 to 8.3 m/s, the
`imprezawrx` at 11.4 to 12.0 and the `rx8` at 7.7 to 8.1; the first checkpoint
follows at 7.47, 7.17 and 7.33 to 7.35 s. Also given: the opponents' base skill
is 0.4 and their catch-up behaves as the remake's. In the remake's trace the
opponents had full throttle and a requested speed above their own from the
first sample, so the pedals did not limit them.

Install data (unmodded reference install, maximum ordinary packages as the
remake reads them): `gto` 1693 kg, rear drive, peak 551 N m, first gear
2.97 x 3.3, red line 6200; `imprezawrx` 1480 kg, torque split 0.5, peak 550 N m,
first gear 3.2 x 3.0, red line 7000; `rx8` 1631 kg, rear drive, peak 489 N m,
first gear 3.7 x 3.5, red line 9000. A flat-ground full-throttle test of these
records (`drive/sim/racer_launch_tests.rs`, ignored, needs the install) on the
racer tire left the two rear-drive cars at their limiters in first gear with the
rear tires at 3 to 13 percent traction until about 2.5 s; on the simple tire
they reached 8.3 and 9.9 m/s at one second against 5.7 and 6.4, and the
`imprezawrx` was within 0.6 m/s on both tires at every mark.

The specs already state the two rules applied: the behaviour selector's Simple
suspension for race opponents (above) carries the simple tire of the police
chassis spec, and the driver specs give staging racers 0.8 gas and the staging
speed request, which the host replaced with full brakes and an unchanged
governor. No executable code was read for this change.

## Stuck opponent at the seventh checkpoint of `16.1.1` (2026-11-18)

Remake runs only. In five of six remake runs one opponent left the course about
50 m past the seventh checkpoint and drove for 30 to 45 s in a low dead end
north-east of it. Its think log showed it on its route the whole time: the
route's last leg went that way. The install's road graph forks 14 m before the
checkpoint; the race's barrier lines close the north-east connector, and the
leg's start had snapped to it, so the strict search failed and the fallback
lifted the lines near the gate. The other racers left that road only when the
corner threw them wide and the off-path check put them on the course road. The
barrier spec already has race routes avoid closed segments. No executable code
was read and nothing about the original's routing is claimed.
