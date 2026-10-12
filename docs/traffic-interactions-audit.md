# Traffic and world-interaction regression audit

This pass initially checked main `b675bf5` and was refreshed onto main `1473f1d`
(2026-10-12) before delivery. It checks truck cargo, breakable
scenery, traffic placement and accident controls, and prop contacts. It uses source
history, synthetic reproductions, installed-data tests and read-only original PC
analysis, followed by a live original prop-contact capture. Complete matched
original/remake driving comparisons are still pending.

## Confirmed fixes

| Problem | Cause and resulting behavior | Evidence |
|---|---|---|
| Truck cargo stopped falling after impacts | Startup no longer connected the loaded event sequences to vehicle physics. Restore that connection so log and car-carrier trailers receive their damage sequencers. | Both real-startup tests fail with the missing wiring. Installed impact tests release seven log-load parts and three carrier cars; uploaded meshes retain their textures and world pose at handover. |
| Scenery and pursuit-breaker fragments were missing | Startup also stopped supplying model hierarchies to the prop catalog. Restore the connection so resident props can resolve their fragment meshes. | Real-startup hierarchy regression plus existing installed breakup tests. |
| Traffic could appear too close | A random probe outside the 150 m exclusion radius could project onto a nearer road. Check the final lane position in the horizontal plane; retain the 20 m vehicle gap and additional trailer room. | A 160 m probe snaps to a 140 m road endpoint and is now rejected; elevation and trailer-spacing boundary cases are covered. |
| Trucks bounced immediately after spawning on slopes | Upright placement left long bodies inside the road, preloaded their suspension and put the hitch more than a metre apart vertically. Align traffic and trailer placement to the support normal, preserve horizontal heading and give them tangent velocity. | The old adapter fails the banked-road regression. Installed slope probes remove initial body penetration and excess suspension travel; trailer peak vertical speed falls from 8.86 to 3.15 m/s for the carrier and 8.58 to 3.21 m/s for the log truck. |
| Traffic swerved when its navigation cursor crossed a road join | The cursor rebuilt the next lane from its local start instead of carrying the previous endpoint and corridor edges. It also chose the next lane from the near profile. Preserve those endpoints and select the lane from the far profile. | The old code jumps 2.83 m in a synthetic turn and up to 23.98 m across 6,739 installed transitions. The fixed transitions have zero endpoint gap; continued movement, curved handles and corridor edges are covered. Original PC routines confirm both rules. |
| Trailer tires appeared pale or untextured | The shared loader omitted the traffic-only tire atlas substitution. Apply the original usage-2 rule from the base model name before gathering textures. | All eight assembled wheel placements across TRAILERLOG and TRAILERB now resolve to the installed TRAFFIC_TIRE atlas through the actual renderer material lookup. Nontraffic tire references retain their existing behavior. |
| Same-model vehicles or loads disappeared or relocated in Bevy | The shared uploaded rig supplied the same instance keys for every copy of a model. Allocate a persistent identity per physical car and trailer while retaining the shared meshes and materials. | Two installed loaded trucks emitted 19 parts each with all 19 keys duplicated. The fixed case has zero duplicate keys, retains those keys during motion and shares the same mesh handles. |
| Semi tractors steered at full lock after an impact | The ordinary traffic accident override applied to tractors and cop drivers too. Tractors now brake immediately with centred steering; cops retain their own driver controls. Tractor identity comes from the inherited body-behavior attribute, including after trailer loss. | Tractor, street-car and cop accident tests; installed tractor classification test. |
| Tractors followed too closely | Following-distance math omitted the tractor's doubled effective mass. Apply the factor only to this AI calculation. | At 10 m/s and 10,000 kg: 32.375 m for ordinary traffic, 50 m for a tractor. Physical mass remains 10,000 kg. |
| Some prop collision centres were displaced | Normalizing the stored placement quaternion before transforming the root pivot changed the authored translation. Expand the stored quaternion for that offset, then use a unit rotation for rigid physics. | Hand-computed transform cases, preserved visual pose and restoration; original PC placement and constructor checks. |
| Some poles and breakables used broad visual bounds | The catalog overwrote the first detailed bounds collection with a later duplicate of the same key. Preserve the first collection, matching the original pack constructor. | A live original traffic-light contact matches the detailed collection's narrow mast. Synthetic and installed regressions reproduce the old hierarchy replacement. There are 27 conflicting keys used by 1,487 placed spawners; this measures affected source selection, not completed gameplay validation for every placement. |
| An invalid traffic body could push a healthy car | The second body in a pair could reach positional separation before its nonfinite state was rejected. Validate both partners before contact dispatch. | Pre-fix regression displaced the healthy car by about 0.4 m; both pair orders now leave it untouched. Valid contacts still separate. |
| Damage-distance queries used a parked free camera while driving | Use the selected driving view with the live fixed-tick car pose for event-distance and lifetime queries. Share camera selection with rendering, including original views and dynamic FOV. | Real-startup positive-distance suppression regression and every selectable installed camera mode, including render-alpha independence and a nonzero nitrous FOV kick. Installed TrailerLog has suppression distance zero, so this was not the cause of its missing cargo. |

