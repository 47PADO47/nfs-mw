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

Open items are listed as "Next leads" in the spec. The main ones: the writer of the car's off-screen time, the car
class's vtable (not yet located), and the meaning of the detached-part predicate. The view object is now read
(second pass below).

### Second pass, 2026-10-16: view object, car slots, 4 s / 100 m rule

Same method and binary as above. Ghidra xref tools were used (`get_xrefs_to`, `get_function_call_graph`,
`disassemble_bytes` for undefined code). Spec section 8 carries the results and a Correction line.

| Address | Role | How found |
|---|---|---|
| `0x009885e0` | view object `V` (0x90 bytes). Created in `0x006fc3a0` (session init, reached through a table at `0x006595e0`), freed in `0x006fabb0`. Vtable `0x008b0600` (final), base `0x00890970`. Slot 0 `0x006f8cb0` (deleting dtor), slot 1 `0x006f0eb0` (task callback) | global xrefs; constructor `0x006f0c80`; teardown; vtable read at `0x008b0600` |
| `0x006f0eb0` | `V` slot 1: scheduler callback. Compares the task id with `V+0x44` (view update, calls `0x006ecf20`), `V+0x40`, `V+0x48`, `V+0x4c` (other tasks). Ids are the returns of the four task registrations in the constructor (`0x006ed0f0`) | vtable slot 1 bytes |
| `0x006ecf20` | **view update** (per rendered frame, task id `V+0x44`, rate 1.0). Copies the constant bank `0x008a2fc0` into `V+0x64..0x70`, then: `k` (`V+0x70`) = 1.0; `P0` = `(-y, z, x)` of the first entry of list `0x00919694` (entry object `-4`, subject at `+0x1c`, position `+0x40..+0x48`); `V+0x84` = that entry's slot-4 result `+0x94`. Same for `P1` from list `0x00919704` into `V+0x74..0x7c` (`V+0x80` = 1.0, `V+0x88` from its slot 4). If a list is empty the copy stays | callee of `0x006f0eb0`; FPU stack traced by hand |
| `0x006ecd90`, `0x006ed070` | scheduler step (node: rate, callback at `+4`, accumulator `+8`, flags `+0x18`, task id `+0x20`); task list head `0x009885d0` | callees of the view update path |
| `0x006ece90`, `0x006ed550` | distance `min(|p - P1|, k |p - P0|)` (3D, `sqrt` at `0x005c5e80`); returns `0x00890968` (0.0) when `V` is null | read in full |
| `0x008a2fc0` (16 bytes) | constant bank, zero in the install; read by the constructor and by `0x006ecf20`; no writer found | read_memory; xrefs (all READ) |
| `0x00410b00` | removal test, read in full (stack args at entry+4 and +8; locals reused): off-world `slot 0xc4` true → removable; else time `slot 0x60` > `T(d)` (`0x008eb36c`) and distance > `D(d)` (`0x008eb358`) → removable; then trailer (`[car+4]`, key `0x004040d0` via `0x005d59f0`, value `slot 1`, recursive) and parts (`car slot 1` → `obj slot 0x80` → `model slot 0x48` visitor `0x00404f90`, query vtable `0x00890c10`, flag `+4`; predicate `part slot 0x2c`) | disassembly |
| `0x00439bb0` | tick, read in full: density (`0x0041ec20`), timers at manager `+0x64..+0x88` (ten) and new-instance timer `+0x8c`, spawn `0x00439a10`, then the pool list at `+0x94`: active (`car slot 0x88`) cars go to `0x00410b00`; invalid ones: `car slot 0xac` → sub-object → `slot 0x8c` | disassembly |
| `0x00439a10` | spawn attempt: `new-instance timer > SpawnTime` (`0x00403af0`) passed to `0x00426380`; placement: sub-object `slot 0x58` with the probe cursor; loading = `car slot 0x5c` | disassembly |
| `0x00426380` | pool lookup (car slot `0x88`, `0x5c`, type key `0x50`); creates a car through the factory with class name `PVehicle` (`0x0089121c`) and hash `0xa6b47fac` when the allow flag is set | disassembly |
| `0x005e8960` | 4 s (`0x008a3724` = 4.0) and 100 m (`0x008a3720` = 100.0) test. Returns true (free) when inactive, not in the entry list, or in it and time > 4 s and distance > 100 m | disassembly |
| `0x005e8a20` | caller of `0x005e8960`; manager method (no Ghidra function at its start; `disassemble_bytes`); checks `[this+0x1960] == 1` and a global at `0x0092c67c` | disassembly |

Lessons (method):

- The global's xref list shows only absolute references. Writes through a loaded pointer are invisible to it. The
  writer was found through the object's own vtable: slot 1 is the scheduler callback, and it calls the update.
  Read the constructor's vtable pointer and then the slot functions.
- A zero constant bank read by a constructor and by the update is a default value. Check the update before
  reading a constructor's zeros as final values.
- FPU order: write the stack after each `FLD`/`FSTP`. The first reading of the view update was wrong (it gave
  `(-y, x, z)`); the `FXCH` order gives `(-y, z, x)`.
- Argument offsets: derive them from the pushes. In `0x00410b00` the stack slots of the two arguments are reused
  as locals (`T` and `D` are stored there), so a reading of `[esp+N]` must follow the push count.
- `search_instructions` with a common operand (`+ 0x64]`) scans about 877,000 instructions and returns 100 hits in
  unrelated classes. Use it only with a function filter.
- `disassemble_bytes` reads code Ghidra has not defined (the manager's and the view's vtable slots, the cop
  method at `0x005e8a20`). It does not create functions.

Dead ends:

- The 23 readers of the view global: all are constructors, getters (`0x006e8d80`, `0x006e8dc0` read the view's
  `+0x24` float or `0x00988dec`), the teardown, the scheduler and the view's own update path. None writes
  `+0x64..+0x80` except through the update in `0x006ecf20`.
- `0x006f0f80` toggles a flag at each object's `+0x3c` and walks lists; `0x006f6cf0` is the view's per-frame
  driver (timers at `0x009885d4`, `0x009885d8`; calls the scheduler step through `0x006ed070`). Neither writes
  the car's time.
- `0x00404f90` (the visitor) has one data reference, the query vtable `0x00890c10`. Its uses are not found outside
  the removal test.
- The car class's vtable is not located. The factory path in `0x00426380` builds the object; its constructor was
  not followed.
- As read, the update's slot-4 call on an empty list (`0x006ecfc1`, `0x006ed03b`) would dereference a null
  pointer. The original must keep both lists non-empty during play, or the code path is not reached. **[unconfirmed]**.

Next leads: spec section 8, "Next leads (removal, section 8)" (items 1 to 7).
