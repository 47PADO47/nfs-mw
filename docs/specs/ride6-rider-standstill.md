# RIDE 6 rider standstill and the reverse legs

How RIDE 6's rider puts his feet down when the bike stops and takes them up when it pulls away, how his legs come off the pegs for
the reverse walk, and how this project plays both. Sources: the animation blueprint of the user's install
(`RiderGP_racing_Master_animBlueprint`) as pseudo-code printed by CUE4Parse's Blueprint decompiler (the bytecode of its functions is
read with `ReadScriptData`; the class defaults, the state machine records and the exposed graph nodes come with it), the animation
sequences of `riderShared/Animations/`, and the data tables. Nothing is copied: the behaviour is written in words and numbers. Provenance:
[ride6-rider-standstill](../provenance/ride6-rider-standstill.md). The pose itself: [ride6-rider-animation.md](ride6-rider-animation.md);
the reverse walk's clips: [ride6-rider-styles.md](ride6-rider-styles.md), section 4.

## 1. The states

The blueprint's `Main_SM` has the states `Standing`, `Racing`, `Start` and `Stop` (and ones for the pit, the starting grid and the
menus that are not played here). `Racing` is the riding pose of [ride6-rider-animation.md](ride6-rider-animation.md); the other three
play one clip each, chosen by the **foot** the rider puts down (`StandingLeg`: 0 both feet, -1 the left, 1 the right):

| State | Clip (race and naked / supermoto / scooter / enduro) |
|---|---|
| `Standing` | `race_stand_*`, `naked_stand_*` / `motard_restart_idle_*` / `rider_scooter_stand_*` / `enduro_restart_idle_*` (`both_feet`, `foot_L`, `foot_R`) |
| `Start` | `race_start_*`, `naked_start_*` / `motard_start_*` / `rider_scooter_start_*` / `enduro_start_*` |
| `Stop` | `race_stop_*`, `naked_stop_*` / `motard_stop_*` / `rider_scooter_stop_*` / `enduro_stop_*` |

The standing clips are long loops (4 s and more, the head turns and the shoulders settle); the start and the stop clips are 1.7 to
4.7 s long. All of them are in `graph_sets.rs` (`ClipSet::legs`).

## 2. What moves the states

`Standing/StartStop_BothFeet_W_I_P` and its left and right twins make two numbers from the bike's speed (`VehicleSpeed`, km/h) every
frame [D]:

- the bike is **moving** when `VehicleSpeed > StartStop_Speed_Treshold`, 6 (or the rider has fallen);
- `StartClip_Explicit_Normalized` = `SmoothFloat_Constant(moving, Start_SmoothSpeed)`: 0 to 1 while the bike moves, back to 0 when it
  does not, `Start Anim Speed` a second;
- `StopClip_Explicit_Normalized` = `SmoothFloat_Constant(not moving, Stop_SmoothSpeed)`: the opposite, `Stop Anim Speed` a second;
- the start clip plays at the second `StartClip_Start + start value * (StartClip_End - StartClip_Start)`, the stop clip likewise
  (`StartClip_Explicit`, `StopClip_Explicit`: stepped by an explicit time, not played);
- `StartStop_to_Racing` is true when the start value reaches `StartStop_to_Racing_treshold`, 0.85 (`Racing blend start (0 to 1)`).

`Standing` goes to `Start` when the start value is above 0, `Start` to `Racing` at 0.85, `Racing` to `Stop` when the stop value rises, and
`Stop` to `Standing` at 1. **The conditions of the transitions themselves are in bytecode that reads these variables; they are
read here from the names** (a choice, section 6). A bike that sits at rest in the scene starts in `Standing` (the machine's initial
state is the first record).

The numbers (the calls of `SET Animations`; speeds a second, windows in seconds of the clip) [D]:

| | window of the start clip | window of the stop clip | start / stop rate |
|---|---|---|---|
| `BikeType` 0, 1, 5 (race, naked, bagger): both feet | 0 to 3.7 | 1 to 3.7 | 0.7 / 0.6 |
| left foot | 0.9 to 3.2 | 0 to 3.3 | 1.15 / 1 |
| right foot | 0.2 to 3.6 | 1 to 4 | 0.85 / 1 |
| `BikeType` 2, 3, 4 (supermoto, scooter, enduro): both feet | 0 to 1.15 | 1.1 to 3.7 | 1.2 / 1 |
| left foot | 0 to 1.65 | 1.1 to 2.5 | 0.7 / 1.4 |
| right foot | 0 to 1.5 | 1.1 to 2.4 | 0.8 / 1.4 |

