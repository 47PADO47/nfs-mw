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
in which the rider walks to the bike and raises it. Its durations (the constructor's defaults: 1.0 s before the get-up, 4.0 s of get-up, 1.0 s to raise the bike, 14 s of recovery) were read in round 12, and the install holds no standing or walking clip of the fall (section 8). Not modelled.

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
4. The fallen bike waits for the rider ([ride6-rider-getup](ride6-rider-getup.md)): once it has lain still it is stood up only when his
   get-up has played to its end (the clip's length; at most 8 s of get-up, or 8 s of waiting for one that has not begun) [M], so the bike
   is not back under the player's hands while he is still getting up. A bike put back by hand (the reset button) takes him back at once.
   The return is a blend from where he stands to the seat, taking the way at a jog (3 m/s, 0.25 to 2 s) [M]; the walk to the bike is
   not modelled (see [ride6-rider-getup](ride6-rider-getup.md), section 3).

A rider without `falls.json` falls limp as before; one without `ragdoll.json` keeps to his bike.

## 4. Choices and what is not modelled

| Choice | Value | Why |
|---|---|---|
| Strength of the drive | 0.5 of the way per solver pass | the reference's 1,000,000 is a rigid hold; this is the stiffest the solver keeps stable |
| Fade of the pose | `1 - exp(-3 t)` | `SmoothFloat_Interpolate(., 3)` not read |
| Face and side axes | the pelvis body's, against the bone's rest orientation | the owner's transform sync was not read |
| Airborne distance | the pelvis' height over the ground | the rider information's trace was not read |
| Frozen get-up | aligned on the pelvis, upright | the raise-bike phases were not recovered |
| Hold of the bike for the rider | get-up to its end (8 s at most), 8 s if not yet begun | the phases' durations were not recovered; see [ride6-rider-getup](ride6-rider-getup.md) |
| Hold of a clip's last frame | `TimeMode::Once` | the clips' loop flags are not in the dumps; each launch ends as its fade ends |

Not modelled (no clip for it, section 8): the standing, the walk to the bike and the raising of it; the airbag alpha (`Ragdoll_Airbag_Alpha`, the smoothed `IsFallen`, an input of the
airbag, which the rider's graph does not use for these clips); the character-physics stances (kinematic and animation strengths, `GetUpTimeAccumulator`);
the impulse curves of the animation instance (`RagdollImpulseCurve`, not set in the blueprint); the physics blend weight of the bodies
(a field of the component, its constructor default was not read).

## 5. Verified by tests

`blackbox-skeleton`: the launch by kind, the hand-over a blend before the dominant clip ends and the smooth fade, the tumble by face and side
and by the flying ramp, the get-up only when still, on the ground and in a band, the pose fade, the data round trip and its checks, the
dominant clip and `TimeMode::Once`. `blackbox-vehicle`: a driven joint holds against gravity, follows a moving target, lets go, does nothing at
gain 0. `ride6-bikes`: the clips each tree picks (against the blueprint's bindings), the get-ups by lying, the refusals of a pack without a
clip. `nfsmw`: the bodies are driven to the animation's pose, the get-up freezes the body and lifts the pelvis, the bike waits for the
get-up, a rider without the set falls limp. Offscreen screenshots of a bike thrown into a wall: `docs/provenance/ride6-physics-D.md`.

## 6. The physics asset the rider really uses

`RiderBP`'s mesh has `PhysicsAssetOverride = RiderGP_PhysicsAsset_CB` and the material `PM_RiderRagdoll_CB` [D]; the earlier pages used
`RiderGP_PhysicsAsset` (11 bodies, 66.7 kg). The `_CB` asset has 16 capsules (pelvis 18 kg, `spine_01` 12, `spine_03` 12, head 12, upper arms 5,
lower arms 5, hands 3, thighs 7, calves 5, feet 2: 108 kg) and 15 joints, with larger limits (hips 130/90/70 degrees, shoulders 120/100/60,
neck free, knees 90/30/40). The hands are boxes, not capsules: the converter stands for a box by the capsule along its longest side with
the mean half-size of the other two as its radius [M] (a hand of 9.0 x 6.2 x 8.4 cm becomes a capsule of radius 3.65 cm and a straight part of 1.7 cm).
`ride6-rider-ragdoll` takes either asset; use the `_CB` one.

## 7. Seen on screen (offscreen runs of a scripted crash)

`--drive-script "2:throttle=1;0.02:fall=low;N:throttle=0"` crashes the bike on cue ([the script's page](../modding/drive-script.md)); a
screenshot run plays the script out and captures its last moment, so one run is one frame: about 100 frames from 80 runs (N from 0.1 to
12 s, low and high side, `--camera-mode far` and `superfar`, the last at 2560x1440 and cropped), in `C:/r6/shots/fall`. KTM 450, flat
road, low side after 2 s of full throttle (seconds after the fall):

| t | What is on screen |
|---|---|
| 0.6 | the bike is over on its side; he is still on it, being thrown |
| 1.2 | in the air, flat out ahead of the bike, as the launch clip holds him |
| 2.0 to 4.0 | tumbling, then sliding on his back and face down; the bike lies still 4 to 5 m behind |
| 4.5 to 5.5 | rolling to rest (the rolling and sliding loops follow how he lies) |
| about 5.6 | he is still enough, face down: the get-up is entered (1.0 s crossfade, then the 1.4 s clip) |
| 5.8 to 8.0 | on hands and knees pushing up: the clip's last frame is that pose (pelvis 0.36 m, head 0.57 m [D]; the back clip ends sitting, the side clip kneeling: standing and walking are the phases not recovered) |
| 8.05 | the bike is stood up (the get-up has played: 1.0 + 1.4 s after it began); he is still kneeling where he lay |
| 8.1 to 9.1 | he goes back to the seat (about 1.0 s for 3.5 m) and is seated at 9.1 s; control was the bike's from 8.05 |

The ragdoll follows the clips (the launch pose, the loops, the get-up), the get-up is played, the bike stays down until it has played, and the
rider is on the bike again without a jump. What was wrong and is fixed: the bike was stood up where it lay, 4 to 5 m from where he
knelt, and the 0.25 s return blend carried him there in a fifth of a second (a pop of metres); it now takes the way at 3 m/s
(`RETURN_SPEED`, at most 2 s) [M]. A bike put back by hand still takes him at once. High side (2 s of throttle): thrown over the bars, face
down and sliding at 2.5 s, up and seated by 6 s. At 69 km/h (8 s of throttle) the bike cartwheels (its own box dynamics) and
the rider is flung clear; on the stretch of road used it ends in the verge. What stays open: the middle of the return blends a kneeling
pose into a seated one in the air (no walk clips), and the bike's tumble at high speed is the box double's, not the animation's.

## 8. The raise-bike sequence: what the install holds (round 12)

The question was whether the clips of the standing and walking phases exist, and were only not exported. **They do not exist as clips of the get-up.** What was searched
(the headless `R6Tool GAME_UE5_6 list <regex>` and `R6ToolRider json`, which read the user's pak files and never start the game; the class layout from `ride6.usmap`):

- **Every animation package under `characters/`** (1609 by the folder names `Anim`/`Animations`): the rider's own are the 700 of `riderShared/Animations` that the exporter lists (158 ACL, 542 per-track; folders `Ragdoll` 22, `Race` 102, `naked` 69, `motard` 54,
  `Scooter` 55, `endurance` 43, `enduro` 32, `TrafficLight` 32, the cutscene folders `HQ`, `Podium`, `Pit*`, `StartingGrid`, `EndCareer`, `FirstBoot`, ...). The names matching get-up, raise, pickup, lift, upright, stand-up, walk-to-bike, run-to-bike or recover over the whole install are 106 packages:
  the four get-up clips of the fall (`race_ragdoll_Back_GetUp`, `_FaceDown_GetUp`, `_Side_GetUp_L`/`_R`, already in `falls.json`); `Rider_HQPro_StandUp_NFS` (5.23 s, the headquarters cutscene [by its name and folder]) and its female twin; `Endurance_RunToBike_L/R_Var1..12` (5.7 s, plus the
  female set and `Endurance_Idle_RunToBike_L_Var1`, 11.6 s) which the animation blueprint plays in the endurance start (the variables `Endurance_Run`, `Endurance_Animation`), the mechanic's `Mech2_PitPro_WalktoBike_NFS`; the rest are trucks and a sound. The walk clips (`Rider_ChampionPresentation_walk_NFS` 13 s, `rider_EndCareer_Walk_NFS` 10.8 s, `Rider_FirstBoot_Desk_Walk_NFS`,
  `Rider_HQPro_WalkToSit_NFS`, `..._WalkWardrobe_NFS`) are cutscene clips of the menus. No montage or composite asset exists (the names `montage`, `AM_`, `AnimComposite`: two sponsor materials), and the bikes' animation packages (`bike*_animBlueprint`, 292 of them) are blueprints, no sequence.
- **The rider's animation blueprint has no walking or standing state**: its 570 variables have none (no locomotion speed, no walk, raise or stand flag), and the only run clips it names are the endurance ones. The fall machine ends in the four get-up states of section 1; they hold the last frame.
- **The raise-bike component has no animation property.** `UIgnitionRaiseBikeComponent` (usmap, 34 properties) holds character-physics configuration *names*, durations, collision-channel lists and references to other components (the snap component, the character-physics component, the simplified ragdoll, the rider movement component);
  no sequence, montage or clip. The walking is done by `UIgnitionRiderMovementComponent` (55 properties: speeds, accelerations, root-motion curve objects and the *names* `StartingRunAnim`, `StartingWalkingAnim`, `RunningAnim`, `WalkingAnim`, `StoppingRunAnim`, `StoppingWalkingAnim`, with their lengths and `bUseAnimationsToCalculateMovement`); its constructor
  (`0x145b2c000`) leaves the names empty and sets the root-motion curve name to `DistanceCurve`; a blueprint of the pilot sets them [I], and that blueprint is not in the packages named like a rider, pilot, pawn or bike blueprint (`RiderBP`, `BaseMotorBike` and their siblings hold none of these components). None of the 700 rider clips has a
  curve named `DistanceCurve`, so the walking clips named there are not in the rider's folder, or are not rider clips at all.
- **The durations are native defaults** (constructor `0x145b2b810`, matched to the usmap's property order; every constant read, none guessed): `FallCharacterPhysicsConfig` "Fall", `SafeRagdollRespawnThreshold` 15, `SafeFallCheckTimer` 0, `bSafeFallCheckTimerEnabled` true, `bSkipBeforeGetupPhase` false,
  `BeforeGetUpCharacterPhysicsConfig` "BeforeGetUp", **`BeforeGetUpDuration` 1.0 s**, `BeforeGetUpDelayDuration` 1.0 s, `GetUpCharacterPhysicsConfig` "GetUp", **`GetUpDuration` 4.0 s**, `bSnapshotPoseOnGetUp` false, **`BikeRecoveryTimerDuration` 14.0 s**, `BlockVolumeComponentTag` "BlockVolume",
  **`RaiseBikeDuration` 1.0 s**, `BrokenBikeDuration` 1.0 s, **`TimeToReachPosition` 0.1 s**, `MaxReplicationErrorDistance` 200, `bSeamlessTransitionEnabled` true. The component is not in `RiderBP` or in `BaseMotorBike`; if a pilot blueprint overrides these, it was not found.

So the sequence of the reference is: the ragdoll lies (Fall), 1.0 s before the get-up (+1.0 s delay), a 4.0 s get-up phase in which the character-physics configuration "GetUp" stands him [I: the fall machine's get-up clip is the pose he is driven to; nothing says a further clip stands him up], then the walk
to the bike by the movement component, then 1.0 s to raise the bike, the whole inside a 14 s recovery timer. **Nothing in the install is a standing or a walking pose of the fall**, so none is modelled: it would be an invented animation. What is *not* changed for that reason: the get-up still ends on the clip's last frame
(hands and knees, or kneeling, or sitting), and the bike is stood up when it has played ([ride6-rider-getup](ride6-rider-getup.md)); wiring the 4.0 s `GetUpDuration` and the 1.0 s raise would hold him in that pose for up to 3 s longer and then move him to the seat without a walk. The durations are kept here for the day a standing pose and a walk exist.
