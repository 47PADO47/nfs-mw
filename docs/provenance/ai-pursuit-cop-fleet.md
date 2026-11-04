# Provenance: the cop fleet (budget, queued requests, spawn spacing and walk, camera base, per-cop sight)

Date: 2026-10-21. Specs: [ai-pursuit-cops](../specs/ai-pursuit-cops.md) sections 1 to 5 and "Implemented in the rewrite".

The code was written from the spec. The behaviours below were learned by reading the public CC0 reconstruction
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) for understanding; nothing is copied or translated, and no
decompiler names appear in the repository.

| Behaviour | Read in | Notes |
|---|---|---|
| At most eight active cop cars (`mMaxActiveCopCars`, equal to the platform budget); the wave's cop request is served only below it; every police car counts (patrols, chase, support, roadblock, wrecks until recycled), the helicopter is counted apart | `AI/Activities/AICopManager.cpp` (`UpdatePursuits`, `IsCopRequestPending`, the constructor) | The rewrite uses the Bartender `MaxCops` (8 in the install) as the budget. Roadblock and support spawns have no budget check in the original either. |
| `UpdateCopPriorities(active + 1)`: only when the count after adding would exceed the budget, the active cop with the lowest priority is un-spawned; priority is the dot product of the unit vector from the player to the cop with the player's forward, plus one when the cop is in view; the first lowest wins | `AICopManager.cpp` | The spec's text says "cop-to-player"; the source subtracts the player's position from the cop's, so it is player-to-cop: cops behind the player and out of view go first. The only caller is `TrySpawnCop`. |
| `TrySpawnCop`: the queued request first makes room, then needs spawning enabled (no lockout) and a loaded car, and a spot clear of every simulated car by both radii plus 2 m, then starts a pursuit, a roadblock or a patrol; a failed request stays queued; while any request is queued the patrol pass wants no new patrol | `AICopManager.cpp` (`TrySpawnCop`, `UpdateSpawnRequests`, `UpdatePatrols`) | Roadblock requests are not built. A chase request without a chase is dropped (the original would start one). The rewrite has no script that queues requests; the console command `pursuit cop <car>` stands in. |
| The inactive pool (`GetAvailableCopVehicleByName`) hands back an un-spawned car of that name, or makes one; a car that is still loading is not handed out; `GetActiveCopVehicleFromOutOfView` takes an active cop out of view for over 5 s and over 100 m (the farthest) when none is free | `AICopManager.cpp` | Not reproduced as a pool of built cars: the rewrite shares each model's rig and builds a car from it cheaply, a retired car is dropped, and a model that has not loaded blocks the spawn exactly as a loading vehicle does. `blackbox_pursuit::fleet::steal` carries the steal rule but nothing calls it: the paths that use it in the original (the forced chase start and the roadblock builder) steal only when the pool has no free car, and the rewrite has no pool to run out of. |
| `CheckSpawnPosition`: the window and the spacing are measured from the camera (`GetBasePosition`); the lane test compares the lane index and the end the car heads for; same lane (or a wreck) 15 m, other lane 5 m, a car off the traffic lanes or a chase spawn 40 m, in 3D | `AI/Common/AISpawnManager.cpp` | The merging lane of a car (`toLaneInd`) is not kept. Applied to patrols and to the cool-down search cop; traffic keeps its own 20 m rule. |
| Chase spawn: the target's seek-ahead point is `min(500, speed * 7.8 + 8)` m along its cop road; the vector back to the target is turned 60 degrees to a random side; the cop walks that far along the cop roads from the seek-ahead point, steering by the turned vector; the spot must pass the window, the zones and the 40 m spacing and the cop is turned to face the target; otherwise 200 m ahead of the target plus a random point of a disc of radius 190 m is snapped to the cop roads and checked the same way | `AICopManager.cpp` (`SpawnPursuitIVehicle`), `AI/Common/AIVehicle.cpp` (`GetSeekAheadPosition`) | The "jerk" factor (0.4 of the seek time) is not modelled. The fallback spot faces the player in the rewrite; the original's does not turn the road cursor round. On a straight road the walk ends at the player, so the fallback does the placing. |
| Per-cop sight for the flee rule: a cop "can see you" when its time since it saw the target is zero | `AI/Common/AIPursuit.cpp` (`FleeCopOfType`) | The rewrite keeps each cop's answer of the last sight test (`TEST_PERIOD`). |
| `AIPursuit::SpikesHit` recruits up to three roadblock cars when none was recruited | `AI/Common/AIPursuit.cpp` | Nothing calls it in the sources; the PC executable's symbol list shows only the vtable slot. Not built. The cheat rule's recruit of one car is the roadblock director's. |

The camera of the rewrite is the simulation lens (the selected camera's retained shot on the physical body), also the
base of the recycle rule; the spawn window now uses its eye too. Not verified in a running original: any of the above as
a play experience, the 8-car budget on the PC build (the decompiled constants are the console build's), and how often the
original's cop requests hit the over-budget branch.

## Reinforcement timer joins and the wave size (2026-11-02)

[ai-pursuit-cops.md](../specs/ai-pursuit-cops.md) section 4. Read in the PC binary (behaviour only): the pursuit's attach
handler restarts the spawn timer for every attached vehicle (reinforcements, patrols, recruited roadblock cars, support
cars, the helicopter); the adjusted-count routine starts its upper bound at the integer limit and never reads the
manager's active-car limit. Rewrite: `Pursuit::joined` is the one join path; a chase with no initiating unit places its
first cop at once and that join starts the opening interval **[ours]**; `limits` drops the active-car cap; support cars do not count against the wave.
Headless checks: `pursuit::tests::spawn_timer`. The helicopter's request gate and weighted pick are changed on another
branch and not here.

## Reinforcement placement by cool-down status (2026-11-03)

[ai-pursuit-cops.md](../specs/ai-pursuit-cops.md) section 4.1. The reference chooses the cool-down placement by the
pursuit's status; a private recording of a stock-install pursuit agrees (a cop due between losing the player and the
cool-down came ahead on the chase placement). Rewrite: `TrafficWorld::update_pursuit` passes the cool-down status
instead of the sight flag.

## The forced start's first cop (2026-11-03)

[ai-pursuit-cops.md](../specs/ai-pursuit-cops.md) section 4.1. Read in the PC binary (behaviour only): the start of a
chase with no unit in it walks a road cursor from the target 340 m along its facing (80 m while either pending-jump flag
is set), reverses the direction after a jump, skips the window check then, and places the car there at rest. A private
recording of the stock game's Riverfront Stadium bounty start agrees: one cop already in the chase at the start, at
rest, about 65 m from the car on the road north of the junction ahead, facing the car. Rewrite: `pursuit/opening.rs`;
the F5 bounty jump marks the start (`TrafficWorld::set_jump_start`). Headless checks: `pursuit::tests::opening`, with an
installed-city check at Riverfront Stadium (10 m along the same road from the recorded spot).
