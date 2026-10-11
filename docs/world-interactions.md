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
| Car response | Fixed friction/restitution, textbook impulse | 32-iteration solver matches 13 PC reference cases; per-body object materials and grounded player contact centre integrated; AI reaction overrides pending |
| Timing | Player/AI step separately; pairs collide once per render frame | Shared 60 Hz world tick; car-state and impact traces identical at 5/10/30/60/120/144/240 FPS and uneven frames; live validation pending |
| Geometry | Flattened prop hierarchy, single car box, no swept object contacts | Car/prop hierarchy and sphere contacts integrated; sphere/box sweeps match PC cases; box manifolds and box sweeps pending |
| Scenery | Name guesses, >150 kg immovable, lighter props disappear for six seconds | All 13,484 placements join exactly to scenery/attributes; persistent bodies, breakaway and no-car-effect response integrated; geometry and lifecycle parity pending |
| Trailers | Approximate anchors and response | Original named anchors read and used for all six installed variants; constrained joint solver and live comparison pending |
| Truck cargo | No released logs/crates | Original event packages and deterministic sequencer implemented; installed log damage trigger emits original fragments once; bodies/render integration pending |
| World/ground | Approximate probes and solver | Pending original comparison |
| Damage/feedback | Partial sparks/audio, incomplete collision events | Pending |
| Set pieces | Missing pursuit breakers, spike strips and scripted objects | Pending |
| Full comparison | No comprehensive repeatable collision suite | Pending live captures and numerical comparison |

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
Body construction selects immediate children, excludes internal fragment subtrees,
keeps disabled flags for contact filtering, and supplies missing fallback boxes.
The cone regression checks collision and an unchanged visual pose with a nonzero
root pivot. Four synthetic construction cases were also executed through the PC
routines. Ground response still uses existing primitive corner probes. Sphere
object contacts and sphere/box sweeps use the measured routines; box/box contact
points still use the earlier approximation. Lifecycle, centre markers, ground clouds and truck cargo release
remain open. Fallen props currently persist while resident; streaming out resets
them. These limits prevent claiming full parity.

## Event sequence validation

All 73 world packages and their 75 engines parse without embedding install data.
The log-trailer install test follows damage-zone IMPACT_1 through the original
state machines to DropLoad, verifies its fragment/explosion events and its empty
state after completion. Queue/timing traces match across render rates and stalls.
See [sequence provenance](provenance/world-sequencers.md) for coverage and limits.
This is event/data validation; falling cargo is not yet wired into the world.

## Vehicle geometry and hitch checks

The intact car body uses its enabled object primitives for car/prop and car/car
contacts, retaining gaps and disabled parts. Broad-phase radii include child
offsets and padding. The frame-rate regression uses a compound vehicle body.
The existing box contact-point algorithm remains an approximation.

All six tractor/trailer variants resolve their original named anchors. A flat
ground test exercises the four variants used in traffic patterns after their
unloaded suspensions settle. The unused cement trailer has unusual rotated
bounds: its stored anchor is lateral rather than below the body. An initial
trial leaves a 0.15 m gap during driving; that model needs separate original-game
validation. Joint constraints and spawning/suspension parity remain open.

Sphere contact tests compare face, corner, containment, tangency, moving-box and
swept vectors against isolated PC execution (0.00001 numerical tolerance). The
world test launches a sphere across a stationary box in a different grid cell
within one tick; momentum transfer and body traces match at 5–240 render FPS.
Travel includes primitive rotation. Box/box sweeps remain unimplemented, and the
coincident-sphere contact point deliberately avoids the PC's residual-memory bug.