A window that runs past the end of its clip holds the last frame (the supermoto's stop clip is 2.9 s).

## 3. The feet on the pegs

The legs' IK pins each foot to its peg. `IK_Alpha_StartStop/Ground_*_W_I_P` decides, from the seconds the clips are at, whether it does
(`LeftFoot_IK_onFootboard`, `RightFoot_IK_onFootboard`, each followed at `Left Foot IK Smooth Speed` / `Right Foot IK Smooth Speed`)
and whether the ground IK takes over (`..._onGround`) [D]:

```
isStart       = the start value is not falling
onFootboard   = (not isStart and stop second < "Stop: off Footboard at")  or  (isStart and start second >= "Start: on Footboard at")
onGround      = (not isStart and stop second > "Stop: on Ground at")      or  (isStart and start second < "Start: off Ground at")
```

Per foot four seconds: `off Ground at` and `on Footboard at` (start clip), `off Footboard at` and `on Ground at` (stop clip), with the
smoothing rate of the two feet. They are the arguments of the calls, in `graph_stand.rs` (`ROAD`, `OTHER`: for example the race
bikes' both-feet call is `(0.3, 3.2, 1.3, 1.7)` for the left foot and `(0.3, 1.9, 3, 3.3)` for the right, 3 a second). A foot that does not go
down (the other one, when one foot is put down) is always on its peg. This project plays `onFootboard` as the strength of the peg
IK (1 on the peg, 0 the leg as the clip poses it); the ground IK (`Standing_GroundDetection`, a trace under the foot) is **not
played**: the clips put the foot where it is authored.

## 4. The reverse legs

In the reverse walk ([ride6-rider-styles.md](ride6-rider-styles.md), section 4) the legs are the clip's: the blueprint's feet IK is off
while `RetroStart` and `RetroLoop` play. Here the legs' peg IK is let go in step with the walk's weight (`reverse`), so the rider's feet
leave the pegs and paddle the bike back. Before this the legs stayed pinned to the pegs under the walk clip, which showed only the
upper body.

## 5. What the game does with it

`crates/nfsmw/src/scenes/world/drive/rider/standstill.rs` makes the signals from the bike's signed speed and the walk's weight: the
graph's `stand` (the weight of the standstill over the riding pose, followed at 6 a second), `stand_state` (0 the stop clip, 1 the
standing clip, 2 the start clip), `stand_leg`, `stop_time` and `start_time`; and the strength of each leg's peg IK
(`Values::foot_release`: the old-school foot out ([ride6-rider-knee-foot.md](ride6-rider-knee-foot.md)), the standstill and the walk
let go, the strongest wins). The graph is `graph_stand.rs`; the numbers travel in `styles.json` (`stand`, [ride6-rider.md](../formats/ride6-rider.md)).

The foot that goes down follows the riding style (`START_LEG` of the `RIDING_STYLE` presets, [ride6-rider-styles.md](ride6-rider-styles.md),
section 8): balanced the right foot, shoulders out the left, old school both. The car viewer's rider stands: his clips play in place.

## 6. What is data and what is a choice

| Item | Origin |
|---|---|
| The clips of each state, the windows, the rates, the IK seconds | Data (`SET Animations`, the calls of the standing functions) |
| The moving test, 6 km/h, and 0.85 | Data (the class defaults, the standing functions) |
| The state machine's transitions from the values reaching 0, 0.85 and 1 | Inference from the names (the transition rules' bytecode names the variables, the conditions are read from them) |
| The fade between riding and the standstill, 6 a second | Choice (the blend times of the state machine are in cooked data) |
| The ground IK (`Standing_GroundDetection`) | Not played |
| The peg IK strength as `onFootboard` | The blueprint's own alpha, used as the IK strength |
| The foot from the riding style | Data (`RIDING_STYLE`); the blueprint's `RidingStyle_DT` disagrees for shoulders out (both feet there) |
| Legs let go with the walk's weight | Choice |

## 7. Cost

Adding the 41 sequences (32 sampled by the tool in under a minute, 9 ACL decoded by `blackbox-acl`) to the rider with `ride6-rider-import --merge` takes
0.3 s and grows `rider.bbsk` from 9.80 MB to 12.83 MB (+3.0 MB, the clips of 120 to 143 frames each) and each of the five graph files
from 15 to 55 KB (the three lean sets of the riding styles, the standstill's selects and the finger layers). The game reads and validates a
graph when a bike is loaded, as before; the per-frame cost is one more clip blend and a select.

## 8. Not done

The `Idle` machine (the rider fidgeting at a standstill: `race_trottle`, `race_check_tyres` and the other idle clips, picked by
`IdleAnim_Index` every 5 to 15 s), the leaning start clips (`*_start_*_leaning_L/R`: they follow the lean at the start), the starting grid and traffic
light states, the ground IK, and the female rider's clips (`race_F_*`).
