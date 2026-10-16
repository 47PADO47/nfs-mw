# AI traffic

Planned module: the traffic manager and the traffic driver of the AI crates (milestone 7). Engine-generic parts
(pool, spawner, lane follower interfaces) belong in `libs/`; the Most Wanted tuning (patterns, car list, constants)
comes from the data and per-game tables in `crates/`.

- **Specs:** [docs/specs/ai-traffic.md](../specs/ai-traffic.md) (cars and driving),
  [docs/specs/ai-traffic-spawning.md](../specs/ai-traffic-spawning.md) (density, patterns, spawn points, removal),
  [docs/specs/ai-traffic-world.md](../specs/ai-traffic-world.md) (lights and intersections, horns, scripted drag
  traffic). Related: [ai-road-network.md](../specs/ai-road-network.md), [ai-road-nav-trail.md](../specs/ai-road-nav-trail.md).
- **Sources read for the specs:** dbalatoni13/nfsmw (CC0-1.0, decompiled, GameCube build), `src/Speed/Indep/Src/`:
  `AI/Activities/AITrafficManager.cpp`, `AI/Activities/AvoidableManager.cpp`, `AI/Actions/AIActionTraffic.cpp`,
  `AI/Actions/AIActionRace.cpp` (the curvature speed function only), `AI/Common/AIVehicleTraffic.cpp`,
  `AI/Common/AIVehicle.cpp`, `AI/Common/AIGoal.cpp`, `AI/AIVehicle.h`, `AI/AISpawnManager.h` (not used by
  traffic), `AI/AIBasics.hpp`; `World/Common/WRoadNetwork.cpp` (`GetNextTraffic`, `CanTrafficSpawn`, `PullOver`,
  traffic lane helpers, `HolePunchAvoidables`, `UpdateOccludedPosition`), `World/WRoadNetwork.h`, `World/WRoadElem.h`,
  `World/TrackPath.{hpp,cpp}` (`FindZone`), `Misc/Table.{hpp,cpp}`, `Misc/Config.cpp`, `Misc/MWAttribUserTypes.h`;
  `Physics/Behaviors/{EngineTraffic,SuspensionTraffic,RBVehicle,RBTractor,DamageVehicle,DrawVehicle}.cpp`;
  `EAXSound/CARSFX/CARSFX_TrafficFX.{hpp,cpp}`, `EAXSound/EAXSOund.hpp`, `EAXSound/EAXTrafficCar.cpp`,
  `EAXSound/States/Registration.cpp`, `EAXSound/sfxctl/SFXCTL_Wheel.cpp`, `Physics/Behaviors/SoundCar.cpp`;
  `Sim/Activities/QuickGame.cpp`, `Frontend/Database/FEDatabase.cpp`, `Gameplay/{GRace,GRaceStatus,GRaceDatabase}.h`,
  `Interfaces/SimActivities/{ITrafficCenter,ITrafficMgr}.h`; `Generated/AttribSys/Classes/{trafficpattern,gameplay,
  pvehicle,aivehicle,world,smackable_hash}.h`; `Tools/Inc/ConversionUtil.hpp`, `Libs/Support/Utility/UMath.h`
  (unit helpers). Plus a search of the whole `src/Speed/Indep` tree for traffic-light, stop-sign, signal, horn and
  traffic-pattern code. Read for understanding only; no code, local names or tweak-variable names copied (the
  field and class names of the attribute data are facts).
- **Checked against the install** (throwaway Python scripts, not committed): the ten `trafficpattern` collections and
  their records; the `pvehicle` entries of all traffic cars (class, mass, behaviours, `aivehicle`, horn, engine and
  woosh ids, trailers); `aivehicle`, `collisionreactions`, `damagespecs`, `rigidbodyspecs`, `transmission`, `chassis`
  entries used by traffic; the `gameplay` fields `TrafficPattern`, `TrafficLevel`, `ForceTrafficDensity` and the
  197 drag-race spawn triggers; the `world` class leftovers; the 11 type-9 `TrackPathZones` of `TRACKS/L2RA.BUN` (order,
  patterns, overlaps); string searches for traffic-light and stop-sign model names in `STREAML2RA.BUN` and
  `L2RA.BUN`; class list of `attributes.bin` (no signal class).