The first two failures were introduced when merge `e1e9dc4` combined the traffic
branch with the world-interaction implementation from PR #59. The underlying
sequence and fragment implementations remained present. Renderer integration
commit `d882b72` introduced the per-model render-key allocation that is incompatible
with Bevy's per-object instance pool. The immediate native renderer does not pool
objects by those keys, so that bug does not explain the reported native bouncing.
The other fixes address existing integration or correctness gaps; they are not
all attributed to that merge.
The lane-transition faults already existed in navigator commit `1b2d32d7`.

The prop placement correction changes 107 installed spawner centres by more than
2 mm relative to the old normalized calculation. Three street-light placements
move by approximately 0.48 m; the largest observed correction is about 1.27 m.
This establishes a placement error, rather than proving every pole shape is correct.

## Checks that ruled out speculative changes

- The spawning navigator starts as a probing cursor, but `can_traffic_spawn`
  changes it to traffic navigation and a legal traffic lane before driving starts.
  It is not accidentally retaining the racing navigation mode.
- The original traffic task's rate 0.1 is a fractional frame accumulator, not ten
  Hz. The original manager's rate 0.5 likewise means roughly two rendered frames,
  not half a second. Keep the rewrite's existing ten fixed-tick traffic cadence;
  correct the documentation instead of introducing a new timing change.
- Original TrailerLog's zero suppression distance disables its view-distance
  gate. Its lost cargo comes from missing startup sequences, not that gate.
- All five installed tractor/trailer combinations stay finite and hitched during
  a first-second flat-ground startup check. Their initial 0.171–0.172 m vertical
  hitch gap closes on the first tick; trailer travel stays below 0.027 m and both
  bodies remain nearly upright. This bounded case does not reproduce severe
  bouncing, so trailer alignment was left unchanged.
- A real highway startup check uses uploaded wheel/model data, resident collision
  packs and the installed cargo sequencers. A car carrier and log truck remain
  hitched through three seconds, with no world-damage events or cargo release.
  This single clear-road case does not establish behavior on banks, curves or
  contacts with other traffic.
- An actual-driver probe covers car carriers and log trucks at six separate
  curved or sloping highway sites. The slope trace reproduces the upright
  placement fault without driver input, with both installed collision geometry
  and an isolated matching support plane. After correction, all twelve bounded
  driving runs retain their hitches without world-damage events or cargo release.
  The steepest fixture's remaining vertical speed includes travel down the hill;
  these results do not establish smooth motion on every road or after impacts.

## Gray Point pole follow-up

The reported Gray Point location resolves to an authored breakable lamp, rather
than one of the inferred static colliders. Its installed intact-car fixture has
a deeper nonclosing contact followed by a shallower approaching contact. Keeping
only the deepest overlap discarded the breakaway hit. The original visits both
eligible primitive pairs and applies response between them. The installed fast
and slow controls distinguish breaking from pushing an attached pole.

The PC rigid-body update also resolves object pairs before end-frame world
contacts. The vehicle step now gives car/prop pairs that incoming velocity before
resolving walls. A separate regression combines a breakable pole with an authored
barrier: the pole receives its hit and the wall still constrains the car.

Scenery without an object spawner no longer gains fixed collision from a model
name or suffix match. The installed lamp census found 682 such inferred rigid
colliders, including other Gray Point placements. Their visual meshes remain;
authored world triangles/barriers still apply, and genuine spawner-driven lamps
retain their bodies and attributes. This separate correction does not explain
the closest reported pole by itself. The original static registration uses
authored world records. Standalone track-box integration remains a baseline gap.

The contact evidence uses read-only PC dispatch and isolated impulse execution
with the measured penetration rule between calls. These regressions establish
specific faults, not a matched full original driving route or all body phases.

## Pole replacement motion follow-up

The reported traffic light and the Gray Point streetlight both detach and invoke
their installed broken-pole replacement sequence. Their replacement uses a
movable material with a nonzero breakaway threshold. The fragment constructor
incorrectly reapplied the authored-placement attachment rule, so the physics loop
skipped the new body even though it had inherited linear and angular velocity.
This explains a mesh change that leaves the broken pole standing.

