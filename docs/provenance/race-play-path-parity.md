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
