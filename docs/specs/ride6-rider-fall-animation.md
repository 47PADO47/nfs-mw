# RIDE 6, the rider's fall animation: which clip plays, how it is blended, when he gets up (clean-room spec)

What the rider's animation blueprint (`RiderGP_racing_Master_animBlueprint`), the rider blueprint and the executable (build
`5.6.1-1697004`) do with the 21 clips of `riderShared/Animations/Ragdoll/` once the rider has left the bike, and what this project does
with it. It continues [ride6-rider-fall](ride6-rider-fall.md) (the launch and the ragdoll's bodies). Tags: **[B]** read from the
binary, **[D]** from the data assets, **[S]** from the class dump, **[I]** inferred, **[M]** a choice of this project's model.
Addresses are in [the provenance page](../provenance/ride6-physics-D.md), section "Fall animation".

## 1. What RIDE 6 does

**The ragdoll follows an animation.** The rider's skeletal mesh is not a free ragdoll [B, D]. `EnableSimulation` sets the mesh to
simulate below the root body (the pelvis), applies the component's physical animation settings to every body and sets the *physics blend
weight* of the bodies (`BlendWeight`, a field of the ragdoll component); every tick the rider's animation instance writes the
component's **strength multiplier** from the curve `Ragdoll_strength_Race` [B] (the curve is the constant **1,000,000** from 0 to 10 s
[D]; the optional second curve, by time since the fall, is not set [D]). The animation blueprint's pose is the target of the bodies'
angular drives, in the local space of the parent body; the pelvis, the root, is free and takes the launch velocity and the ground. A
drive stiff by that many orders of magnitude holds each body at the pose of the animation as if it were rigid: **what the player sees
is the animation, carried by a pelvis the physics moves**. The rider mesh's physics asset is `RiderGP_PhysicsAsset_CB` [D] (16
capsule bodies, 15 joints with generous limits that fit the animations; see section 6), not `RiderGP_PhysicsAsset`.

**The graph.** The animation blueprint's final node is a two-way blend `A: riding graph, B: state machine Falling`, its alpha the
variable `IsFallen_Alpha` [D]. `IsFallen` is 1 as soon as the ragdoll simulates and 0 when it stops: `OnRagdollSimulationChanged` sets it,
copies the fall type, resets the clock of the animation instance and calls `EventFallen(IsFallen, 1 - IsFallen)` [B]. The alpha is
`SmoothFloat_Interpolate(IsFallen, 3)` [D]; the library function was not read, a first-order approach at 3 per second is assumed [I]
(0.78 after half a second, 0.95 after a second). So the pose the bodies are driven to goes from the riding pose to the fall animation's in
about a second.

**The state machine `Falling`** [D] (five states; the node numbers of the compiled machine run opposite to the order of the class's
node list: node `n` of a baked machine is entry `449 - n` of the list, which is how the states were matched to their graphs):

| State | Plays | Leaves for |
|---|---|---|
| `Side` | the sub-machine `Fall_Side` (below) | `Falling`, when the state is nearly over (automatic remaining-time rule), crossfade **0.6 s** |
| `Falling` | the sliding, rolling and flying loops (below) | `FaceDown_end` (**1.0 s**), `Side_end` (**0.2 s**), `FaceUp_end` (**1.0 s**) by the rules of section 2 |
| `FaceDown_end` | `race_ragdoll_FaceDown_GetUp` | nothing |
| `FaceUp_end` | `race_ragdoll_Back_GetUp` | nothing |
| `Side_end` | `race_ragdoll_Side_GetUp_L`, `_R` | nothing |

All the crossfades are the engine's standard blend with the Hermite cubic curve, `3k^2 - 2k^3` of the elapsed share `k` [D]. A state's clips start at
time 0 when the state is entered and run at the clips' rate; the end states hold their last frame.

**The sub-machine `Fall_Side`** has a *dispatcher* state (a one-frame standing pose) and two states [D]; its two transitions (crossfade 0)
are the rules `FallType == 3` (low side) and `FallType == 2` (high side) [D]. The type is the one the rider component stored (2 high side, 3
low side, 1 other [B]); for the other kind the dispatcher stays in its one-frame pose and the machine leaves `Side` at once.

- **Low side** plays `ragdoll_lowside_center_R` and nothing else: the two-way blend between `_L` and `_R` has its alpha fixed at 1 [D]. The clip
  is 0.467 s long, shorter than the crossfade, so `Falling` starts to fade in at once.
- **High side** plays seven clips blended by `RagdollSide` and `RagdollFace` [D]: a three-way blend on the side (positive: the `_L` set, zero:
  `HighSide_Front`, negative: the `_R` set) over two sets of three on the face (positive `High_FaceDown`, zero `Side`, negative `FaceUp`). The
  names are the blueprint's: a positive face value plays the `FaceDown` clip. The clips are 1.13 to 1.8 s long.

