# RIDE 6 rider styles and the reverse walk

How RIDE 6 gives each kind of bike its own rider pose set, and how it walks a bike backwards, as far as the user's own game data
and binary say, and how this project plays both. Sources: the animation blueprint decompiled to pseudo-code by CUE4Parse (its
`SET Animations` and `Retro` functions), the data tables and physics assets dumped from the install, and the native input
handler of the game binary (`ride6-Win64-Shipping.exe`, build `5.6.1-1697004`; addresses below are RVAs). Nothing is copied: the
behaviour is written in words and equations. Provenance: [ride6-rider-styles](../provenance/ride6-rider-styles.md). Files:
[ride6-rider.md](../formats/ride6-rider.md). The pose itself: [ride6-rider-animation.md](ride6-rider-animation.md).

## 1. The bike types

The animation blueprint (`RiderGP_racing_Master_animBlueprint`) holds an integer `BikeType` and an integer `RidingStyle`. `SET
Animations` switches on them and assigns the clip variables the pose is built from. `BikeType`:

| `BikeType` | Kind | Clip folder | `EBikeTypeOverride` | Input handler class |
|---|---|---|---|---|
| 0 | race (sport and racing bikes) | `Animations/Race/` (`race_*`) | `RACING` | `Default`, `2502T` |
| 1 | naked | `Animations/naked/` (`naked_*`) | `STREET` | `Naked` |
| 2 | motard (supermoto) | `Animations/motard/` (`motard_*`) | `MOTARD` | `Motard` |
| 3 | scooter | `Animations/Scooter/` (`rider_scooter_*`) | `SCOOTER` | `Scooter`, `ScooterGear` |
| 4 | enduro and adventure | `Animations/enduro/` plus the motard clips | `ENDURO` | `Maxienduro` |
| 5 | bagger (cruiser) | the naked clips (`SET Animations` sends 1 and 5 to the same branch) | `BAGGER` | `Baggers` |

`RidingStyle` (0 to 2) picks the lean set inside a type. It is the rider's own setting (the riding style page; the data tables
`RIDING_STYLE` and `RidingStyle_DT` carry its leg, brake-finger and substyle numbers, not the clips): 

| `RidingStyle` | race | naked | motard | enduro |
|---|---|---|---|---|
| 0 | `race_Rossi_*` | `naked_Balanced_*` | `motard_Leaning_Knee_*` | the motard's |
| 1 | `race_Lorenzo_*` | `naked_ShoulderOut_*` | `motard_Leaning_*` | the motard's |
| 2 | `race_Schwantz_*` | `naked_Old_School_*` | `motard_Old_School_*` | `enduro_Old_School_*` |

### How a bike gets its type

The type is not in the rider's data. Two things in the bike's data name it:

- `RiderAnimationCorrection_DT` has a row per vehicle (364) whose `BikeType` is an `EBikeTypeOverride`: 338 rows say `NOOVERRIDE`,
  26 name a type (7 `RACING`, 6 `STREET`, 6 `ENDURO`, 3 `SCOOTER`, 3 `BAGGER`, 1 `MOTARD`). The override numbers are the blueprint's
  `BikeType` plus one (`RACING` 1 is 0, `STREET` 2 is 1, `MOTARD` 3 is 2, `SCOOTER` 4 is 3, `ENDURO` 5 is 4, `BAGGER` 6 is 5).
- Every bike's physics asset runs one input handler class (`Default`, `2502T`, `Naked`, `Motard`, `Maxienduro`, `Scooter`,
  `ScooterGear`, `Baggers`; of the 316 bikes with a physics asset: 196, 18, 72, 8, 15, 2, 2 and 3). The game's camera containers
  (`data/cameras/Containers/{Sport,Racing,naked,motard,MaxiEnduro,Scooter,Bagger}`) follow the handlers, except that six
  `Naked`-handler bikes (the race-modified editions of naked bikes) sit in the `Racing` folder.

