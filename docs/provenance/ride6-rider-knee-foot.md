# RIDE 6 rider knee out and foot out: provenance

What [ride6-rider-knee-foot](../specs/ride6-rider-knee-foot.md) read, and from where. Code: `mods/ride6-bikes/src/rider/{graph,graph_sets}.rs`,
`crates/nfsmw/src/scenes/world/drive/rider/{lean_pose,foot_out,params,pose,style}.rs`.

## Sources

| Source | What was taken | Status |
|---|---|---|
| The animation blueprint of the user's install, as pseudo-code printed by CUE4Parse's Blueprint decompiler | `Rider_Lean_Polish` (the two assignments `RiderLeanLeft_Explicit = 1 - RiderLeaning`, `RiderLeanRight_Explicit = RiderLeaning + 1`, and the polish already in `params.rs`); `LeaningAnimations` (`RiderLean_LeftOrRight`, `LeanOrStraight_Alpha`, their rules and constants: 0.15, -0.25/-0.12/0.12/0.25, rates 5/6/4 and 2); `IK Overrides and Smoothing` (the foot override: `RidingStyle == 2` on `BikeType` 2 or 4, 0.3, rate 4); `PreTurn_KneeAndLeg` (its call `15000, 3, 0, 1, 3, 0.5, 1200, 0.3, 6, 5, 120, 150, 120, 150`, read only to say what is not done) | Facts about the user's data asset (Kismet bytecode); the branch structure was followed by scratch scripts, only the numbers and the rules are kept |
| The blueprint's node graph (the class's default object as JSON) and its property-access table | The tree under the `Racing` state (which node reads which variable; the lean evaluators are `SequenceEvaluator` nodes whose clip and explicit time are the copies `Leaning_Standard_Left` with `RiderLeanLeft_Explicit` and `Leaning_Standard_Right` with `RiderLeanRight_Explicit`, and the aero pair); the leg-out players and the three-way blend on `PreTurn_3BlendAlpha` | Data facts |
| The 185 clips of the rider pack, sampled by scratch tests (scratch tests, not kept) | Length 2 s of every lean clip; the pelvis, knee and foot places at the first, middle and last frame of each (the sweeps' ends: pelvis +-15 cm, inner knee 19 to 21 cm out; the old-school supermoto clips: inside foot 19 to 21 cm out and 12 to 17 cm up) | Measurements of the user's own data |
| `RidingStyle_DT`, `RIDING_STYLE`, the pilot struct of the SDK dump | `LegOut`, `LegOutType`, `LegOut_Default`, `LegOut_Offroad`: the leg-out is a rider setting | Names and flags only |

## What is a choice

See section 6 of the spec: the supermoto plays riding style 2; the aero straight pose; the IK weight `1 - release` of a released leg
and the foot's turn; the limb names.

## Exported for this

Nothing new: the clips and the graph came from the export folder of the earlier rounds (`C:\r6\rider\export-styles`); only the
importer's graphs and `styles.json` change.
