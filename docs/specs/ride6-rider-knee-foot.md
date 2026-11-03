# RIDE 6 rider: the knee out, the foot out, and a lean that does not snap

Why the rider's knee never came out on a road bike and his foot never left the peg on a supermoto, what RIDE 6's animation
blueprint (`RiderGP_racing_Master_animBlueprint`) does instead, and what this project plays now. Sources: the blueprint as
pseudo-code (CUE4Parse's Blueprint decompiler: the functions `Rider_Lean_Polish`, `LeaningAnimations`,
`IK Overrides and Smoothing`, `PreTurn_KneeAndLeg`), its node graph and property bindings, the clips of the install (the 185 of the
rider pack, sampled). Nothing is copied: the behaviour is in words and equations. Provenance:
[ride6-rider-knee-foot](../provenance/ride6-rider-knee-foot.md). Earlier: [ride6-rider-styles](ride6-rider-styles.md),
[ride6-rider-animation](ride6-rider-animation.md). Tags: **[D]** read from the data, **[M]** a modelling choice.

## 1. What was wrong

The lean clips were read as three poses (`Left`, `Centre`, `Right`) that play as 2 s loops and are blended by the size of the
lean. They are **not loops**. Each lean clip (`race_Rossi_standard_Left`, `motard_Leaning_Knee_LowSpeed_R`, ... 44 of them over the
five types) is a 2 s **sweep of the whole body**: its first frame is the rider hung off to one side (pelvis 15 cm over, the inner knee
19 to 21 cm out of the line of the bike), its middle frame (1 s) is upright, and its last frame is the rider hung off to the other
side [D, measured on every lean clip of every type]. The blueprint evaluates them at an **explicit time** that is a function of the
lean. Played as loops the clips made the rider sway from one side to the other every two seconds at any lean, and the knee was out
only for the part of each cycle in which the clip happened to be there. That is why the knee "never came out" at the lean, and the
sway is what made a release of the stick look like a snap.

## 2. How RIDE 6 poses the lean [D]