- **Gaps in the sources:** the implementers of `ITrafficCenter`, `IVehicle::GetOffscreenTime` / `IsOffWorld` and
  `IVehicle::Count`, `GRaceStatus` / `GRaceDatabase` traffic-density code, `GTrigger` / `GCharacter` (empty),
  `AIParkedCarSpawner`, and the callers of `FlushAllTraffic` are absent from the decompilation. The spec marks
  those parts **[unconfirmed]** or lists them as open questions.
- **Date written:** 2026-10-09.
- **Implemented:** lane navigation, traffic spawning/recycling, driver controls and accident state;
  the full original traffic engine/suspension and vehicle pool are not implemented.
- **Checked against the game by:** pending. The specs' "How to check it" sections list the in-game
  measurements (spawn distance and facing, per-type counts at each density, removal distance and time, the
  accident phase, honk timing).
- **Known differences from the original:** the rewrite uses a deterministic 60 Hz world clock. Traffic thinks
  every ten physics ticks, the manager runs every 0.5 simulated seconds, and models are cached without
  an inactive vehicle pool or the original four-second new-instance creation limit. Traffic uses the full
  vehicle engine/suspension and may optionally obey the rewrite's traffic lights. Points where the original
  looks like a bug and a conscious choice is needed: the accident phase counts thinks, not seconds;
  the stop-sign and clear-intersection flags and pull over are dead code; the road-curvature speed limit can never bind;
  the quick-race "Low / Medium / High" levels map to 10 / 30 / 50 percent.

## PC scheduler check, 2026-10-12