**The rule used here**: the override if the row has one, else the handler's type. Of the 21 overrides whose bike has a physics asset
18 name the type its handler already gives (`STREET` on `Naked`, `ENDURO` on `Maxienduro`, `BAGGER` on `Baggers`, `SCOOTER` on
`Scooter*`, `RACING` on the two KTM RC 390); the other three move the bike: `RACING` on the naked MV Agusta RVS-1 and Triumph Speed
Triple 1200 RR, `ENDURO` on the Honda CRF 450R SMT (a `Motard` handler). The `STREET` row of the Suzuki GSX-S 1000 RM sits on a
bike whose camera folder is `Racing` while its handler is `Naked`, which hints that the engine's own default may follow the camera
folder (the five other race-modified naked bikes have no row). That the engine's variable follows the handler when there is no
override is an **inference**: the native code that writes `BikeType` into the blueprint was not read. The five race-modified naked
bikes without a row therefore play the naked set here and may play the race set in RIDE 6.

## 2. What each type plays

Every type builds the same tree, with its own clips ([ride6-rider-animation.md](ride6-rider-animation.md), section 5, for the tree):
three-way lean (left, centre, right) at low speed and at high speed, blended by the tuck; the sitting wheelie (minimum to maximum,
left to right); the stoppie, stepped; the standing wheelie (a blend space of the steering and the entry) where the type has it; the
drift poses where it has them; the reverse walk; the additive shake. The tables are `mods/ride6-bikes/src/rider/graph_sets.rs`:

| | race | naked | motard | scooter | enduro |
|---|---|---|---|---|---|
| lean centre, low / high | `race_standard_pose` / `race_aeroPose_Down` | `naked_low_standard_pose` / `naked_aeroPose_Down` | `motard_CenterPose_LowSpeed` / `_HighSpeed` | `rider_scooter_standard_pose` / `_aero_pose` | `enduro_standard_pose` / `enduro_aeroPose` |
| wheelie | `race_Wheelie_{L,R}_{min,max}` | `naked_wheelie_{min,max}_{L,R}` | `motard_wheelie_{min,max}_{L,R}` | `rider_scooter_wheelie_*` | the motard's |
| stoppie | `race_stoppie` | `naked_stoppie` | `motard_stoppie` | `rider_scooter_stoppie` | the motard's |
| standing wheelie | `race_wheelie_Standing_*` (ACL) | `naked_wheelie_Standing_*` (ACL) | `motard_wheelie_Standing_*` (ACL) | none | the motard's |
| drift poses | none | none | `motard_drift_pose_{L,R}` (ACL) | `rider_scooter_drift_pose_*` (ACL) | `enduro_drift_pose_*` (ACL) |
| reverse walk | `race_retro`, `race_retro_loop` | `naked_retro`, `naked_retro_loop` | `motard_retro_both_feet`, `motard_retro_loop_R` | `rider_scooter_retro`, `_retro_loop_R` | `enduro_retro_both_feet` (ACL), `enduro_retro_loop_R` (ACL) |

The Rossi, Lorenzo and Schwantz lean clips are the sport rider's styles; their bodies hang off the bike with the knee out, in the
clip itself. A sport bike therefore leans differently from a supermoto: it hangs the inner knee and the torso to the inside in
`race_Rossi_standard_*`, a supermoto sticks the inner leg forward in `motard_Leaning_Knee_*`.

**What the "knee" clips are**: `*_ass_knee_{L,R}_{Open,Closed}` are not knee poses. Only the two `gluteus_Vol` bones of the suit
(the buttock volume) differ between the open and the closed clip (17 degrees); they correct the skin of the suit as the knee goes
out. They are not played. The ground-trace corrections (`LeftKnee_Correction_Alpha`, the knee-to-ground distance of
`TraceKnee_*_DistanceMin/Max` 13 to -4 cm) and the leg-out layers (`*_leg_out_*_down_Rossi`, `_up`, braking into a corner with the
upcoming bend known from the track spline) are not played either; section 7.

