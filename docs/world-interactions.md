# World interaction parity work

Target: original Most Wanted PC 1.3 behavior, including car impacts, scenery,
breakable objects, trailers, loose cargo, and pursuit/world set pieces.
Full parity has not been established. Record evidence separately for data,
isolated routines, scene integration and live-game comparisons.

## Base

- Traffic stack: [PR 24](https://github.com/47PADO47/nfs-mw/pull/24),
  [PR 25](https://github.com/47PADO47/nfs-mw/pull/25), and
  [PR 29](https://github.com/47PADO47/nfs-mw/pull/29), ending at
  `291a72e451d3058d9c1f55b115bc7d30708e8de7`.
- Traffic lights are a rewrite extension: disable them for original-parity runs.
  Unrelated renderer PRs are not required by this work.

## Work ledger

| Area | Starting gap | Status |
|---|---|---|
| Car response | Fixed friction/restitution, textbook impulse | Measured 32-iteration solver, original per-body materials, player contact centre and installed AI collision profiles integrated; live comparison and original goal states pending |
| Timing | Player/AI step separately; pairs collide once per render frame | Shared 60 Hz world tick; camera and car render between the same ticks; camera input retained across frames; driving continues while distant scenery loads; live comparison pending |
| Geometry | Flattened prop hierarchy, single car box, no swept object contacts | Car/prop hierarchy and measured sphere/box contacts integrated; 5,012 box cases match PC, including travel, clipping and tumbling; compound response order pending |
| Scenery | Name guesses, >150 kg immovable, lighter props disappear for six seconds | All 13,484 placements join exactly to scenery/attributes; breakaway, per-object lifetime, sinking and restoration integrated; static trigger recreation and live comparison pending |
| Trailers | Approximate anchors and response | Original named anchors, geometric correction, 32 point-moment iterations and conical/prismatic limits integrated; cement spawn frame and live comparison pending |
| Truck cargo | No released logs/crates | Original rear-impact triggers release seven logs or three carrier cars into collidable/renderable bodies; explosion effects and lifecycle parity pending |
| World/ground | Approximate probes and solver | Measured iterative plane response, original probe tolerances, surface friction and disabled-ground flags integrated; cached face queries and live comparison pending |
| Damage/feedback | Incomplete collision events and camera feedback | Measured contact camera shake integrated; collision audio remains deferred |
| Set pieces | Missing pursuit breakers, spike strips and scripted objects | Installed support/roof, scaffold and gas-pump sequences integrated; exact triggers, explosion/damage, feedback and spike strips pending |
| Full comparison | No comprehensive repeatable collision suite | Pending live captures and numerical comparison |

Further parity implementation is deferred while the movement regression is fixed.
The actionable backlog and acceptance checks are in
[remaining world interaction work](world-interactions-todo.md).

## Movement regression

The player was rendered between consecutive 60 Hz poses while the chase camera
used the latest physics pose. At 60 m/s this produced up to a metre of relative
offset and alternating movement at high render rates. Both now interpolate
position, camera target and field of view from the same two ticks. Cleanup still
observes the current physics camera; drawing cannot change physics or timers.
Spawn/reset snaps both camera endpoints to the new car.

Camera input is a frame delta. It now accumulates on frames without a physics
tick and divides across multiple ticks, instead of being lost or repeated.
The app supplies real elapsed time without its former 0.1-second clamp, so low
frame rates and stalls do not discard physics time. Per-second look input uses
the same real clock.
Streaming no longer pauses the whole world for every newly requested distant
tile. The collision grid's nearby packs must be resident; old-zone collision and props
remain available during the new zone's load. A necessary loading pause retains
the displayed interpolation phase so resuming cannot rewind the car. Installed
checks verify local coverage using actual collision-pack IDs; the start zone
`A1` has no same-numbered streaming tile and gets its road from `A41`.

Presentation regressions check smooth camera/car movement at 5, 30, 60, 120,
144 and 240 FPS plus uneven frames, unchanged fixed-tick follow/collision
traces, camera input totals and reset behavior. Opt-in runtime diagnostics use
`RUST_LOG=nfsmw::movement=trace`; each `frame` record includes physics/render
positions, camera endpoints, interpolation phase and streaming readiness.

## Verification

Run `cargo xtask check`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
Run synthetic checks with no `NFSMW_GAME_DIR`; set that variable explicitly for
ignored install tests. Keep local tool configuration, build output and private
research outside tracked source. Game data and decompiled output are never
published. Compilation and synthetic tests alone do not prove gameplay parity.

## Dynamic scenery validation

Synthetic contacts exercise momentum transfer, the no-car-effect override,
attachment thresholds (including permanent attachment), secondary prop impacts,
repeated impacts on fallen objects, interpolation, moving grid entries and tile
unload. Complete prop position/orientation/velocity traces match bit for bit at
5, 10, 30, 60, 120, 144 and 240 render frames per second.

The install checks resolve 13,484 attribute/visual pairs; 13,479 have track bounds.
They also check cone, tree, traffic-light and log mass, tensor and detach values.
An exhaustive runtime check additionally constructs all 13,479 resolved bodies,
checks their original identity, mass, inertia, attachment and initial visual pose,
and exercises collision registration and unload. Shared collision/attribute keys
no longer reuse another model's name. Body construction selects immediate children, excludes internal fragment subtrees,
keeps disabled flags for contact filtering, and supplies missing fallback boxes.
The cone regression checks collision and an unchanged visual pose with a nonzero
root pivot. Four synthetic construction cases were also executed through the PC
routines. Ground support includes enabled installed clouds and box corners, with
16 contacts retained. Plane response matches measured PC vectors, including sliding
and off-centre rotation; height probes use the original velocity-dependent tolerance
and root extent. The closed-cloud support fixture can settle on either its base or
its side. Cached face queries and live ground comparisons remain open. Sphere
object contacts and sphere/box and box/box displacement tests use measured routines.
Separation now translates the incident contact point and temporary collision centres;
prop pairs use original normal-speed shares. Per-object offscreen and off-world
clocks, resting, dropout sinking, fragment release and placement restoration run
on fixed ticks. Resting scenery with no dropout retains its final pose; streaming
out resets it. Frustum/LOD/occlusion equivalence, cached world positions, static
trigger recreation and centre markers remain open. These limits prevent claiming
full parity. See [lifetime rules](specs/prop-lifetime.md).

## Event sequence validation

All 73 world packages and their 75 engines parse without embedding install data.
The log-trailer install test follows damage-zone IMPACT_1 through the original
state machines to DropLoad, verifies its fragment/explosion events and its empty
state after completion. Queue/timing traces match across render rates and stalls.
See [sequence provenance](provenance/world-sequencers.md) for coverage and limits.
Vehicle collisions now feed the original damage-zone stimuli. Successful fragment
spawns hide the corresponding uploaded parent mesh and retain its world pose in
the shared prop solver. Installed rear-end collision tests release seven log
meshes and three carrier-car meshes, verify one load per trailer and parent
cleanup, and compare complete fixed-tick body traces at 5–240 render FPS.
Missing meshes leave parent parts visible. Original side-zone scaling does not
release cargo. These CPU tests use installed meshes and bounds; an interactive
collision/render comparison with the original game remains necessary. Trailer-local explosions,
shock and load-loss feedback remain unimplemented. Detached-part cleanup uses the
original separate offscreen timer and parent ownership.

## Vehicle geometry and hitch checks

The intact car body uses its enabled object primitives for car/prop and car/car
contacts, retaining gaps and disabled parts. Broad-phase radii include child
offsets and padding. The frame-rate regression uses a compound vehicle body.
The box contact algorithm agrees with 5,012 isolated PC cases; contact ordering
across compound primitives remains open.

All six tractor/trailer variants resolve their named anchors and both angular
constraints. Five variants, including every one used in installed road patterns,
remain hitched after their unloaded suspensions settle, with under 0.00002 m
anchor separation. Full turning/braking/collision traces match at 5–240 FPS.
The unused cement trailer has a rotated body/kingpin frame that does not settle
in the current spawn adapter. Its spawn frame, original-game comparisons,
singular rotations and immobile limit velocity response remain open. See
[constraint evidence](provenance/trailer-constraints.md).

Sphere contact tests compare face, corner, containment, tangency, moving-box and
swept vectors against isolated PC execution (0.00001 numerical tolerance). The
world test launches a sphere across a stationary box in a different grid cell
within one tick; momentum transfer and body traces match at 5–240 render FPS.
Travel includes primitive rotation. Box crossing tests also transfer momentum
across grid cells and have identical traces at 5–240 FPS. The
coincident-sphere contact point deliberately avoids the PC's residual-memory bug.

## Pursuit-breaker integration

Original scenery sequences now drive named fragment models, support-to-roof area
messages, timed collapse events, explosion expansion and stop-cop areas. A
collision with the installed tire-shop support brings down its roof; the beach
scaffold releases four pieces; a gas-pump impact yields 31 surviving pieces
through the support/roof chain after ten seconds. Full body/force, model-state,
visibility and stop-event traces match at 5, 15, 30, 60, 120, 144 and 240 FPS and
uneven frame durations. Both the local tile and permanently shared solids supply
art in these CPU checks, matching the runtime's asset lookup. Parent unload
removes all child physics and grid registrations. Synthetic cleanup checks
cover parent release during a pass that already contains its child IDs.

The player explosion target is independent of the non-player vehicle target,
matching the measured PC bitmask. Waves apply a force only when outward point
velocity is below expansion speed, once per target; faster departing bodies
receive no additional acceleration. Stop-cop areas brake pursuing cops inside
the sphere and include later entrants for their fixed-clock lifetime.

This does not establish complete pursuit-breaker parity. Initial impact triggers,
simple-physics behavior, explosion contact points/shell selection and damage,
cinematics, authored effects/audio, score/wreck notifications and exact stop-cop
AI behavior still have gaps. See [breaker provenance](provenance/pursuit-breakers.md).

## Contact camera feedback

Acceleration after world and car-pair contacts drives the measured PC shaker.
Its waveform uses the same presentation time as the player and chase camera;
sampling leaves the physics follow/cleanup view unchanged. Thirteen native
sampler vectors and a native view-transform fixture cover the isolated math.
Presentation checks compare active shake at the same elapsed time at
5/10/30/60/120/144/240 FPS and uneven frame durations. Teleports clear it.
Authored explosion/cinematic camera events and full gameplay comparison remain
open. See [the camera spec](specs/contact-camera-shake.md). Collision audio is
deferred and is outside this contribution.
