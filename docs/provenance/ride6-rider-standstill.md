# RIDE 6 rider standstill and reverse legs: provenance

What the standstill work read, and from where. Spec: [ride6-rider-standstill](../specs/ride6-rider-standstill.md). Code:
`mods/ride6-bikes/src/rider/{graph_stand,graph_sets,style}.rs`, `crates/nfsmw/src/scenes/world/drive/rider/{standstill,params,style}.rs`,
`tools/ride6-exporter/R6ToolParts/{Anim,Bp}.cs`.

## Sources

| Source | What was taken | Status |
|---|---|---|
| The animation blueprint of the user's install (`RiderGP_racing_Master_animBlueprint`), printed as pseudo-code by CUE4Parse's Blueprint decompiler with the functions' bytecode read (`ReadScriptData`; the tool's `bp` mode) | The functions `Standing/StartStop_{BothFeet,LeftFoot,RightFoot}_W_I_P` (the moving test, the two smoothed 0 to 1 values, the explicit clip times, the racing threshold), `IK_Alpha_StartStop/Ground_*_W_I_P` (the foot IK conditions and their named inputs), the ubergraph's calls of both with their numbers per `BikeType` and `StandingLeg`, the class defaults `StartStop_Speed_Treshold` (6) and `StartStop_to_Racing_treshold` (0.85), the state names of `Main_SM` | Facts about the user's data asset (Kismet bytecode), not native code. The numbers and the structure are kept in `graph_stand.rs`; no pseudo-code is stored |
| The animation sequences `riderShared/Animations/{Race,naked,Scooter,enduro,motard}/*_{stand,start,stop,restart_idle}_*` | The clips themselves, exported by the tool's `anims` and `acl` modes; their lengths | The user's own export; nothing committed |
| `RIDING_STYLE` (`graphics/Bindings/Rider`), `RidingStyle_DT` (`data/datatables/Anim`) and the `RidingStyle` global option (`data/options/GameOptions`) | The foot that goes down by riding style, the option values | Data tables; they disagree for shoulders out (the bindings table puts the left foot down, the anim table both) and the bindings table was taken |

## What is a choice or an inference

Section 6 of the spec. The ones to remember: the state machine's transition conditions are read from the variables' names (the
transition rules are bytecode that only references them), the fade between riding and the standstill, the legs let go with the walk's weight, the
ground IK not played.

## Exported for this

`riderShared/Animations/Race/race_{stand,start,stop}_*`, `naked/naked_*`, `Scooter/rider_scooter_*` (Unreal's own codecs, `anims` mode into an `R6RA`
file) and `enduro/enduro_*` (ACL, `acl` mode); the 41 sequences of the standstill and the finger clips. They are added to the rider
already imported with `ride6-rider-import --merge` ([ride6-rider.md](../formats/ride6-rider.md)). Nothing is committed.