## 3. How the project picks the set

`ride6-rider-import` writes one graph per type (`graph_race.json`, `graph_naked.json`, `graph_motard.json`, `graph_scooter.json`,
`graph_enduro.json`) next to the shared `graph.json` (the motard's, for a bike without a kind), and `styles.json` listing them with
the walk's numbers. `ride6-bike-import contacts` writes, for each bike, `contacts.json` with `style`: the type's name, the
blueprint's `BikeType`, where it came from (`correction-table` or `input-handler`) and the handler class. The game reads the bike's
`style.name`, looks it up in `styles.json` and plays that graph; a bike or a rig without them plays `graph.json`.

The riding style written is 0 for all (the clips of the first row of the table in section 1): a choice, as the setting is the
rider's and the data has no default for it. `ride6-rider-import --style 1` and `--style 2` write the other two rows into every
type's graph.

## 4. The reverse walk (animation)

The blueprint's `Retro` function is called once with `(0.8, 1.567, -1, -1, -20, 1, 4)`; its state machine has a `RetroStart` and a
`RetroLoop` state:

- The bike is "reversing" when `VehicleSpeed < RetroStart_Speed_Treshold` (-1) and the rider has not fallen. The speed is
  `VehicleSpeed`, whose unit is km/h (the same variable the lean thresholds of 120 to 150 are in).
- `RetroStart_Explicit` is `SmoothFloat_Constant(rule, 1)` of `Defuzzify([(reversing, 1.567)], 0.8)`: it moves from 0.8 to 1.567 (the
  start clip's seconds) at one clip second a second while reversing, and back to 0.8 otherwise. The state `RetroStart` plays the
  start clip (`Retro_Start_BothFeet`, or `_FootL`/`_FootR` by the leg that is down) at that time.
- When it reaches 1.567 the state becomes `RetroLoop`: the loop clip (`Retro_Loop_R` or `_L`) plays at the rate
  `RetroLoop_PlayRate`, which is `PlayRateMin` (1) at a speed of -1 and `PlayRateMax` (4) at -20, linearly between and held beyond
  (`FuzzifyRamp(VehicleSpeed, -1, -20)` of the two rates).

The motard's and the enduro's start clips are shorter than the window (0.367 and 0.4 s for the both-feet one, 0.733 s for the
single-foot ones; the race, naked and scooter clips are 1.567 s): the clip holds its last frame while the window runs.

Here (`crates/nfsmw/src/scenes/world/drive/rider/reverse.rs`): the graph's parameters `reverse` (the weight of the walk over the
riding pose, 0 to 1), `reverse_start` (the seconds), `reverse_loop` (0 while the start clip plays, 1 after) and `reverse_cycle` (the
loop clip's phase, wrapped by the clip's length) are made from the bike's signed speed with the numbers of `styles.json`. The
leg that is down is not modelled: the both-feet start clip and the `Retro_Loop_R` clip are played (choice; the blueprint's
`StandingLeg_Initialized` is a state of its start-up animations). The fade in and out of the walk takes a quarter of a second
(choice; the transition time of the state machine is in cooked data not read).

## 5. The reverse (physics)

The bike's input handler (`UBikeInputHandler`) pushes the bike back. The logic, from the functions at `0x50d8240` (the request),
`0x50d8620` and `0x50d8a80` (the force, one per handler variant), `0x50ff580` (the force on the body) and `0x50d96a0`
(`HasReverse`):

**Request.** The handler counts a lever as pressed above 0.1. Each step, with a reverse available, no throttle pressed, the main
brake pressed, the rear brake (the second brake axis) not pressed, a gear below first selected or the bike at a standstill (a speed
along the bike below 100, in cm/s), `RequestingReverse` is set. `ReverseTimer` counts the seconds it has been set, and is 0 when it is not.

**Force.** While the request stands and the timer is at most `ReverseTimeThreshold` (0.5 s) the brake is held (the lever input is
1). Past it the handler applies, at the body's centre of gravity, along the bike's forward axis and backwards, the force

```
F = BackwardForce * brake * (1 + min(0, v / BackwardMaxSpeed))        (v: speed along the bike, negative backwards)
```

then lets the brake go (lever input 0) and sets `IsReverse`. At a standstill `F` is the full force; it falls to 0 at
`-BackwardMaxSpeed`; faster backwards it pushes forwards. `HasReverse` is a test that a count in the vehicle state is above 2; the
count is read as the number of forward gears: a bike with at most two (a scooter, the vehicles with a continuously variable box)
has no reverse (an inference).

**Numbers.** `BackwardForce` and `BackwardMaxSpeed` are 200000 and 125 in all eight input handler assets (the native constructor
defaults are 100000 and 250; the data wins); `ReverseTimeThreshold` stays 0.5. The units are kg cm/s^2 and cm/s (the speed is
turned into km/h with 0.036 where the binary compares it): **2000 N and 1.25 m/s (4.5 km/h) for every bike**, held 0.5 s.

Here (`libs/blackbox-vehicle/src/bike/reverse.rs`, `[bike.reverse]` of the manifest): the same law with the numbers as parameters.
The request is the front lever above 0.1, throttle below 0.1 and a speed below 1 m/s (the speed test of the binary); the game's
single brake input pulls both levers (`drive/sim/bike.rs`), so the rear brake's "not pressed" condition cannot be told apart and
is left out: **holding the brake with the bike standing for half a second walks it backwards**. The brake levers are let go while
the rider pushes. No gear logic: the model's gearbox is automatic and the rider's push is a force on the body, so the engine
stays at idle. The bike keeps itself upright at walking pace as it does standing.

## 6. What is data and what is a choice

| Item | Origin |
|---|---|
| The clip names of each type and riding style | Data (`SET Animations`) |
| The type = override, else the handler | Data for the override; the handler rule is an inference (18 of the 21 overrides repeat it; the engine's own default may be the camera folder's kind for five race-modified naked bikes) |
| Riding style 0 for every bike | Choice (the setting is the rider's) |
| Aero centre pose = `AeroPose_Low` (`race_aeroPose_Down`, `naked_aeroPose_Down`) | Choice (the blueprint has two aero poses, low and high, chosen by a tuck variation function) |
| Enduro poses = `enduro_standard_pose`, `enduro_aeroPose` | Inference from the order of the assignments in `SET Animations` |
| Scooter lean set = the on-road one | Choice (the off-road set is chosen by `TrackCategoryAnimationModifier`) |
| Walk: window 0.8 to 1.567 s, threshold -1 km/h, rate 1 to 4 over -1 to -20 km/h | Data (the call `Retro(0.8, 1.567, -1, -1, -20, 1, 4)`) |
| Both feet down, the right loop clip | Choice |
| Walk fade of 0.25 s | Choice |
| Force 2000 N, limit 1.25 m/s, hold 0.5 s, the law above | Data and binary (`0x50d8620`) |
| Reverse from the single brake input | Choice |
| Reverse needs more than two forward gears | Inference |

## 7. Not done

- The ground-trace knee correction, the elbow trace and the `*_ass_knee_*` buttock-volume corrections (the clips differ in two
  bones of the suit only), the leg-out layers (they need the bend ahead), the bump and imbalance layers of every type, the
  look-back clips, the start and stop animations of the feet (the clips of every type are in the export but not in the pack's
  graph), the leg that is down (left, right or both) and the female rider's sets (`race_F_*`).
- The riding style as a setting: all bikes play style 0.
- The scooter's off-road lean set, the bagger's (type 5 is played as naked, as the blueprint does) and the endurance variants.
- `HasReverse` and the request conditions that depend on the gear and the rear brake.