The state `Side` is left by the **automatic rule**: when the remaining time of the state's dominant clip (the one of greatest blend weight) is
under the crossfade, 0.6 s [B, D: the engine's `bAutomaticRemainingTimeRule`, trigger time -1 meaning the crossfade]. So a launch plays for the
length of its dominant clip less 0.6 s, then fades into the tumble over 0.6 s, ending as the clip ends.

**The tumble** (`Falling`) [D]: a two-way blend on `Ragdoll_FlyingAmount` between the ground blend (alpha 0) and the flying loops (alpha 1).
The ground blend is a three-way on `RagdollFace` (positive: `Sliding_FaceUp`, zero: `Rolling`, negative: `Sliding_FaceDown`), each of its three a
left and a right clip by `RagdollSide`: the slides and the get-ups by `0.5 side + 0.5`, the roll by `side` alone (so only a positive side
brings in the right clip) [D]. The flying blend is the three `fall_front_loop` clips (`_L` negative, centre zero, `_R` positive) by the side.

## 2. The inputs, and when he gets up

The variables are fields of the rider's animation instance, written every tick by native code [B] (`0x145b24c50`):

| Variable | Value |
|---|---|
| `RagdollSpeed` | the length of the owner's velocity, cm/s times **0.01**: m/s |
| `RagdollFace` | the vertical part of the owner's forward vector (+1 face up, -1 face down, 0 standing) |
| `RagdollSide` | the vertical part of the owner's right vector |
| `Ragdoll_isLowSpeed` | `RagdollSpeed < 0.5` [D] |
| `Ragdoll_isFlying` | `AirborneDistance > 1` [D] |
| `Ragdoll_FlyingAmount` | `FuzzifyRamp(AirborneDistance, 1, 2.5)`: 0 up to 1, 1 from 2.5 [D] |
| `AirborneDistance` | the rider information's distance from the ground, traced down from a socket [S]; not read further |

The owner is the actor's root component, which follows the pelvis when the ragdoll simulates [I: the engine's component-to-body sync was not
read]. The get-up rules of `Falling` [D] are `Ragdoll_isLowSpeed AND NOT Ragdoll_isFlying AND` a band of `RagdollFace`: below -0.6
`FaceDown_end`; between -0.2 and 0.2 `Side_end`; above 0.6 `FaceUp_end`. **That is the "ready to get up" condition**: the pelvis nearly still
(under 0.5 m/s), within 1 (distance) of the ground and lying in one of the three bands; in between the bands he keeps tumbling. The clips are
then 1.0 s (back), the face-down one (an ACL clip) and 2.53 s (side).

What follows the get-up clip is a **raise-bike component** [S, B]: phases riding, fallen, before get-up, get-up, walk, each with a
character-physics configuration name and a duration (`BeforeGetUpDuration`, `GetUpDuration`, `RaiseBikeDuration`, `TimeToReachPosition`),
in which the rider walks to the bike and raises it. Those durations and the walk clips are not in the files read (the component is a native
subobject of the pilot; its constructor defaults were not traced), and the walk clips are not exported. Not modelled.

## 3. What this project does

**Data** (`mods/ride6-bikes/src/rider/falls.rs`, tests there): the importer writes `RIDER/falls.json`, a `blackbox_skeleton::fall::FallSet`:
the launches by kind (`high_side`, `low_side`; no launch for the others), the tumble tree, the three get-ups with their face bands and
crossfades, and the thresholds above, as ordinary blend trees over the pack's clips with the parameters `t`, `face`, `side` and `flying`.
The values of section 1 and 2 are in that file's constants with their sources.

**Runtime** (`libs/blackbox-skeleton/src/fall/`, generic, no game names): `FallPlayer` is the state machine (launch, tumble, rest) with the
crossfades, the automatic hand-over, the band tests and the `IsFallen_Alpha` fade (`fade`); `TimeMode::Once` plays a clip once and holds
its last frame; `Node::dominant_clip` names the clip a tree is dominated by.

**The ragdoll** (`libs/blackbox-vehicle/src/ragdoll/`): a joint can be *driven* (`Ragdoll::drive`): the child's orientation in its parent's body
frame is pulled to a target by a share of the way per solver pass (`set_drive_gain`), through the same inverse-inertia shares as the
limits; limits and contacts win over it.

**The game** (`crates/nfsmw/src/scenes/world/drive/rider/fall.rs`, `fall_anim.rs`):