`Rider_Lean_Polish` ends with two assignments (the variables are the blueprint's):

```
RiderLeanLeft_Explicit  = 1 - RiderLeaning        (seconds into the clip)
RiderLeanRight_Explicit = 1 + RiderLeaning
```

`RiderLeaning` is the polished lean, -1 (left) to 1 (right) ([ride6-rider-round6](ride6-rider-round6.md), section 4, and
`crates/nfsmw/src/scenes/world/drive/rider/params.rs`). The left clip (`Leaning_Standard_Left`) is evaluated at `1 - lean` seconds
and the right clip (`Leaning_Standard_Right`) at `1 + lean` seconds, each by a `SequenceEvaluator` (a held frame, not a player).
Both clips cover the whole range, from hung off to the right to hung off to the left (the left clip starts hung off to the right,
the right clip to the left); they differ in how the body travels between the ends. The pose is

```
standard = Blend( Blend(LeftClip @ 1-lean,  RightClip @ 1+lean,  RiderLean_LeftOrRight),  Standard_Pose,  LeanOrStraight_Alpha )
aero     = Blend( Blend(AeroLeft @ 1-lean, AeroRight @ 1+lean, RiderLean_LeftOrRight),  AeroPose,      LeanOrStraight_Alpha )
riding   = Blend(standard, aero, RiderTuckedIn_Alpha)
```

and the two signals that choose are made by `LeaningAnimations` ("leaning Out and In turn anim"):

| Signal | Rule | Numbers |
|---|---|---|
| `RiderLean_LeftOrRight` | `SmoothFloat_Interpolate(BikeLean_Derivative > 0.15, speed)`: the right clip while the bike rolls to the right faster than 0.15 of its steepest lean a second, else the left one | speed 5 for the race and naked types, 6 for the bagger, 4 for the others |
| `LeanOrStraight_Alpha` | `SmoothFloat_Interpolate(max( FuzzifyTrapezoid(BikeLean, -0.25, -0.12, 0.12, 0.25) * (1 - abs(VehicleOriginalSteer)) * (1 - ramp(abs(PreturnAdditionalLeaning))), RiderLean_SpeedReducerCondition), 2)` | the upright pose below 12 percent of the bike's lean with a neutral stick, the lean sweep above 25 percent, the upright pose again from 250 km/h (ramp to 400); the variable starts at 1 |

**The knee-out threshold of the data is therefore not one number.** The inner knee is out by the clip's own sweep: at a bike lean of
0.12 the pose is still the upright one, by 0.25 it is the lean sweep, and the sweep hangs the pelvis 9 cm over at a quarter of the
polished lean and 15 cm at half, but keeps the inner knee at the bike until the last quarter (the sweep brings the knee in as the
body crosses over): it is out by 6 cm at three quarters and 19 cm at full lean (race `Rossi` set, `Schwantz`: the same; measured
in section 3). A bike that reaches 0.8 of its steepest lean (a polished lean of 1) shows the knee out in full.

## 3. What was changed (knee out, on every bike with a lean sweep)

- `ride6-rider-import` builds each lean set as above (`mods/ride6-bikes/src/rider/graph.rs`): the two clips swept by the `lean`
  parameter with `TimeMode::Fraction` (the left one mirrored), crossed by the new parameter `lean_out`, crossed to the centre pose by
  the new parameter `straight`, then tuck over it. The parameters are 18 now (`lean_out`, `straight` are the last two).
- The game makes the two signals (`rider/lean_pose.rs`, with the blueprint's constants above; the bike type from `styles.json` sets the
  rate of `lean_out`). `straight` starts at 1 as the variable does.
- A graph made before this reads 0 for both new parameters and plays the left sweep: install the new rig (section 7).

Measured on the shipped clips and the new graph (tuck 0, speed in the sitting range; x is lateral, the rider's right is negative, in
cm; the pelvis and the inner knee, from the graph evaluated on the shipped pack by a scratch probe that is not kept; the `probe` test in
`rider/probe_tests.rs` dumps the same for any parameters):

| Lean (right) | 0 | 0.25 | 0.5 | 0.75 | 1 |
|---|---|---|---|---|---|
| `race` set: pelvis x | 0 | -9 | -15 | -15 | -15 |
| `race` set: right knee x (pelvis-relative / absolute) | -27 / -27 | -20 / -29 | -12 / -27 | -18 / -33 | -31 / -46 |

(The right knee is the inner one for a right lean; in the middle of the sweep the body is on its way and the knee passes through the
seat before it goes out; the figures at 1 are the end frame of the clip.)

The ground-trace knee layers (`LeftKnee_Correction_Alpha`, `TraceKnee_*`) and the `*_ass_knee_*` buttock volumes are still not
played ([ride6-rider-styles](ride6-rider-styles.md), section 7).

## 4. The foot out on a supermoto [D, with a choice]

In the blueprint the inside foot comes off the peg in **one** place: `IK Overrides and Smoothing`. It runs only when
`RidingStyle == 2` (old school) and `BikeType` is 2 (supermoto) or 4 (enduro), or on an off-road scooter (not modelled):

```
Leaning_FootDown_L = BikeLean < -0.3          Leaning_FootDown_R = BikeLean > 0.3
OverrideFootIK_Leaning_L/R = SmoothFloat_Constant(Leaning_FootDown_L/R, LeanFootDwn_Speed = 4)
LeftFoot_IK  = (foot on a footboard or the ground) * (1 - HasLeftLegOut) * (1 - OverrideFootIK_Leaning_L) * ...
RightFoot_IK = ...                                                 * (1 - OverrideFootIK_Leaning_R) * ...
```

so beyond 30 percent of the bike's steepest lean the foot on the lean side is no longer pinned to its peg and keeps the place the
lean clip gives it. The old-school clips (`motard_Old_School_*`, `enduro_Old_School_*`) put that foot **19 to 21 cm out and 12 to 17 cm
up** at the end of the sweep (measured: `motard_Old_School_standard_L`, last frame: left foot x +38 against +19 upright, z -35
against -52). The other styles (`motard_Leaning_Knee_*` and `motard_Leaning_*`) never move the foot more than 3 cm, and the IK
pins it anyway. So the foot-out of the supermoto **is** the old-school riding style; there is no foot-out in the knee style.

Here:

- `rider/foot_out.rs`: the override, with the blueprint's 0.3 and 4 a second, only when `styles.json` says the bike's tree was made for
  riding style 2 and its type is 2 or 4.
- `rider/pose.rs`: `Goals::release` (left, right) lowers the IK weight of the limb named `left_leg` or `right_leg` in `attach.json` to
  `1 - release` and gives the foot back its animated turn by the same share. Released fully, the leg is the clip's pose.
- The importer gives each bike type its own riding style (`ClipSet::riding_style`): **the supermoto plays the old-school style (2)**,
  every other type the first (0, the Rossi / Balanced / knee set). `--style N` still forces one style on every type. This is a
  **choice [M]**: the style is the rider's own setting in RIDE 6 (the pilot's `RidingStyles_Offroad`, `LegOut_Offroad`) and the data
  has no default; the supermoto is given the one that has the foot out because that is what the request is. The enduro type keeps
  the knee style: the enduro bikes are adventure bikes (the Honda CRF 450R SMT, a supermoto the correction table calls `ENDURO`, is
  the exception: `--style 2` for a rig that wants him with his foot out).
