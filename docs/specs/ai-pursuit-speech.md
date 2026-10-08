# AI pursuit speech: when the cops and dispatch talk

How the original decides to put police-radio lines on the air during a chase: the object that watches the pursuit
(`SoundAI`), the flows that run the opening exchange, the tactics chatter, lost-contact and arrest, the observer that
reacts to crashes and stunts, the roadblock flow, the cop "actors" with voices and call signs, the 911 and heat
reactions, and the interactive pursuit music that the same object drives. The data (the `speech` and `speechtune`
tables, the queue and validation rules, every event) is in
[ai-pursuit-speech-events.md](ai-pursuit-speech-events.md). The pursuit being described is in
[ai-pursuit.md](ai-pursuit.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src/Speech/`: `SoundAI.{h,cpp}`, `PursuitFlow.{h,cpp}`, `StrategyFlow.{h,cpp}`, `RoadblockFlow.{h,cpp}`,
  `Observer.{h,cpp}`, `MusicFlow.{h,cpp}`, `EAXCharacter.h`, `EAXCop.cpp`, `EAXDispatch.cpp`, `EAXAirSupport.cpp` (skimmed),
  `SpeechCache.h`; `EAXSound/EAXSoundTypes.h` (speaker ids), `EAXSound/Stream/GameSpeech.cpp`; `AI/Common/AIVehiclePursuit.cpp`
  (`UpdateSiren`); `AI/Activities/AICopManager.cpp` (messages). Read for understanding; no code copied.
- **Data inputs:** AttribSys `speech`, `speechtune`, `pursuitlevels` (the siren fields, `TimeInactiveFor911`, `Lifetime911`,
  `CTSFor911`, `NumCiviHitsFor911`, `roadblock*chance`), `pvehicle` (`VerbalType`), `aivehicle` (`DetachmentID`); the sound
  files of [formats/audio.md](../formats/audio.md).

Evidence tags as in the [docs README](../README.md#evidence-tags). Everything not tagged is **[decomp]**. Speeds are mph (the
speech code works in mph), times seconds. The speech machine only *reads* the game; the one way it writes back is by
setting the 911 call (section 7) and by sending music events (section 9). It can be left out of a first pursuit
implementation without changing any chase behaviour, except that siren states, the 911 call and the pursuit music hang on it.

## 1. Structure

`SoundAI` is a Sim activity created by the cop manager (it is not created when sound is off or the game is split screen). Two tasks run:
the **main update** (0.1 of the sim rate) and the **observation update** (0.25 of the sim rate). **[decomp]**

```
main update (unless the busted flag is set):
    SyncPursuit()      which pursuit is the player's, the speech "pursuit state", lost/abandon timers
    SyncCarsToActors() one speech actor per cop car; LOS, ahead, closest distance, blow-by memory
    SyncPlayers()      player speed, position, road id, heading, heat, 911 counters, heat jumps
    SyncFormations()   which actors are in a formation
    ShuffleActors()    unique voices, choose the leader
    UpdateStateMachines()  one of PursuitFlow / StrategyFlow (+ roadblock) by "focus"; MusicFlow always
    if speech enabled and the channel has been silent: DealWithDeadAir()
observation update: Observer.Update (only while the pursuit state is Active), RoadblockFlow.Update, Manager::Deduce (the speech queue)
```

Sub-objects: `PursuitFlow`, `StrategyFlow`, `RoadblockFlow`, `Observer`, `MusicFlow` (all "speech flows" with a state number and a
busy counter), the `EAXDispatch` actor (speaker 1), cop actors `EAXCop`, and at most one helicopter actor `EAXAirSupport`
(speaker 2). Speaker ids: dispatch 1, helicopter 2, primary 3, 4, 5, secondary 6, 7, 8, cross 9. **[decomp]**

## 2. The speech pursuit state and the focus

`PursuitState` (not the same as `ePursuitStatus`):

| State | Condition (evaluated every main update) |
|---|---|
| Inactive | no pursuit for the player (or the speech has terminated); also after the bust |
| Active | the pursuit is not "re-acquiring", the target is in sight, status is not cool-down, and at least one cop car has line of sight (or the helicopter sees it), or a roadblock exists |
| Searching | no cop (nor the helicopter) has line of sight, or the pursuit is re-acquiring, or its status is cool-down |
| OtherTarget | the player has no pursuit but an AI racer's pursuit exists |

When the state becomes Active after not being Active, `TimeSinceLastChase` (time since the last time it was Active) is recorded. In
states Active and Searching the 911 counters are cleared. The state is tracked by watching `IPursuit::List`: the first pursuit whose
target is a player is the player pursuit, the first other is the AI one.

`focus` (the machine that is allowed to talk):

| Focus | Value | Meaning |
|---|---|---|
| Pursuit flow | 1 | the opening exchange of a chase (the initial state) |
| Strategy flow | 2 | normal chase commentary |
| Roadblock flow | 5 | declared, shares the strategy machine |
| Lost | 666 | contact lost; the "quadrant" sequence |
| Terminal | 999 | busted or abandoned; everything stops |

```
PursuitFlow --(its state reaches "transition")--> StrategyFlow(state 0)
StrategyFlow --(Inactive or Searching and no contact for PursuitInactivityTimer[0] + TimeConsideredLostNoLOS = 5 s,
                and not in an active cool-down with time left [cool-down needs time remaining])--> TerminatePursuit(outrun) --> Lost
Lost --(Active again within 150 + 3 s)--> PursuitFlow (reacquire: "re-initiate pursuit"; music reacquire; queue cleared)
Lost --(still searching/inactive; contact lost between 5 and 153 s)--> quadrant lines (section 6)
Lost --(cool-down finished: status EVADED or no pursuit)--> dispatch "time expired" once (flag), then
Lost --(no contact for more than 153 s)--> ResetPursuit (everything back to PursuitFlow, actors dropped)
any --(MPerpBusted)--> Terminal; music event 15; BUSTED flag stops the main update
TerminatePursuit(forced bail-out; game-mode driven) --> music terminal, queue cleared, "bail-out" line, Terminal
```

`ResetPursuit` is also called on a race restart and by the cop manager's `ResetCopsForRestart`.

## 3. Actors: voices, call signs and the leader

- Every cop car with the cop driver class that the pursuit lists gets an `EAXCop` actor when it first appears (`AddNewCop`), with a
  **voice** (speaker id) taken from a shuffled pool of ids 3 to 9 (one voice per actor, so at most six ordinary speaking cops plus the cross car; extra
  cars get no actor until a voice frees up), a **battalion** and a **call sign number**.
  Voice choice: roadblock cars take a secondary voice (6 to 8), the cross car takes voice 9, others the first primary or
  secondary left. If the pool is empty the new car adopts the identity of the actor that is farthest away (the actor's handle moves
  to the new car).
- **Battalion** (what the cop calls himself): by vehicle for the SUVs (`copsuv`, `copsuvl`, `copsuvpatrol`: Rhino Units 32) and the
  super-pursuit cars (`copsport`, `copsportghost`, `copsporthench`, `copcross`: Super Pursuit 16); otherwise by the road the cop is on (region
  city: City 4, coastal: Coastal 2, college town: Rosewood 1) or random among Rosewood, Coastal, City, Alpine (1, 2, 4, 8). Dispatch is 256, air
  support 64. Call sign numbers are drawn without repetition from shuffled pools per battalion (20 for City, 10 for Coastal, Rosewood, Alpine,
  6 for Rhino, 5 for Super Pursuit).
- **Primary / leader:** voices 3, 4, 5 (and heli 2, cross 9) are "primary". The leader (`mLeader`) is the first primary with line of
  sight when none is active; when the leader goes away another actor takes over, swapping voices with an existing primary, or becoming
  primary 1, 2 or 3 at random. When the leader is set during the strategy focus it says the "primary engage" line.
- **Per-actor state each update:** active, line of sight (the cop's `TimeSinceTargetSeen < 0.05`, or a roadblock cop that is in view), "ahead"
  of the player, in formation, in position, distance to the player, health, destroyed, airborne time (wheels on ground below a quarter), times
  rammed and last time rammed, traffic hits, time since the closing distance last shrank.
- An actor is dropped when the cop is un-spawned with a "remove" reason (restart, steal) and made inactive for the other reasons (message
  `MUnspawnCop`, parameter 1, 3, 4, 5, 6 deactivate; 0, 2 remove). When a cop **becomes inactive** (leaves) it may speak, once per 2 s:
  a cop that hit traffic twice: "bail-out traffic"; a helicopter: "bail-out"; a destroyed cop: "unit disabled" (by another cop with line of
  sight, or itself); health under 0.33: high bail-out; otherwise the low bail-out, followed by a "deny bail-out" from another primary.
  Bail-out lines need a chase length of at least 60 s. When a cop **becomes active** during the lost or strategy focus it says "spotted" or
  "regain visual" if it sees the player, or "backup arrives" / "unit backup reply" if not (not for roadblock cars).

## 4. Requesting a line

Game code never plays audio: it calls an actor method that fills the parameter structure of one sentence function and schedules it
(`Manager::ScheduleSpeech(parameters, sentence id, speaker)`). The structure names fix what the line says. The structures used (the sentence
tables are generated; the **parameter names** are facts): **[decomp]**

| Event family | Parameters |
|---|---|
| attempt vehicle stop | `pursuit_type` (generic speeder, possible wanted, reckless, hit and run, unit rammed), `num_suspects` (one / multiple), `speaker_battalion`, `speaker_call_sign_id` |
| vehicle report | `car_type` (the car's `pvehicle.VerbalType`), `car_color` (from the paint's speech colour), `speed` (steps of 20 from 100 to 300 in the player's unit; below 100: "over the speed limit"), `measurement` (imperial / metric / generic) |
| location report, 911 report, break-away | `direction` (N, S, E, W bits from the smoothed heading), `location_region`, `location` (from the road's speech id), `encounter` (first / subsequent), `address_group_type` |
| backup | `code` (10-code, usually none), `backup_type` (all units, air support, Rhino units), `disp_backup_eta` (15 s, 30 s, 1 min, 1:30, 2 min, over 2) |
| strategy | `self_strategy_type` (pit, self pit, side ram, ram, rolling roadblock, herding), strategy formation number |
| collisions | `intensity` (normal / high), `world_object_type`, `heli_hazard_alert_type` |

The player's heading is the dominant axis of the velocity smoothed over about 10 samples (0.9 / 0.1), and must hold for 3 s before it is reported.
The road id comes from the road network's nav (`GetRoadSpeechId`); the mapping from road ids to regions/locations is a generated table (116-entry
road-name list). The speech colour of the car and the car's `VerbalType` select the voice samples that describe it. If the game cannot place the
road (no speech id), location lines are skipped.

## 5. The opening exchange (`PursuitFlow`)

State machine (a state number and a `busy` counter; "speech busy" means the cop channel is playing or holding): **[decomp]**

```
CullCheck       wait until the speech pursuit state is Active                          -> CloseInCheck
CloseInCheck    Inactive or no actors: reset.  wait PursuitInitDelay (2 s) of chase.   choose the cause:
                  blow-by memory (distance < 100 m, speed diff >= 60, within 10 s, player >= 35 mph, cops not ahead) -> Spotted
                  911 active -> 911Reported;  pursuit race -> Scripted;  a cop rammed within 10 s -> CopAssaulted (unless scripted)
                  Spotted -> SpotterBranch;  CopAssaulted/Unknown -> PrimaryBranch;  911Reported -> WaitForSpotter;  Scripted -> ScriptedBranch
PrimaryBranch   closest cop (not heli) is made leader; if cause is Reacquired: "re-initiate pursuit"; else the leader says
                "attempt vehicle stop" (pursuit_type from the last infraction, section 8), dispatch "go ahead", then a "vehicle report" (not when
                cop assaulted); with 2 pursuits "suspect confirmed", with 3 "driver history".  Player stopped >= 2 s (and not assaulted) -> PlayerStopped.
                When the channel falls silent -> Terminal
SpotterBranch   high intensity or last infraction = resisting: the closest cop (heli allowed) says "spotter wanted", else "spotter"; maybe a vehicle report;
                dispatch "go ahead" or "acknowledge"; another cop "911 reply".  -> WaitForSpotter when silent and a cop sees the player,
                else a "lost visual" and LostWhileSpotterWait
WaitForSpotter  a cop with LOS becomes leader and says "spotted" / "spotter reply" (when it is not the first cop on scene or the cause was 911);
                dispatch escalation (with location when the heading is valid, else generic) unless 911.  -> Terminal when silent.  Stopped >= 2 s -> PlayerStopped
ScriptedBranch  dispatch escalation (+ vehicle description if the colour is known).  -> Terminal when silent
PlayerStopped   if an opening line was already said and the player has been stopped over 1 s or is under 10 mph: interrupt the leader's line; -> Terminal
Terminal        leader: "initiate pursuit" when the player is over 60 mph and the cause is not Spotted/911/Scripted/Reacquired/Assaulted;
                "location report" when not high intensity; a cop with LOS: "suspect behaviour" when the damage tally >= 200;  -> transition to StrategyFlow
Bailout         pursuit became Inactive: a random cop says the low bail-out
ChangeTarget    pursuit became OtherTarget: the closest cop says "focus change", dispatch escalation
```

A hit on a cop by the player during PursuitFlow (closing speed over 25 mph, deliberate) restarts the flow with cause CopAssaulted unless an
"assault" line has already been said; the cop that was hit calls `WasRammed`. **[decomp]**

## 6. Strategy flow, lost contact and dead air

### 6.1 `StrategyFlow` (focus 2)

```
CullCheck  Active or Searching -> SoloCheck
SoloCheck  needs a leader and a silent channel.  one active cop or no LOS: request backup (backup type 32) -> ReqBackup
           otherwise record formation type; fewer than two LOS cops or fewer than two in formation: leader says "self strategy"
           (heli: "heli self strategy"), SOLO flag; else leader says "initiate strategy" for the formation type.  -> Waiting
Waiting    (transitionable state) channel silent only.  Searching -> Outrun.  formation changed (and not follow) with >1 cop -> "strategy reset", SoloCheck.
           collapse/finisher active: PIT: half the time the closest cop says "intent to ram" else the leader "strategy execute", -> Outcome;
           BOX_IN / ROLLING_BLOCK (>=2 cops): leader "strategy execute" or call to position, -> Outcome; FOLLOW/HERD/STAGGER: closest cop "bullhorn".
           Active: contact formations (PIT, BOX_IN, ROLLING_BLOCK, HERD) with no cop in formation and >1 cop: CallToPos; cops in formation say "bullhorn";
           pursuit distance > SuspectOutrunRange (120 m) -> Outrun.  fewer cops in formation than before, or evade level > 0.25, or a single cop: backup request.
CallToPos  for every cop in formation but not in position: the leader says "call to position" (or the reminder if already said); that cop acknowledges.
ReqBackup  one cop (the first of the sort) says, by history: first time and not high intensity: "initial call for backup" + dispatch reply or ETA
           (ETA between 10 and 200 s); later: "call for backup"; then a "backup reminder" (>= 20 s after the last call of the same kind) with a dispatch update; when
           all were said: "call for swarming" with dispatch ETA and a swarming reply.  A denied request (MReqBackup "ReqDenied") gets a negative reply.
Outcome    when silent: the pursuit distance or speed grew: "anticipate fail" (if far / no LOS / not ahead) or "strategy reset"; else "anticipate success" (if not SOLO)
Outrun     one LOS cop and the cops are not ahead and distance >= 120 m: "suspect outrun"; no LOS cops for NoLOSCommentaryTime (2 s): the closest cop (or the helicopter) says "lost visual"
           (+ heli spotter and location) -> Lost
```

Backup events are triggered also from the game side: heavy support calls (`MReqBackup` types 16 = Rhino units, 64 = leader support, with "Request" or "ReqDenied"
messages), roadblock creation (`MReqRoadBlock`), and the heli flow. **[decomp]**

### 6.1b Roadblock flow (`RoadblockFlow`, run with the observations)

A bit set tracks the current roadblock: set up, seen, helicopter joined, averted, hit spikes / other object / vehicle, engaged, positioned. It runs only in the strategy or lost focus. **[decomp]**

- **Enabled flags.** When `roadblockprobability`, `roadblockhelichance` or `roadblockspikechance` of the current row change between zero and non-zero, a request line is
  made once the pursuit has said an "initiate strategy" or "self strategy" line: with roadblocks still disabled and none called yet, the leader says "call for roadblock" and dispatch
  replies *no* and the leader "negative roadblock reply"; when they become enabled (or spikes become enabled) the leader says "call for roadblock" (or the reminder if it was said)
  and dispatch replies *yes* (type spikes, multiple or generic); a helicopter roadblock adds a backup call for air support.
- **Set-up.** When a roadblock appears (`REQ_SERVICE`): if it is the second or later one, a secondary cop says "call for sub-roadblock" or dispatch "sub-roadblock reply";
  otherwise dispatch updates (no) and a secondary says "negative roadblock reply" (first state), or, once the cars are placed (message `MReqRoadBlock` "Position" with the
  spike-strip offset), the spike position comment (one strip) or the roadblock cop says "roadblock approach"; before the player sees it, half the time dispatch updates and a primary says
  "pursuit approaching", otherwise the roadblock warning plays. With line of sight to the roadblock (any roadblock car seeing the player or in view) a random secondary says "roadblock approach".
- **Outcome.** The player hitting spikes, another roadblock object or a roadblock car marks "engaged" (the message `TireBlown` marks spikes); a dodge (`MReqRoadBlock` "Dodged") marks "averted".
  After `RBOutcomeTimer` (1.5 s): engaged and not averted with LOS: a random cop says "arrest"; then a secondary says "roadblock engage" (with or without the spikes variant) or a generic engage
  line; averted without engagement: "roadblock averted" (from dispatch, a secondary, or a generic line; spikes always use the generic averted line) and a primary may call the
  next sub-roadblock. The flow resets after `RBPostOutcomeResetTime` (2 s) once the line was said.

### 6.2 Lost contact

In focus Lost the machine says, as `PerpLostTime` passes 5 s (and stays below 153 s) and not in the middle of a speech: one "quadrant forming"; then half the time "possible suspect" followed by
"wrong suspect", else (random of 3) "suspect possibly gone", "quadrant moving" or "other lead"; then it is expired. When the cool-down is complete (evaded) dispatch says
"time expired" once. When forced to terminate by bail-out: the "bail-out" line. On `TerminatePursuit` with no cop in view the last speaker says "lost suspect"; dispatch says "break away" with the last known
location and heading. **[decomp]**

### 6.3 Dead air (`DealWithDeadAir`)

While the pursuit is Active, a leader exists, the machine is not in the pursuit flow and the channel has been silent: after 60 s of chase dispatch asks "pursuit update" (at most twice, 15 s dead air
needed) and the leader answers with "pursuit update reply" or a "location report"; when the suspect has been out of contact for 2 s the leader (or helicopter) says "lost visual". **[decomp]**

## 7. Observations: crashes, stunts and the player's driving (`Observer`)

`Observer` runs only while the speech pursuit state is Active, skips when the focus is the pursuit flow and busted is near (< 0.5), and processes queued observations plus the assessments below. Collision
observations come from `SoundAI::OnCollision` (every vehicle in the chase is a listener): closing speed above 25 mph and an impact **intensity** =
`clamp((impulse - 10 mph) / (30 mph - 10 mph), 0, 1)` (from `PlayerSmashSpeedRange`). **[decomp]**

| Trigger | Condition | Line |
|---|---|---|
| player hits a cop | deliberate (the cop is not braking / not a roadblock cop / not an SUV; cop "immune" flag off); classified by relative heading and contact arms into rear-end, T-bone, head-on, side-swipe; speaker = the hit cop | intensity 0.15 to 0.5: normal-intensity line; 0.5 to 0.75: 67 % high-intensity line, else expletive interrupt; over 0.75: violent interrupt |
| cop hits traffic | | "cop-traffic" observation; during strategy the cop bails out (traffic) and another primary may deny |
| player hits world | speed loss over 66 % (`CrashSlowdownPct`) | "collision world" family: the closest LOS cop; variants for semi/tractor trailer, train, guardrail, spike belt, gas station explosion (set by the world event `expl_gas_station`; "call for EV" from cops within 50 m for 15 s) |
| player hits traffic | traffic hits with intensity >= 0.5 counted for 911 | "collision world civilian" |
| cop hits world | intensity >= 0.85 | the cop bails out (bad-road variant with weather) |
| airborne | altitude >= 0.5 m for 0.9 s, a cop has LOS | "collision world air" normal / high (speed >= 100 mph) |
| rolled over | up vector down for 1 s | "collision world flip" |
| u-turn (180) | heading dot road < 0.7, passes -0.7 within 12 contiguous samples and 50 m and 5 s | "suspect u-turn" |
| hard braking | brake or handbrake full, road alignment >= 0.7, speed > 60 mph, no crash/ram within 5 s, then a speed loss over 66 % | "suspect brake" |
| outrun | all LOS cops have not closed in for 8 s, 2 s in view, cops not ahead | "suspect outrun" |
| off-road moment | the SPAM map's speech id under the player changes while a cop sees the player for >= 2 s | "off-road moment" (first or subsequent time) |
| heli sees tunnel | message `TunnelUpdate` | heli hazard alert |
| arrest | `TimeUntilBusted > 0.8` and no arrest line is queued | clears the queue, the best cop (heli when closer than 100 m) says "bullhorn arrest"; later the "arrest" line; every update `TimeUntilBusted` is sent as the busted level to the mixer |
| outcome | 3 s after a strategy line, if busted < 1 | "anticipate success" or, if the player is over 20 mph, "outcome fail"; a deliberate "intent to ram" tracks the ram result |
| blow-by | the player passes a cop with a speed excess over 50 mph while at most one cop sees it (message `BlewByCop`) | "spotted" by that cop |
| regain line of sight | after a "lost visual", a cop with LOS | "regain visual" |

## 8. Infractions and the "pursuit type" wording, 911, heat jumps

- The wording of the opening line (`pursuit_type`) comes from the last infraction message (`MMiscSound`, channel "Infraction", the bit of
  [ai-pursuit-heat.md](ai-pursuit-heat.md) section 4): speeding / racing: generic speeder, or possible wanted when the chase is high intensity (heat >= 2 or longer than 150 s);
  reckless / off road: reckless; assault: unit rammed; hit and run / damage: hit and run; resisting: possible wanted. A cop rammed within 5 s overrides it with unit rammed;
  with no infraction the type is picked by a coin flip from traffic hits, havoc over 1000, or earlier lines said. `num_suspects` is "multiple" when AI racers are within 145 m.
- **911 (free roam):** with no pursuit active, the dispatcher says the 911 report when the damage cost seen by the speech (sum of cost to state of smacked objects) reaches
  `CTSFor911`, or hard traffic hits reach `NumCiviHitsFor911`, and cops are enabled; the report's `pursuit_type` is "possible wanted" in high intensity, else "generic speeder".
  When the report ends, `Force911State` sets `Lifetime911` seconds on the player's 911 timer and clears the cop lockout. "More details" and a "vehicle description"
  (colour, car type) follow with 50 %. A scripted version occurs during the final 20 s of the cop lockout in non-roaming, non-race play.
- **Heat jumps:** after 5 s of chase in the strategy focus, the heat rising across 2, 3, 4 or 5 makes dispatch (50 %) or a cop/heli say "heat jump" with the level; another coin flip adds a
  "jurisdiction shift" to state police at heat 3.x and federal at 5+. **[decomp]**

## 9. Interactive pursuit music (`MusicFlow`)

Runs only when interactive music is on and the music volume is above 0. It switches the PathFinder music graph between four pursuit sets ([music-graph.md](music-graph.md): sections 1 to 4) and
sends an intensity value each update. **[decomp]**

| Machine state | Music event sent | PathFinder part |
|---|---|---|
| Neutral | 0x0A | 4 |
| Lose (cops close) | 0x0B | 1 |
| Win (player pulling away) | 0x0C | 3 |
| Elude (player stopped/hidden) | 0x0D | 2 |
| Terminal | 0x0E | 5 to 7 end parts |
| start (0.5 s after "init") | 0x11 | |

Intensity (0 to 1, sent as 0 to 127): a slow filter `i = 0.99 i + 0.01 x target`; the targets and the state transitions depend on the pursuit distance averaged
(0.97 filter) against 32 m, the player's average speed against fractions of the car's top speed (12.5 %, 35 %, 45 %, 50 %, 80 %), crashes or cop hits within 2.5 s, collapse, and cool-down
(Neutral: close cops raise it to 1; Lose: collapse and stopped player at high intensity set 1; Win: speed above half top speed raises it, crashes drop it to 0.25, a 15 % boost every 10 s above 80 %;
Elude: movement above 10 mph). A restrained start keeps intensity at or below 0.5 and the state Neutral for the first 30 s unless the focus is the strategy flow. Reacquiring after lost contact
puts the state back to Lose (distance <= 50 m, intensity 0.5) or Neutral (0.8). A busted perp sends part event 15, an evade success during free roam sends event 14 (from the cop manager). The
names `MControlPathfinder(..., event, intensity)` are the messages used. The *licensed* music resumes about 40 s after the chase (music graph spec).

## 10. Sirens (`AIVehiclePursuit::UpdateSiren`)

Each cop car in a pursuit exposes a siren state used by the car sound (the siren AEMS bank): off, yelp, wail, scream, die. Rules (`pursuitlevels`: `SirenInitMinPeriod`, `SirenInitVariation`, `SirenWailPeriod`,
`SirenMaxYelpTime`, `SirenScreamPeriod`, `SirenMaxScreamTime`): during the opening exchange of the player's single pursuit the siren yelps for 0.5 s every `SirenInitMinPeriod + random(SirenInitVariation)`
seconds; when contact is lost the siren turns off; afterwards: for the first 6 s of a cop's chase yelp (or scream for the first 3 s at high intensity then yelp); then wail, switching to yelp for `SirenMaxYelpTime`
(6 s) every `SirenWailPeriod + SirenMaxYelpTime` (11 + 6 s); while a formation is collapsing, a finisher is running, or the re-initiate / spotted / bullhorn lines play, a cop in formation screams
(high intensity) for up to 3 s every 60 s, or yelps. A cop in a roadblock always wails; a cop whose lights are off is off, or "die" when destroyed. **[decomp]**

## Rust implementation notes

- Split into `pursuit` (game logic, no audio) and `pursuit_speech` (a resource that reads `PursuitView` and emits `SpeechRequest { event, params, speaker }`). The audio layer owns the queue of
  [ai-pursuit-speech-events.md](ai-pursuit-speech-events.md) section 3.
- Needed from the pursuit: status, `IsPerpInSight`, `GetEvadeLevel`, `TimeUntilBusted`, `IsCollapseActive`, `IsFinisherActive`, `GetFormationType`, `GetBackupETA`, `GetNumCopsDestroyed`, cops list with
  LOS flag, distance, formation flags, roadblock presence and cop membership, the 911 timer and lockout hooks. Needed from the world: road speech ids (not documented yet), the SPAM map for off-road moments,
  car colour and verbal type, the camera view test.
- Without the `.evt` / `.idx` / `.csi` decoders, a rewrite can log the requests or use the event name only; the siren state, the 911 call, the music state and the busted-mixer level can be implemented
  without speech samples.
- Voices and call signs: keep a seeded RNG; the shuffling is cosmetic.

## How to check it

(1) Start a chase at 70 mph past a patrol cop: the first line is "attempt vehicle stop" within a few seconds, the dispatch "go ahead" next; (2) stop the car: the player-stopped branch cuts the cop short;
(3) hit a cop at speed and watch the cause switch to "assaulted"; (4) hide for 5 s: "lost visual", then the quadrant lines, then "time expired" at the end of cool-down; (5) get arrested: "bullhorn arrest" while the
busted bar is above 0.8; (6) free roam: smash property to the 911 threshold with no cops around and confirm the call and the `Lifetime911` window.

## Open questions

- The `.evt` / `.idx` / `.csi` formats and the SPCH sentence selection (how parameters pick samples).
- The road speech id table (`MWRoadNames`) and the SPAM speech map for off-road moments: data layouts not documented.
- The `kNeverVisibleRespawnTime` and `DESTROY_COPS_ON_INACTIVITY` constants (build-time) and the PC value of `SpeechFlow` display flags.
- Behaviour of the helicopter actor lines (`EAXAirSupport.cpp`) beyond the event list of the events file.