1. At the release the ragdoll is built as before, its joints get drive links (the two bones and the bodies' axes) and a gain of **0.5** per
   pass (16 passes a step) [M]: the reference's strength is so high that the pose is held rigidly; half the way per pass holds it within a few
   degrees and keeps the solver stable.
2. Each physics step, before the ragdoll steps, the player is told what the pelvis does (`face` and `side` from the pelvis body's axes, with
   the front and the right of the body taken from the pelvis bone's rest pose [I], `speed`, and `airborne` as the pelvis' height above the ground
   [M: the reference's trace was not read]). The pose it returns, mixed with the riding pose the fall began from by the fade (0 at the start),
   is the target; the joints are driven to it. A bone with no body follows its parent in that pose.
3. When the player enters a get-up the body is **frozen** and the get-up clip plays in a frame that puts the pose the bodies were driven to on
   the pelvis where it lies, set upright (the ground is taken as level) [M]: a driven free pelvis cannot stand up, and the reference leaves the
   physics at that point too (the get-up phase of the raise-bike component). The player's own crossfade (1.0 s, 0.2 s on a side) blends the
   tumble into the clip.
4. The bike stood up (`off` false): the rider gets back on at once if he has no animation; if he is in the get-up, when the clip has played
   (at most 8 s); if he has not begun to get up, after 2 s [M]. The return is the earlier blend of 0.25 s from where he stands [M]; the walk to
   the bike is not modelled.

A rider without `falls.json` falls limp as before; one without `ragdoll.json` keeps to his bike.

## 4. Choices and what is not modelled

| Choice | Value | Why |
|---|---|---|
| Strength of the drive | 0.5 of the way per solver pass | the reference's 1,000,000 is a rigid hold; this is the stiffest the solver keeps stable |
| Fade of the pose | `1 - exp(-3 t)` | `SmoothFloat_Interpolate(., 3)` not read |
| Face and side axes | the pelvis body's, against the bone's rest orientation | the owner's transform sync was not read |
| Airborne distance | the pelvis' height over the ground | the rider information's trace was not read |
| Frozen get-up | aligned on the pelvis, upright | the raise-bike phases were not recovered |
| Hold of the rider after the bike is up | get-up to its end (8 s at most), 2 s if not yet begun | the phases' durations were not recovered |
| Hold of a clip's last frame | `TimeMode::Once` | the clips' loop flags are not in the dumps; each launch ends as its fade ends |

Not modelled: the walk to the bike and the raising of it; the airbag alpha (`Ragdoll_Airbag_Alpha`, the smoothed `IsFallen`, an input of the
airbag, which the rider's graph does not use for these clips); the character-physics stances (kinematic and animation strengths, `GetUpTimeAccumulator`);
the impulse curves of the animation instance (`RagdollImpulseCurve`, not set in the blueprint); the physics blend weight of the bodies
(a field of the component, its constructor default was not read).

## 5. Verified by tests

`blackbox-skeleton`: the launch by kind, the hand-over a blend before the dominant clip ends and the smooth fade, the tumble by face and side
and by the flying ramp, the get-up only when still, on the ground and in a band, the pose fade, the data round trip and its checks, the
dominant clip and `TimeMode::Once`. `blackbox-vehicle`: a driven joint holds against gravity, follows a moving target, lets go, does nothing at
gain 0. `ride6-bikes`: the clips each tree picks (against the blueprint's bindings), the get-ups by lying, the refusals of a pack without a
clip. `nfsmw`: the bodies are driven to the animation's pose, the get-up freezes the body and lifts the pelvis, the return waits for the
get-up, a rider without the set falls limp. Offscreen screenshots of a bike thrown into a wall: `docs/provenance/ride6-physics-D.md`.

## 6. The physics asset the rider really uses

`RiderBP`'s mesh has `PhysicsAssetOverride = RiderGP_PhysicsAsset_CB` and the material `PM_RiderRagdoll_CB` [D]; the earlier pages used
`RiderGP_PhysicsAsset` (11 bodies, 66.7 kg). The `_CB` asset has 16 capsules (pelvis 18 kg, `spine_01` 12, `spine_03` 12, head 12, upper arms 5,
lower arms 5, hands 3, thighs 7, calves 5, feet 2: 108 kg) and 15 joints, with larger limits (hips 130/90/70 degrees, shoulders 120/100/60,
neck free, knees 90/30/40). The hands are boxes, not capsules: the converter stands for a box by the capsule along its longest side with
the mean half-size of the other two as its radius [M] (a hand of 9.0 x 6.2 x 8.4 cm becomes a capsule of radius 3.65 cm and a straight part of 1.7 cm).
`ride6-rider-ragdoll` takes either asset; use the `_CB` one.
