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
- **Implemented:** not yet.
- **Checked against the game by:** pending. The specs' "How to check it" sections list the in-game
  measurements (spawn distance and facing, per-type counts at each density, removal distance and time, the
  accident phase, honk timing).
- **Known differences from the original (intended for the rewrite):** none decided yet. Points where the original
  looks like a bug and a conscious choice is needed: the accident phase counts thinks, not seconds (0.3 s);
  the stop-sign and clear-intersection flags and pull over are dead code; the road-curvature speed limit can never bind;
  the quick-race "Low / Medium / High" levels map to 10 / 30 / 50 percent.
