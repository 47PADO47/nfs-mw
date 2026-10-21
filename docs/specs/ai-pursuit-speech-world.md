# The world's messages to the chase radio

Which facts of the game the speech machine ([ai-pursuit-speech.md](ai-pursuit-speech.md)) is told, where the original learns them, what
thresholds apply, and what this port feeds. Behaviour read from the decompiled sources (CC0, dbalatoni13/nfsmw: `Speech/SoundAI.cpp`,
`Speech/Observer.cpp`, `Speech/EAXCop.cpp`, `AI/Activities/AICopManager.cpp`, `EAXSound/EAXAITunerCar.cpp`,
`EAXSound/sfxctl/SFXCTL_Tunnel.cpp`, `EAXSound/Stream/GameSpeech.cpp`), read for understanding; nothing copied.

Tags: **[decomp]** read in those sources; **[Ghidra]** checked in `speed.exe` (string references only: the program's symbols are stripped);
**[install]** measured on the installed files; **[inferred]** the port's own reading. Nobody has listened to any of it next to the original.

## 1. Collisions (`SoundAI::OnCollision`)

Every collision of a vehicle in the chase reaches the radio with: the closing velocity, the speed each body lost (`impulseA`, `impulseB`, metres
per second: the impulse over the body's mass) and, for two vehicles, the contact point in each car's own axes. **[decomp]**

- **Gate.** The hit is ignored when `LengthSquare(closingVel) < MPH2MPS(25)`. The comparison is on the *squared* length against 25 mph taken
  as metres per second (11.176), so what is dropped is every hit closing slower than 3.34 m/s (7.5 mph), not those under 25 mph. The earlier
  text of the spec (closing speed above 25 mph) was a misreading. **[decomp]**
- **Intensity** = `clamp((impulseA + impulseB - 10 mph) / (30 mph - 10 mph), 0, 1)` (`PlayerSmashSpeedRange`).
- **Crash** (the player against the static world): the speed after the hit over the speed before is under `1 - CrashSlowdownPct` (0.66 in the
  install, so under 34 percent of the speed is left). The same ratio stamps `mT_lastCrashed` for a hit against a traffic car or a prop, which
  the hard-braking remark reads. A hit on a cop does not stamp it.
- **Player and cop.** The classification reads the dot of the two forward vectors and the two contact arms (in each car's axes) against the half
  extents. Running the same way (dot 0.7 or more): abeam, both arms past three quarters of the half width, is a side swipe, and the player rams
  when the cop's speed along the contact normal is at most the player's; the player's front on the cop's rear (arms past three quarters of the
  half length) is a rear-end, and the player rams when it is the faster. Across (dot between -0.7 and 0.7): the player's front (arm 0.7 of the
  half length) on the cop's side is a T-bone. Head to head (-0.7 or less): head-on, and the player rams when its speed along the normal is the
  larger. A hit the cop caused is no ram. Where both noses are in, the original rams but has no wording (neither of the four): nothing is said at
  normal strength, an expletive or violent interrupt at high. One comparison reads `armRacer.z` against itself in the decompiled text; it leads
  only to a non-ram, so it changes no output. **[decomp]**
- **Deliberate.** A ram counts when the cop is not in a roadblock, not an SUV (`copsuv`, `copsuvl`, `copsuvpatrol`), not braking (brake or
  handbrake above 0) and the cops are not immune. The time of the last ram (`mT_lastCopNailed`) is stamped for every ram before those
  questions are asked, and also by the restart of the opening. **[decomp]**
- **Cop and traffic:** the cop's `JustHitTraffic` count and the observation `Collision_Cop_Traffic`; during the strategy flow with the pursuit
  active an active cop bails out for traffic and another primary may deny. **[decomp]** Two actors hitting each other are ignored.
- **Cop and the static world** (`DRIVER_COP` against the world): the cop must be on screen; at intensity 0.85 or more it bails out (the bad-road
  wording when it rains); a cop in formation may draw a denial from another primary; `Collision_Cop_World` is observed (and says nothing:
  the handler returns early). **[decomp]**
- **World object variants.** `Collision_Suspect_Guardrail`, `_Train` and `Behavior_Suspect_Spunout` exist in the observation table and have a
  handler each, but no code of `SoundAI` or `Observer` raises them: a crash is always `Collision_Suspect_World`, a tractor or trailer is the
  semi variant, spikes and the gas station have theirs. The guardrail, train and spin-out lines are therefore never asked for. **[decomp]**
- **Props.** A smackable the player hits adds its cost to state to the 911 counter while no pursuit is on (`Smashed`), and, if it is one of the
  roadblock's smackables, marks "hit object" (0x20) or, for the spike strip prop, "hit spikes" (0x10) with an observation. Hitting a roadblock
  car marks "hit vehicle" (0x40). The message `TireBlown` marks "hit spikes" and has the closest cop with sight remark on it. **[decomp]**

## 2. Messages

| Message | Sent by | Effect in the speech |
|---|---|---|
| `BlewByCop` | a cop car's drive-by sound, every frame the cop is within 12 m of the nearest player car closing at 15 m/s or more | during the strategy flow, pursuit active or searching, at most one cop with sight: if the player's speed is more than 50 mph over the cop's and the cop is no roadblock car, that cop says "spotted" **[decomp]**; the message string has one sender and one handler in `speed.exe` **[Ghidra]** |
| `TunnelUpdate` (1 / 0) | the player's tunnel sound controller on entering / leaving any enclosure (tunnel, overpass, small overpass, garage, dynamic zone with visit info other than 1) | the helicopter, if active and with sight, gives the "approaching tunnel" hazard alert on entering; the tunnel flag changes its "lost visual" wording **[decomp]** |
| `MomentStrm` with `expl_gas_station` | the gas station's sequence | during the strategy flow, if the player set it off: the closest cop with sight remarks on it (only while the pursuit is active) and for 15 s every cop within 50 m calls for an emergency vehicle **[decomp]** |
| `MReqRoadBlock(offset)` "Position" | the cop manager when a spike strip element is placed (once per strip) | the roadblock flow marks "positioned" and keeps the offset **[decomp]** |
| `MReqRoadBlock` "Dodged" | the cop manager, once per roadblock, when the player is past it | "averted" **[decomp, Ghidra: sender and handler]** |
| `MReqRoadBlock` "ReqHeliJoin" | **nobody**: the string has a single reference in `speed.exe`, the handler's registration | the roadblock flow's helicopter branch never runs **[Ghidra]** |
| `MReqBackup(16)` "Request" | the cop manager when the cars of a *ram* heavy support request are taken | Rhino units asked for **[decomp]** |
| `MReqBackup(16)` "ReqDenied" | the same request when its start fails, on every try | the same, denied **[decomp]** |
| `MReqBackup(64)` "Request" / "ReqDenied" | the cop manager when a leader support request starts / fails | all units asked for, dispatch answers with type 64 **[decomp]** |

The backup type 16 is the Rhino units, 64 the leader's (super pursuit) cars. Dispatch's "no" answer has a recording only for the type "no units"
(event 184: the only `yes = no` version of the sentence is the one for that type), so a denied request for type 16 or 64 finds no sentence and the
original says nothing for it either **[install]**; the cop's own "negative backup reply" follows.

## 3. Assessments from the player's state (`Observer`)

- **Airborne** (`NotifyAirborne(altitude, seconds)`): remarks when a cop has sight, the altitude is `MinHeightAirborne` (0.5 m) or more and the
  time `HangTimeForCommentary` (0.9 s) or more; high intensity at 100 mph or more. The decompiled sources have **no caller** of
  `NotifyAirborne`. The port calls it on landing with the jump's peak height above the ground and its length. **[inferred]**
- **Rolled over:** the car's up vector has a negative y for `FlipTimeForCommentary` (1 s) with a cop in sight. **[decomp]**
- **U-turn / hard braking:** read the dot of the car's forward vector and the *road's forward vector* at the car (the player's drive-to nav),
  which keeps its sense while the car turns; braking reads the pedals (full brake or handbrake), the speed over 60 mph and that no crash or ram
  happened for 5 s. **[decomp]** The port takes the road's direction from the nearest road segment's centre line, turned to the way the car
  first went and kept while the road keeps about the same line; on a road across the old one the car's heading chooses. **[inferred]**
- **Off-road moment:** the id under the car comes from the parameter-map layer "Speech" (the `ParameterAccessor` named "Speech", integer field 1
  of the set at the car's position). The installed track has the layer: 26 sets, ids -1 (the default) and 0 to 27 without 8, 15 and 18
  **[install]**. The table from that id to one of the 24 named places (`MiscSpeech::GetSPAMLocation`, a switch) was not available when this was
  written, so the layer is read by an installed test and not yet used.

## 4. What the port feeds

The world notes each fact as it happens in an event log carried by the speech feed (`audio/chase/events.rs`, recorded in
`scenes/world/ai/police/speech_events.rs` and `speech_hits.rs`); the radio turns them into the machine's notices with the speech tuning
(`audio/chase/radio/world_notices.rs`). The log keeps the last 24 events with a growing number; the radio takes those above the last number it
saw, and skips what happened before the chase began.

| Fact | Where the world knows it |
|---|---|
| player against a wall or barrier / a prop | the step's contacts of the player's car (`drive/sim/memo.rs`: the one contact it reacted to, its speed lost, its closing speed) |
| player against a cop or traffic, cop against traffic | the car pair response (`drive/sim/pair_contact.rs`, `ai/collide.rs`): both speeds lost, closing speed, contact arms |
| cop against a wall | the cop's own step contacts, read after the cars stepped |
| blown-by cop | the close passes of the collision step (12 m, 15 m/s) |
| jump | the wheels off the ground, the ground height under the car (a ray cast while in the air), told on landing |
| roll, brake, road direction | the player's state at the collision step; the nearest road segment |
| tunnel | the track's enclosure zones (kinds 3, 4, 5, 7, 10, with the elevation rule and the dynamic zone's visit info) |
| gas station | the breaker sequence of a model named `gasstation*` opening its cop zone; taken as set off by the player **[inferred]** |
| roadblock built / dodged / sawhorse hit / car hit / tires burst | `roadblock/build.rs` (the last spike slot's offset), `roadblock/director.rs`, the player's touched props, the pair response, `spikes.rs` |
| backup | `support/session.rs`: ram cars taken (Rhino), leader request placed (super pursuit), a refused placement (denied) |
| backup arrival time | `PursuitFeed::backup_eta` (the pursuit budget) |
| cop destroyed by a breaker | the cop is listed destroyed and inactive; the machine treats that as a wreck (`unitdisabled`) |

Not raised, and why: the free-roam 911 call's smashes (`Smashed`; the cop manager's 911 state is another piece of work), the helicopter joining a
roadblock (no sender), the guardrail, train and spin-out lines (no raiser), the off-road moment (id table), AI racers (`OtherTarget`,
`racers_near`: this world has no racers).

## 5. The radio's own sound (`GameSpeech`)

For the record, since the port does not play it: **[decomp]** a line's *clarity* (0 to 10) is sent to the speech object of the dynamic mixer as
`clarity / 10` in Q15 (input 3); the *pan* flag sets input 2, and the stream's azimuth is taken from the mixer's azimuth output 0; the volume of
the stream is the mixer's output for the speaker (speaker 1: output 11, 2: output 10, 3 to 8: outputs 3 to 8, any other: output 14) as a
percentage; the busted level (`TimeUntilBusted`, clamped to 0 to 1) goes to input 5 as Q15. The radio *chirp* is a static stitch (`AEMS_StichStatic`
bank), played before a line (stitch 4 to 9 at random) and after it (a third of the time one of 0 to 3, else one of 7 to 21). The installed map
`MAPOUTPUT.dyn` names the objects involved ("Busted Meter Scaler", "Speech Ducker", "Cop Voice Pos", "@Cop Speech", "@RADIO volume") **[install]**.
The speech object's number in the mixer map, its controllers and the stitch bank are not integrated, so none of it is applied: the speech sink
only plays the line at the speech volume.