Read-only PC inspection confirms that runtime fragments clear the original
virgin-placement flag; world attachment applies only to virgin placements.
Released fragments now start unattached. Intact placement and restoration keep
their authored attachment rules. Material immobility and no-physics handling
retain their separate rules. Attribute-free visual-only pieces remain an
unimplemented adapter gap and do not explain the stationary full-pole child.

The synthetic regression checks falling, rotation, rendering and owner cleanup
for fragments with zero, positive and negative breakaway thresholds, alongside
placement/restoration controls. The installed regression checks physical hits,
replacement events and uploaded mesh transforms for both reported pole types.
These tests isolate the release pipeline; they do not establish every pole's
exact original gameplay response or the full world-contact solver.

## Remaining parity work

1. **Pole contact response.** The duplicate-record correction restores authored
   pole primitives and child hierarchies. The earlier broad fallback observation
   used the incorrectly selected collection; it did not establish the original
   live collider. A captured traffic-light contact now identifies the source
   error. Pair ordering, tolerances and non-unit placement orientation still
   need broader runtime comparison before claiming every pole behaves identically.
2. **Traffic bounce and lane motion.** Traffic currently uses the full vehicle
   engine and suspension, rather than the original traffic-specific behaviors.
   The verified endpoint discontinuity and far-profile lane selection are now
   corrected. A matched runtime route comparison remains useful.
   Ground-aligned placement fixes a reproduced startup fault, and the finite-state
   guard contains another proven fault. Neither establishes the cause of every
   visible bounce or swerve.
3. **Population timing.** The manager still runs every 0.5 simulated seconds.
   It caches models without the original inactive vehicle pool or four-second
   restriction on creating new instances. Spawn distance is corrected, but full
   spawn/recycle timing parity remains incomplete.
4. **Accident triggers.** The original view-distance gate, qualifying non-player
   impacts and restarting an ended accident are not fully represented.
5. **Contact dispatch and flags.** Prop compound contacts now visit eligible
   primitives with immediate response instead of keeping only the deepest hit.
   Global body visitation, vehicle-pair dispatch and the complete original
   world/object collision-disable flags still need broader comparison.
6. **Detached scenery.** Static-wall interaction, deferred events, some fragment
   flags and exact original suppression behavior remain incomplete. Existing
   frame-rate and installed-data tests cover the implemented paths, not full parity.

Prioritize repeatable driving/contact traces before replacing the traffic physics
model or adding collision-shape heuristics. The corrected street-light placements
and log/car-carrier trucks provide concrete regression targets.

## Validation

Focused reproductions were run against the old behavior before applying each fix.
The final checks include synthetic workspace tests, the installed world tests,
tractor attribute classification, source guard rails and both renderer builds.
The pole-motion follow-up workspace run passed 2,298 tests, with 261
install-dependent tests ignored. Its install-only world run passed 24 cases,
including the exact lamps' fast/slow contacts, replacement motion, restoration
and scenery registration. Restoring the old fragment attachment rule reproduces
zero child translation, rotation and render change for both installed poles;
the corrected initialization passes their release tests.
The preceding follow-up passed 82 installed data tests and 56 installed road
tests, including all 6,739 measured joins. Both native and Bevy Vulkan captures
cover Gray Point driving; preceding captures cover textured trailer assemblies.
Changed Rust files pass formatting, and the
leak and size checks pass. Whole-workspace formatting still reports untouched
main files. With GNU Rust 1.99, strict Clippy reports baseline dead code in the
front-end room, a constant-size chunk lint in an installed data test, and argument
count/nested-condition lints in the console and pursuit code. The all-targets
workspace run with Bevy enabled passes with those four lint categories allowed.
The live original prop contact and static routine inspection establish the
specific bounds-selection, lane-transition and tire-substitution corrections.
A complete matched original/remake driving comparison remains unfinished.

For manual verification, compare the same route and impacts in the original and
rewrite: a log trailer rear-end, a car carrier impact, a pursuit breaker, a slow
pass beside a lamp arm, and straight-road traffic at several frame rates. Record
positions and contact shapes alongside controls and collision events. Compare
spawn positions after road projection, not just the random probe distance.

Behavior and evidence records:
[traffic](specs/ai-traffic.md), [spawning](specs/ai-traffic-spawning.md),
[task timing](specs/ai-simulation-lod.md), [sequencers](specs/world-sequencers.md),
[traffic provenance](provenance/ai-traffic.md), and
[prop runtime provenance](provenance/prop-runtime-audit.md).