Read-only static analysis of the original PC 1.3 executable, SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`, used the task
constructor (`0x006eccd0`), scheduler step (`0x006ecd90`) and traffic-manager registration
(`0x004418d0`). No original code was copied into the implementation.

The task's rate is a fractional accumulator increment. A variable task counts rendered frames and
passes accumulated elapsed frame time to its callback. The manager registers rate 0.5 and offset 0.5,
so its normal interval is approximately two rendered frames. The older interpretation of this as 0.5
seconds, and of traffic's rate 0.1 as ten Hz, was incorrect. The existing simulation-LOD spec had the
correct scheduler interpretation. Original task timing remains different from the rewrite's fixed clock.

The spawn clearance and accident follow-up uses the existing behavior specs. Synthetic regressions
cover a 160 m probe snapping to a road endpoint only 140 m away, planar clearance on elevated roads,
trailer spacing, tractor braking without full lock and cop drivers retaining driving controls after
an impact. Tractor classification comes from the inherited `BEHAVIOR_MECHANIC_RIGIDBODY` attribute,
not the model folder or whether a trailer is still attached. The following-distance regression uses
10 m/s and 10,000 kg: 32.375 m for an ordinary vehicle, 50 m for a tractor, without changing physical mass.
Accident camera-distance gating, non-player human/cop/racer impacts and restarting an ended accident
remain gaps. Original/remake gameplay comparisons of traffic paths, swerving, collision feel and
population timing remain pending.

## Sloped traffic startup, 2026-10-12

Installed highway probes isolate placement from driver input using real road
collision, a matching infinite slope and flat ground. Upright placement produced
initial body penetration, 1.08–1.41 m hitch error and roughly 700 kN rear-wheel
loads; trailers launched upward even while coasting. Ground-aligned placement
on the same plane removed the initial penetration and excess suspension travel.
The adapter preserves horizontal heading, uses tangent velocity and updates
contact history. This follows the observed remake fault and geometric support
requirements, not copied original code. It does not establish complete original
traffic suspension or turning behavior.

## Traffic removal rule, 2026-10-16

Read-only static analysis of the original PC 1.3 executable (`speed.exe`, same SHA-256 as above), in
Ghidra project `nfsmw`, program `/speed.exe`. Symbols are stripped, so every address below is a
Ghidra address (image base `0x00400000`). The rule is written in [specs/ai-traffic-spawning.md section 8](../specs/ai-traffic-spawning.md#8-removing-cars).

Found by this chain:

| Address | Role | How found |
|---|---|---|
| `0x004418d0` | traffic manager constructor; sets its vtables (`0x0089221c` primary, `0x00892214` at object offset 8) and registers its task with class key `AITrafficManager` (string `0x00892228`) | string xref by reading the vtable table at `0x00892200` |
| `0x006ee9b0`, `0x006ed0f0`, `0x006eccd0` | task registration: rate 0.5, offset 0.5, the callback object is manager offset 8, the task id is stored at manager offset `0x5c` | callees of the constructor |
| `0x006ecd90` | scheduler step: calls the callback object's vtable slot 1 with the task id and the frame time | task object layout (the step is the one in the 2026-10-12 record) |
| `0x00439cd0` | callback: vtable slot 1 of `0x00892214`; returns 1 and forwards to the tick only when the id matches the manager's stored id | bytes read at `0x00439cd0` (Ghidra had no function there) |
| `0x00439bb0` | manager tick (created as a function in the Ghidra project, which had none there): density, timers, one spawn attempt, then the removal loop over active pool cars | target of the callback |
| `0x00410b00` | removal validity test (recursive: trailer child, detached parts) | callee of the tick loop |
| `0x0041ec20` | density (passed to the validity test) | callee of the tick |
| `0x004307c0` | pattern selection and traffic-centre mean position (vector at `0x0092fe24`) | callee of the tick |
| `0x006533b0` | piecewise-linear table lookup used for the rate and the removal limits | callee of the tick and the validity test |
| table structs `0x008eb358` (distance) and `0x008eb36c` (time), data `0x00890564` and `0x00890590` | D(d) and T(d) values, 11 points, index = d * 10 | data read from the struct and the data arrays |
| `0x006ed550`, `0x006ece90` | distance `min(|p - P1|, k |p - P0|)` on global `DAT_009885e0` (3D) | callee of the validity test |
| `0x00404f90` and query vtable `0x00890c10` | visitor: sets the query's flag at +4 when a child matches the predicate; used for the detached-part keep test | callee of the validity test |
| `0x005d59f0` with key `0x004040d0` | attached child lookup (trailer role, unconfirmed) | callee of the validity test |
| `0x00406600` | stub that returns false (`xor al,al; ret 8`), a vtable slot of the manager | read at the vtable |

Other callers of the distance function: `0x00420040` (80 m test on a per-car record), `0x005e8960`
(4 s and 100 m test), `0x0066a200` (a per-car ratio), `0x0069c030` (a region code test with 0.8 margins; not
traffic-specific as far as read). Only `0x005e8960` is near the removal limits and it is a lead, not confirmed.

Dead ends and checks:

- No string contains `offscreen`, `despawn`, `visible` or `inview` (`search_strings` on the whole binary).
  The only traffic strings are the class and message names (`AITrafficManager`, `AIVehicleTraffic`,
  `SetTrafficSpeed`, `TRAFFIC_PATTERN`, `DRIVER_TYPE_TRAFFIC_CAR`, `TrafficLevel`).
- Name-based function searches (`find_functions` with patterns for offscreen, despawn, visibility, traffic) and
  class-member lists for `AITrafficManager` and `IVehicle` return nothing: the program has no class names.
- The cross-reference tools (`get_xrefs_to`, `get_function_call_graph`) were not callable in this session. Function
  callers were taken with `get_functions` and `fields=callers` instead; data references to a string or global
  were not searched.
- Two programs named `speed.exe` are open (`/New Traces/pydbg/speed.exe` and `/speed.exe`). Calls must pass
  `/speed.exe`.
- Vtable slots at `0x00433030` and `0x00433040` and the tick's neighbours have no function in the project; the
  manager vtable beyond slot 0 is mostly stubs or undefined.

Open items are listed as "Next leads" in the spec. The main ones: the identity of the view object
`DAT_009885e0` (it is also the scheduler object that the task registration uses, so the identity is not obvious),
the writer of the car's off-screen time (where any in-view test must be), the car class's vtable slots, and the
predicate of the detached-part visitor.