- The in-game importer (`mods/ride6-bikes/src/job.rs`) used the plain style 1 for every type; it now passes none, so each type gets
  the above.

**Not done:** the pre-turn leg-out (`PreTurn_KneeAndLeg`: the inside leg swings out when the rider brakes hard, 0.5 of the lever and
1200 N of front suspension load, towards a bend that the track's spline says is within 150 m, below 0.3 of the lean; the clips
`*_leg_out_*_down_Rossi` and `_up`). It needs `NextCurveDistance` and `NextCurveDirection`, which come from the race track; free roam
has none. It is also only enabled by `LegOutSwitch`, the pilot's `LegOut` setting, which the blueprint never sets itself.

## 5. A lean that does not snap

Reproduced with the real bike model (`rider/smooth_tests.rs`): the bike is put at 30 m/s, held in a corner with the stick at 1 for 4
s, and the stick is let go; every number the graph reads is followed frame by frame (60 Hz). The result before the change, and after:

- The parameters were already smooth, and no jump could be reproduced in them: `lean` is `Interpolate(.., 4)` of a mix (0.3 of the
  controller's target, 0.7 of the bike's roll) with a fastest change of 4 a second; the tuck, the wheelie signals, the drift and the
  foot override are `SmoothFloat_Constant` or `Interpolate` too; the hang-off shift follows at the data's 4 a second. The stick
  never reaches the graph except through the controller's second-order target, which relaxes over 0.5 s.
- What the old graph did differently was the pose: with the clips played as loops, the pose at a constant lean was a moving one (the
  pelvis swung 15 cm either way every 2 s), so a change of the lean rearranged a moving pose. With the sweeps the pose is a pure
  function of the lean (and of the two smoothed signals): continuous, monotonic as the lean falls, as slow as the bike's own roll.
  I could not reproduce a one-frame snap in the parameters; if one is still seen after the rig is imported again, the next place to
  look is the physics' rider body (`rotation`, passed on unsmoothed by design).
- The new tests hold the rates: `lean` at most 5 a second, `tuck` 6.5, `lean_out` 6.5, `straight` 2.5 (a quarter of a second to
  half the distance), the foot override 4.5; the pose 0.1 s after the stick is let go is still at least 0.8 of the held lean; the
  lean never rises on the way back; and it is back at upright within 0.6 s of the bike's own roll.

## 6. What is data and what is a choice

| Item | Origin |
|---|---|
| The lean clips are 2 s sweeps evaluated at `1 -/+ RiderLeaning` seconds | [D] (`Rider_Lean_Polish`, the property copies of the evaluators) |
| `RiderLean_LeftOrRight` and `LeanOrStraight_Alpha`, their thresholds (0.15, 0.12 to 0.25, 250 to 400), rates (5/6/4, 2) | [D] (`LeaningAnimations`) |
| Aero straight pose = `AeroPose_Low` | [M] (the blueprint blends `AeroPose_High` and `_Low` by `AeroTankCorrection`) |
| Foot override: old school, supermoto and enduro, 0.3 of the lean, rate 4 | [D] (`IK Overrides and Smoothing`) |
| The supermoto plays riding style 2, every other type 0 | [M] |
| The leg's IK weight `1 - release` and the foot's turn slerped by the same share | [M] (the blueprint's control rig does the equivalent in cooked data) |
| Left/right-leg limbs found by the names `left_leg`, `right_leg` | [M] (the rig's own names) |
| No pre-turn leg-out, no knee-to-ground trace | not done (section 4) |

## 7. Install

The rider rig has to be imported again (the graphs and `styles.json` change; `rider.bbsk`, `attach.json` do not):

```
target\release\ride6-rider-import.exe 'C:\r6\rider\export-styles' mods\ride6-bikes\files\RIDER
```

and the same into `%APPDATA%\nfsmw\config\mods\ride6-bikes\files\RIDER` (or run the mod's own import from the game, which now picks
each type's style). `--style N` forces one riding style on every type (0 knee, 1 plain, 2 old school). The bikes' `contacts.json` need
no new import.
