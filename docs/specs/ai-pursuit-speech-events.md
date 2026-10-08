# AI pursuit speech: the event tables and the speech tuning

The data behind the cop and dispatch radio chatter: the `speechtune` thresholds, the meaning of the 28 fields of
the `speech` class, the queue and validation rules that use them, and every one of the 133 event collections with the
values that matter. How the game decides *when to ask* for each event is in [ai-pursuit-speech.md](ai-pursuit-speech.md);
the sound files are in [formats/audio.md](../formats/audio.md) (section "Speech and NIS streams").

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src`: `EAXSound/Stream/SpeechManager.{hpp,cpp}`, `EAXSound/Stream/GameSpeech.{hpp,cpp}`,
  `EAXSound/SND_GEN/COPSPEECH.{hpp,cpp}` (the sentence-builder tables, names only), `Speech/EAXCharacter.h`,
  `Speech/SpeechCache.{h,cpp}`, `Generated/AttribSys/Classes/{speech,speechtune}.h`. Read for understanding; no code copied.
- **Data inputs:** AttribSys `speech` (28 fields, 133 collections) and `speechtune` (39 fields, 1 collection); the game's
  stream files (`SPEECH/copspeech.big`, `.idx`, `.evt`, `.csi`). Values are **[verified]** against the install.

Evidence tags as in the [docs README](../README.md#evidence-tags). Speeds in the tables are in **mph**, distances in
metres, times in seconds. Event ids are the 8-bit `SpeechID` of each collection.

## 1. `speechtune` (one collection, `default`) [verified]

| Field | Value | Used for |
|---|---|---|
| `PursuitInitDelay` | 2 s | wait after the pursuit starts before the opening line is chosen |
| `RangeForSpotterBranch` | 100 m | blow-by distance for the "spotter" opening |
| `SpeedDiffForBlowby` | 60 mph | speed difference that makes a blow-by |
| `BlowbyInterval` | 10 s | blow-by memory time |
| `MaxRangeForPrimaryBranch` | 20 m | (declared; not read in the sources) |
| `TimeWaitForSpotterReply` | 6 s | (declared) |
| `SuspectOutrunRange` | 120 m | pursuit distance at which "he's outrunning us" is said |
| `TimeConsideredLostNoLOS` | 3 s | added to the inactivity timers |
| `PursuitInactivityTimer` | 2 s, 150 s | (low, high) times without contact before the pursuit counts as lost, then abandoned |
| `NoLOSCommentaryTime` | 2 s | no-contact time before "lost visual" |
| `MinSpeedConsideredStopped` | 5 mph | speed below which the stop timer runs |
| `MinTimeConsideredStopped` | 2 s | stop time that selects the "player stopped" branch |
| `HighIntensityMark` | heat 2 | heat at or above which the chase is "high intensity" |
| `PursuitDurationHighIntensity` | 150 s | chase length that is high intensity |
| `MinPursuitDurationForBailouts` | 60 s | no cop bail-out lines before this chase length |
| `MinHealthForCommentary` | 0.33 | cop health below which a deactivated cop says the high bail-out |
| `SpeechDropoffRamp` | (30, 500) s | the playback probability falls from 1 at 30 s of chase to 0 at 500 s |
| `PriorityWeight` | 0.15 | (computed in the drop-out roll but not applied) |
| `CrashSlowdownPct` | 0.66 | speed loss fraction that counts as a crash |
| `PlayerSmashSpeedRange` | 10, 30 mph | closing speed range mapped to intensity 0..1 |
| `CollisionMinClosingVelSq` | 5 | (declared; the code uses 25 mph) |
| `MinIntensityTrafficSmash` | 0.5 | traffic hit intensity that counts for 911 and the semi/traffic lines |
| `MinIntensityCopSmash` | 0.85 | intensity at which a cop hit by the world bails |
| `MinIntensitySideswipe` | 0.5 | (declared) |
| `MinHavocForSuspectBehavior` | 200 | damage tally for the "suspect behaviour" line |
| `MinHeightAirborne`, `HangTimeForCommentary` | 0.5 m, 0.9 s | airborne line |
| `FlipTimeForCommentary` | 1 s | rolled-over line |
| `SpeedThreshFlyFlipIntensity` | 100 mph | normal vs high intensity for airborne/flip lines |
| `MinContigFramesFor180`, `MaxRangeFor180`, `MaxTimeFor180` | 12, 50 m, 5 s | u-turn detector |
| `OutcomeTrackTime`, `OutcomeFailSpeed` | 3 s, 20 mph | wait before commenting on a strategy's result; speed that means "failed" |
| `RBOutcomeTimer`, `RBPostOutcomeResetTime` | 1.5 s, 2 s | roadblock outcome |
| `BURemindTime` | 20 s | backup reminder |
| `AIRacerProximity` | 145 m | AI racers within this distance make the speech talk about "multiple suspects" |
| `CacheDisplayCoords` | (150, -155) | debug display |

## 2. Fields of the `speech` class [verified field list, meaning from the code]

| Field | Meaning |
|---|---|
| `SpeechID` | the event id (8-bit) used by the sentence tables |
| `CollectionName` | the event's name (the collection name, lower case) |
| `priority` | queue priority (higher plays first); an `interrupt` event gets +100 |
| `interrupt` | the event may cut in; it is validated at once and handled in the interrupt queue |
| `Interruptable` | whether a later interrupt may cut this event off while it plays |
| `Interval` | minimum seconds since the same event was last said (global history) |
| `expiry` | seconds a queued event may wait before it is dropped |
| `InitDelay` | the event waits this long after being queued before it is considered |
| `DeadAir` | minimum silence on the cop channel before it may start |
| `EnforceDeadAir` | silence the channel holds after the event ends (default 2 s; the next event cannot start before it) |
| `MaxPlayback` | maximum times per pursuit session (-1 = unlimited) |
| `MinPlayerSpeed`, `MaxPlayerSpeed` | player speed window (mph) |
| `MinHeat`, `MaxHeat` | integer heat window |
| `CullingRange` | the speaker must be nearer than this to the player (m) |
| `reqLOS`, `OnScreenOnly` | the speaker must see the player / be in the camera view |
| `DepFollow` (list of refs), `BackTime` | the event may only play right after one of the listed events (the last event said); with `BackTime > 0`, after one of them within `BackTime` seconds, or while it is still playing |
| `RecallList` (list of refs) | queued events in this list are removed when this one is queued |
| `DoNotDropout` | skip the random drop-out roll (section 3) |
| `Pan` | play with the speaker's azimuth instead of centred (used for the roadblock events and strategy calls) |
| `RadioChirp` | play a radio static "squelch" before and after |
| `Clarity` | 0 to 10, scaled to the mixer's clarity input (how clean the voice is) |
| `RedundancyCheckIsGlobal` | always true in the data: the history is shared between speakers |
| `cache_OnCreate`, `cache_SysInit` | preload the samples when the cop is created / at start-up |

## 3. Queue and validation rules (Speech::Manager)

Four lists are used: 0 incoming, 1 filtered, 2 ready to play, 3 interrupts. **[decomp]**

1. **Scheduling** (`ScheduleSpeech`): refused when speech is off, the speech volume is 0, the game is split screen, or the
   event pool is full. If an event with the same id is already queued, its data and speaker are refreshed instead
   (and nothing new is queued). Unless `DoNotDropout`, a roll is made: an event never said in the session always passes;
   otherwise it passes with probability `p = 1` for a chase shorter than 30 s, falling linearly to 0 at 500 s, 0 after.
   Queued events named by `RecallList` are removed. The event is queued with `priority` (+100 if `interrupt`).
2. **Deduce** (every 0.25 sim-tick fraction): each incoming event is pre-validated: events still waiting for samples or an
   `InitDelay` are deferred; interrupt events run the full validation at once and go to list 3 if they pass;
   everything else moves to list 1.
3. **Post-validation** (the same checks run again when an event is about to play and while its samples load): drop if it waited
   longer than `expiry` or if its `MaxPlayback` is exceeded; *defer* (keep waiting) if the silence since the last event is under
   `DeadAir`, if the same event was said less than `Interval` seconds ago, if the speaker is farther than `CullingRange`,
   if the `DepFollow` condition fails, if `OnScreenOnly` or `reqLOS` fail, if the heat or the player speed are outside the windows.
4. **Playing:** list 1 is sorted by priority; for each event the sentence function of the sentence tables is called with the
   event's parameter structure (section 4 of [ai-pursuit-speech.md](ai-pursuit-speech.md)) which chooses the sample sequence ("stitches"); the event moves to list 2
   when its samples are found. The next event to play is the highest-priority ready event whose samples have all loaded; one event
   plays at a time. An interrupt may start if the playing event is `Interruptable` (or if it is an interrupt of lower priority).
5. **History:** when 3 s of an event have played (or when it ends) its id, time, count and speaker bit are recorded in the global
   history (264 slots) and in a 10-entry "last said" list that `DepFollow` reads; the completion message `MNotifySpeechStatus` is
   sent when it ends, and the channel stays closed for `EnforceDeadAir`.
6. **Playback details** (`GameSpeech`): the speaker's volume is read from the dynamic-mixer output of that voice
   (dispatch 11, helicopter 10, primaries/secondaries 3 to 8, others 14; a low-pass is driven from output 12); `Clarity` goes to
   mixer input 3, pan to input 2; the **busted level** (0..1, `TimeUntilBusted`) goes to mixer input 5 so the radio muffles as
   the player is about to be arrested. `RadioChirp` events play a static stitch before (random of ids 4 to 9) and after (a 33 %
   chance of ids 0 to 3, otherwise 7 to 21).

## 4. The 133 collections

Columns: `prio` queue priority (before the +100 for interrupts), `interval`, `expiry`, speed window (blank = 0..550), heat
window (blank = 0..10), `cull` (blank = 300 m). Flags: **INT** interrupt; **noint** not interruptable; **nodrop** skips the
drop-out roll; **los** needs line of sight; **onscr** needs the speaker on screen; **deadN** needs N seconds of silence;
**holdN** holds the channel N seconds after (default 2); **delayN** `InitDelay`; **maxN** `MaxPlayback`; **depN/B** N events in
`DepFollow` with `BackTime` B; **recallN** N events in `RecallList`. The collection `default` is the parent of all rows;
`outcomes` and `acknowledge` are parents with a dependency list shared by their children.

**(top)**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| default | 0 | 50 | 20 | 5 |  |  |  |  |
| outcomes | 0 | 80 | 40 | 5 | 40-550 |  |  | noint dep8/12 |
| acknowledge | 57 | 50 | 60 | 15 |  |  |  | noint dep4/10 |
| cellcall | 204 | 100 | 0 | 55 |  |  |  | INT noint nodrop hold-1 |
| dispintrorace | 263 | 100 | 20 | 5 |  |  |  | INT noint nodrop hold-1 |

**setup**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| spotter | 58 | 90 | 0 | 5 | 30-550 |  | 1000 | nodrop |
| spotterreply | 59 | 90 | 0 | 10 | 30-550 |  |  | nodrop |
| initpursuit | 61 | 50 | 30 | 8 | 20-550 |  | 1000 | noint max1 |
| primaryengage | 62 | 50 | 60 | 5 |  | 0-2 | 150 | max1 |
| bullhorn | 64 | 60 | 40 | 5 | 10-40 |  | 25 | los |
| selfstrategy | 66 | 50 | 40 | 5 | 50-550 |  |  | nodrop |
| attmptvehstp | 154 | 90 | 0 | 9999 |  |  | 1000 | nodrop |
| vehiclereport | 155 | 99 | 20 | 10 |  |  | 1000 | noint nodrop hold1 dep4/60 |
| locationreport | 158 | 30 | 60 | 3 | 45-550 |  |  | nodrop hold1 |
| suspectconfirmed | 159 | 50 | 20 | 5 |  |  | 32000 | nodrop |
| reinitpursuit | 160 | 100 | 10 | 3 |  |  |  | INT noint nodrop hold1.5 delay0.5 |
| initialcallforbu | 161 | 50 | 20 | 5 |  |  |  | max1 dep5/30 |
| dispgoahead | 174 | 90 | 20 | 20 |  |  |  | nodrop hold1 dep1/0 |
| vehiclereporttag | 182 | 50 | 20 | 5 |  |  |  |  |
| initialcallforbu_ms | 205 | 50 | 20 | 5 |  |  |  | max1 dep5/30 |
| bullhornprefix | 215 | 60 | 90 | 2.5 | 10-100 |  | 0 | los hold1.5 |
| spotterwanted | 216 | 89 | 20 | 5 | 30-550 |  |  | nodrop |
| dispnovehdescrip | 225 | 50 | 20 | 20 |  |  |  | noint nodrop dep1/0 |
| dispcustpaint | 226 | 90 | 20 | 5 |  |  |  | noint nodrop |
| moredetails | 232 | 50 | 20 | 15 |  |  |  |  |
| dispvehdescrip | 257 | 50 | 20 | 15 |  |  |  | nodrop dep4/0 |
| dispvehdescripvinyls | 258 | 48 | 20 | 5 |  |  |  | noint nodrop |

**backup**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| callforbu | 67 | 50 | 60 | 10 | 30-550 |  |  | dep2/300 |
| unitbureply | 68 | 50 | 60 | 3 |  | 0-2 | 100 | max1 dep3/0 |
| bureminder | 73 | 50 | 60 | 5 |  |  |  |  |
| dispbackupupdate | 74 | 50 | 5 | 10 |  |  |  | noint nodrop dep5/0 |
| buarrives | 150 | 50 | 60 | 3 |  |  | 75 | los max1 |
| negativebureply | 162 | 50 | 60 | 20 |  |  |  | hold5 dep2/0 |
| dispbackupreply | 184 | 50 | 5 | 10 |  |  |  | dep4/0 |
| callforswarming | 221 | 50 | 60 | 10 | 30-550 |  |  | nodrop |
| dispbueta | 228 | 50 | 45 | 10 |  |  |  | nodrop dep5/0 |
| disphelibueta | 229 | 50 | 60 | 20 |  |  |  | nodrop dep1/0 |

**anytimeevents**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| disppursuitescalation | 70 | 90 | 60 | 10 | 30-550 |  |  | noint nodrop recall1 |
| collisionworld | 98 | 98 | 20 | 2 |  |  | 1000 | INT noint nodrop delay1 |
| collworld_spin | 98 | 90 | 300 | 3 |  |  |  | INT nodrop |
| disppursuitupdate | 99 | 50 | 300 | 5 |  |  |  | dead15 max2 |
| pursuitupdaterep | 100 | 50 | 15 | 10 |  |  | 32000 | dep1/0 |
| disp911report | 102 | 100 | 60 | 5 |  |  | 32000 | noint nodrop |
| unit911reply | 103 | 50 | 60 | 20 |  |  |  | nodrop dep6/45 |
| suspectuturn | 104 | 50 | 60 | 3 | 20-550 |  |  | noint |
| suspectoutrun | 105 | 75 | 60 | 1.8 | 50-550 |  | 500 |  |
| lostvisual | 106 | 92 | 40 | 6 |  |  | 1000 |  |
| regainvisual | 107 | 92 | 10 | 3 |  |  |  | INT noint nodrop hold1.5 delay0.5 dep4/30 |
| lostsuspect | 108 | 91 | 40 | 15 |  |  | 100000 | nodrop |
| callforev | 112 | 100 | 40 | 5 |  |  |  | noint nodrop dep4/200 |
| intenttoram | 113 | 86 | 30 | 1.5 |  |  | 100 | INT noint nodrop |
| bailout | 114 | 70 | 180 | 3 | 50-550 |  | 20 | noint los max1 |
| focuschange | 115 | 100 | 60 | 5 |  |  | 100 | noint nodrop hold3 max1 dep3/500 |
| suspectbehaviour | 116 | 50 | 20 | 5 | 45-550 | 3-10 |  |  |
| driverhistory | 118 | 50 | 20 | 5 |  |  | 32000 |  |
| unitdisabled | 164 | 80 | 30 | 6 |  |  | 1000 | noint nodrop |
| bailoutdeny | 165 | 50 | 60 | 10 | 50-550 |  | 100 | los max1 dep1/0 |
| offroadmoment | 166 | 79 | 30 | 3 | 10-550 |  | 800 | INT noint nodrop |
| disptimeexpired | 180 | 80 | 60 | 150 |  |  | 32000 | nodrop delay1 |
| disppursescgen | 181 | 90 | 60 | 10 | 30-550 |  |  | noint nodrop recall1 |
| dispbreakaway | 183 | 90 | 40 | 20 |  |  | 32000 | nodrop hold3 |
| regainvisualinterrupt | 200 | 70 | 20 | 3 |  |  | 600 | INT los |
| collworld_civi | 210 | 85 | 300 | 2 |  |  | 100 | nodrop delay0.5 |
| collworld_air | 212 | 80 | 300 | 3 | 20-550 |  | 100 | noint nodrop los delay0.5 |
| collworld_flip | 213 | 90 | 300 | 3 |  |  |  | INT nodrop delay1 |
| spotted | 214 | 90 | 10 | 5 | 30-550 |  |  | nodrop |
| suspectbrake | 217 | 60 | 60 | 5 |  |  |  | INT los delay1 |
| weatherreport | 219 | 50 | 60 | 30 | 30-550 |  | 1000 | max1 |
| heatjump | 220 | 87 | 20 | 20 |  |  | 10000 | noint nodrop |
| dispjurisshift | 223 | 82 | 1000 | 20 |  |  |  | noint nodrop dep3/10000 |
| dispevreply | 224 | 75 | 60 | 10 |  |  |  | nodrop dep1/10 |
| directionhigh | 233 | 50 | 40 | 2 | 50-550 | 2-10 |  | noint los dep1/200 |

**staticroadblock**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| callforrb | 76 | 50 | 60 | 2 | 30-550 |  | 32000 | los onscr hold0.5 dep7/300 |
| disprbreply | 77 | 50 | 30 | 8 | 35-550 |  |  | nodrop dep3/0 |
| disprbupdate | 78 | 65 | 20 | 8 | 35-550 |  |  | nodrop hold0.5 recall5 |
| rbapproach | 79 | 90 | 40 | 2.5 | 30-550 |  |  | nodrop hold0 dep4/30 recall1 |
| rbengage | 80 | 95 | 5 | 4 |  |  | 1000 | INT noint nodrop hold1 |
| pursuitapproaching | 119 | 64 | 40 | 2 | 40-550 |  |  | INT hold0.5 dep3/10 recall1 |
| rbreminder | 145 | 50 | 40 | 5 |  |  |  | dep1/60 |
| rbaverted | 152 | 95 | 10 | 2 | 10-550 |  | 1000 | noint nodrop los hold1 |
| negativerbreply | 163 | 50 | 30 | 20 |  |  |  | hold5 dep2/0 |
| callforrb_sub | 218 | 50 | 20 | 5 |  |  | 32000 | hold1 |
| dispsubrb | 227 | 50 | 20 | 5 |  |  |  | dep1/0 |

**rollingstrategy**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| initstrategy | 83 | 90 | 40 | 5 | 50-550 |  |  | nodrop |
| calltoposition | 84 | 75 | 60 | 3 | 35-550 |  | 20 | hold1 dep1/0 |
| calltopositionrem | 86 | 70 | 60 | 5 | 35-550 |  | 10 | dep1/10 |
| strategyexecute | 89 | 80 | 60 | 1 | 40-100 |  | 50 | noint los hold1.5 |

**outcome**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| anticipatesuccess | 90 | 70 | 40 | 5 | 40-40 |  |  | noint los dep8/12 |
| outcomefail | 93 | 80 | 20 | 5 | 40-550 |  |  | noint dep8/12 |
| strategyreset | 94 | 80 | 60 | 5 | 80-550 |  |  | hold1 dep8/12 |
| anticipatefail | 173 | 75 | 40 | 5 | 40-550 |  |  | noint nodrop dep8/12 |

**arrest**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| arrest | 95 | 80 | 20 | 2 | 0-10 |  | 10 | noint nodrop los hold1 |
| disparrestreply | 96 | 90 | 20 | 15 |  |  | 65000 | noint nodrop |
| bullhornarrest | 222 | 97 | 5 | 5 | 0-30 |  | 35 | INT noint nodrop |

**interrupts**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| interrupt | 153 | 80 | 20 | 2 |  |  |  | INT noint nodrop hold1 |
| staticinterrupt | 185 | 10 | 5 | 1.5 |  |  | 32000 | noint nodrop onscr hold1 |
| interruptram | 186 | 80 | 60 | 1 | 30-550 |  |  | INT noint nodrop onscr hold1 delay0.5 |
| interruptramhigh | 198 | 80 | 20 | 2 | 30-550 |  | 15 | INT noint nodrop onscr hold1 delay0.5 |
| interruptram_re | 206 | 80 | 20 | 2 | 30-550 |  |  | noint nodrop onscr hold1 delay0.5 |
| interruptram_ho | 207 | 80 | 20 | 2 | 30-550 |  |  | INT noint nodrop onscr hold1 delay0.5 |
| interruptram_ss | 208 | 80 | 20 | 2 | 30-550 |  |  | INT noint nodrop onscr hold1 |
| interruptram_tb | 209 | 80 | 20 | 2 | 30-550 |  |  | INT noint nodrop onscr hold1 delay0.5 |

**helispecific**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| helilostvisual | 169 | 50 | 60 | 5 |  |  |  |  |
| heliintenttobail | 170 | 50 | 60 | 5 |  |  | 0 | max0 |
| heliselfstrategy | 175 | 85 | 60 | 5 |  |  |  | nodrop |
| helibailout | 176 | 50 | 80 | 5 |  |  |  | max1 |
| heliswarming | 177 | 50 | 20 | 5 |  |  |  |  |
| helispotter | 178 | 50 | 60 | 5 |  |  |  | dep3/0 |
| helihazardalert | 179 | 50 | 20 | 5 |  |  | 0 | max0 |
| heliquadrent | 230 | 50 | 20 | 5 |  |  |  |  |
| heliquadrentmoving | 231 | 50 | 20 | 5 |  |  |  |  |
| helibullhornarrest | 259 | 97 | 5 | 5 | 0-30 |  | 35 | INT noint nodrop |

**extracops**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| swarmingreply | 171 | 50 | 20 | 15 |  |  |  | dep3/15 |
| superpursuitreply | 234 | 50 | 20 | 5 |  |  | 32000 |  |
| swarmingreplyfollow | 235 | 50 | 20 | 15 |  |  |  | dep1/10 |
| quadrentforming | 236 | 50 | 20 | 40 |  |  |  | nodrop hold3 delay1 dep2/0 |
| suspectpossiblygone | 237 | 50 | 20 | 90 |  |  | 3000 | hold3 dep2/0 |
| quadrentmoving | 238 | 50 | 20 | 90 |  |  | 3000 | hold3 dep2/0 |
| otherlead | 239 | 50 | 10 | 90 |  |  |  | hold3 dep1/0 |
| possiblesuspect | 240 | 50 | 10 | 30 |  |  |  | nodrop hold4 dep2/0 |
| wrongsuspect | 241 | 50 | 20 | 50 |  |  | 3000 | hold3 dep1/20 |
| rbwarning | 243 | 90 | 30 | 2 | 30-550 |  | 32000 | INT nodrop recall1 |
| rbposition | 244 | 91 | 20 | 2 | 40-550 |  | 3000 | nodrop recall1 |
| extrarbengage | 245 | 95 | 5 | 4 |  |  |  | INT noint nodrop hold1 |
| extrarbaverted | 246 | 95 | 10 | 2 | 20-550 |  | 1000 | noint nodrop los hold1 |

**cross**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| crossbureply | 247 | 100 | 20 | 15 |  |  |  | nodrop max1 |
| crossfailreply | 248 | 100 | 60 | 10 |  |  |  | nodrop hold4 dep2/0 |
| crossrbfailreply | 249 | 80 | 60 | 10 |  |  |  | nodrop hold4 dep4/0 |
| crosspursuitesc | 250 | 50 | 60 | 12 | 30-550 |  |  | nodrop |
| crossselfstrategy | 251 | 80 | 60 | 5 | 30-550 |  |  | nodrop |
| crossmultistrategy | 252 | 80 | 20 | 5 | 30-550 |  |  | nodrop |
| crossbailoutdeny_sub | 254 | 100 | 120 | 10 |  |  |  | nodrop hold4 max1 dep1/0 |

**d_day**

| Event (collection) | id | prio | interval | expiry | speed mph | heat | cull m | flags |
|---|---|---|---|---|---|---|---|---|
| day | 260 | 100 | 20 | 5 |  |  |  | INT noint nodrop hold-1 |

Notes on the table: two collections share id 98 (`collisionworld` and `collworld_spin`): the sentence tables pick the variant from
the event's parameters. The prefix of a collection name is the game's grouping of the sentence tables (`setup`, `backup`,
`anytimeevents`, `staticroadblock`, `rollingstrategy`, `outcome`, `arrest`, `interrupts`, `helispecific`, `extracops`, `cross`). The last
group is the collection `d_day` (the scripted "D-Day" race, priority 100, interrupt). The event `heatjump` (220) and
`dispjurisshift` (223) are the radio's reaction to the heat reaching 2, 3, 4 and 5 (speech spec section 6).

## Open questions

- The `.evt`, `.idx` and `.csi` layouts, and therefore how a sentence function plus its parameters pick the stitched samples,
  are not documented ([formats/audio.md](../formats/audio.md)); this file only fixes *which* event is requested and *when* it may play.
- Whether `PriorityWeight` and `CollisionMinClosingVelSq` have any effect on PC (declared, unused in the sources read).
