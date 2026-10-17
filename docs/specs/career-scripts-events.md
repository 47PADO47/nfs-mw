# Career scripts: race life cycle, prologue, rival gauntlets, final race, challenge series

Second half of [career-scripts.md](career-scripts.md): what the `basicrace` graph and its per-event copies, the
engage triggers, the prologue, the boss-race chains, the final race, the challenge ladder, speed traps and the OPM
mini game do. Sources, tags and the runtime model are in the first file. Collection and attribute names are the
game's; attributes whose names could not be recovered are given as `0x`hash.

## 1. The generic race graph (`basicrace`) **[verified]**

### 1.1 State order

```
initial -> tutorial -> intro_message -> (intromovie) -> intronis -> prerace -> (countdown) -> racing
racing -> raceover -> postrace -> outronis -> outro_message -> outromovie -> done
any state -> abandoned -> done          racing -> onlineover -> raceover   (online only)
```

- **initial.** Sets `CareerRace` from `IsCareerMode`. In career mode it asks for a time-of-day change; if the
  race's time of day differs it reloads the world and stops. The special-setup hook can also reload the world
  (forced player car, section 2.3). Then: cops on or off (`CopsInRace`), chance of rain, clear racers, set the race
  activity, create the racers at the start grid, count them, clear the busted flag, clear the race-status "has been
  won" byte, then `tutorial`.
- **tutorial.** D-Day races skip it; any other race calls `PlayTutorial` (native) and waits for a movie-finished
  message. What the native call does when no tutorial is due is **[unconfirmed]**.
- **intro_message.** If `IntroMessageID` is not -1 show that message screen first. Then choose the intro: a movie
  is played only when `IntroMovie` is set and no `IntroNIS` is set; otherwise the NIS state runs (it also covers
  "nothing to play"). In quick race a boss race uses `QuickRaceNIS` instead and never the movie.
- **intronis.** Plays the intro cutscene at the start marker (or warps the player to the start trigger when there
  is none or the game is split screen). On exit it saves the start positions.
- **prerace.** Resets cops, the per-racer tables (goal list, laps left, in-race flag), the knockout allowances, the
  checkpoint list (every checkpoint then the finish), the online flag, restores start positions and waits for the
  world to load.
- **countdown.** If `RollingStart` the first two racers are put under AI control; with `DoCountdown` the staging
  is held and the 3-2-1 screen runs; otherwise it goes straight to `racing`.
- **racing.** Section 1.2.
- **raceover / postrace.** `raceover` shows the race-over summary for `RaceOverMessageDuration` (3 s; 0.5 s
  online, 1 or 2 s after a destroyed car) and then calls the losing-screen hook (a stub in this build; the native
  summary offers the retry and quit choices **[unconfirmed]**) or goes to `postrace`. `postrace` calls
  the native post-race flow once (the result and reward screens) and sets the audio flag; the flow answers with
  `flowreadyforoutro` (to `outronis`), `restartrace` (to `prerace`) or `enterfreeroam` (cops back on, to `done`).
- **outronis / outro_message / outromovie.** Outro cutscene if both `OutroNIS` and a marker exist, an optional
  outro message screen, and the outro movie when set; then `done`.
- **done.** Calls the special-finalisation hook, disables the race's barriers and re-enables the current bin's
  barriers. If the race was abandoned it returns to roaming. In career mode, if `PostRaceActivity` is set that
  activity is run (the chained race, sections 2 and 3) and the game stays out of free roam until it ends;
  otherwise it returns to roaming.
- **abandoned.** Marks the race abandoned, stops racing, turns cops on, calls `AbandonRace` and goes to `done`.
  Copies for some drag races warp the player to an `abandon_respawn_marker` when roaming resumes.

### 1.2 `racing`

- **Checkpoint trigger.** For each racer: ignore if not in the race. A player entering a trigger flagged
  `ResetsPlayer` goes back to `prerace` (offline) or is warped to the start (online). For the racer's current goal
  checkpoint: report it, award the checkpoint's `TimeBonus` to the player (`AwardBonusTime`, HUD extension),
  advance the goal, tell the engine. With `SharedCheckpoints` the other racers aiming at the same checkpoint are
  advanced too. A trigger with a `TokenValue` pays that cash into the racer's in-race total.
- **Finish line.** Each crossing decrements laps left. At zero: `NumberFinished` + 1, rank = `CalculateRanking`, then
  `NotifyRaceFinished(racer)` (the native finish record, blacklist spec 4.2) and `NotifyRacePlacement(race, racer,
  rank)`, and the racer leaves the race.
- **End of the race for the human players.** When every human player is done: first place goes to `postrace`;
  anything else shows the losing screen through `raceover`. Time expiry, destroyed car, blown engine (after a 1 s
  timer), knock-out and being busted all end the race the same way (busted ends it elsewhere in the native flow).
- **Cash grab, sudden death, knock-out** per-kind copies are in section 5.

### 1.3 Kinds of race and what their copies add **[verified]**

| Kind (parent) | Copies and rule |
|---|---|
| `tollboothrace` | prerace: `TerminateRaceTime` 0 and the finish cinematic enabled (the time limit is the race's `TimeLimit`) |
| `speedtraprace` | prerace shows each speed-trap icon and clears the per-racer trap records; a trap pays only when the speed beats both the trap's `ThreshholdSpeed` and the racer's previous best at that trap; points are awarded through `AwardCash`, ranking is by points (`RankPlayersByPoints`) |
| `speedtrapjumprace` | one `SpeedTrapTrigger`; reaching `TargetBronze` km/h ends the race as finished |
| `cashgrab` | bags (`CashRewards`) with `CashReward`; each is taken once per race; sudden death ends the race on the first bag |
| `checkpointrace` | after the placement message, sudden death turns on; each second in sudden death costs every racer `OvertimePenaltyPerSec` points |
| `challengerace`, `milestonerace` | pursuit challenges: section 6 |
| `lapknockout` (part of `circuit`) | `NumAllowedForLaps` counts down by `KnockoutsPerLap` (never below 1) |

## 2. The prologue

### 2.1 Chain **[verified]**

The prologue races are not children of any bin. They chain through `PostRaceActivity` and all force the player car
preset `M3GTRCAREERSTART`:

| Step | Event | Collection | Intro | Notes |
|---|---|---|---|---|
| 1 | `16.1.0` | `16_1_0_partial_dday` | movie `storyfmv_rac01`, NIS `IntroNisDD` | one lap; an explosion effect is spawned on leaving `prerace` |
| 2 | `16.2.2` | `16_2_2_sprint` | movie `storyfmv_cro06_coh06a`, NIS `GenericStart` | |
| 3 | `16.2.3` | `16_2_3_sprint` | movie `storyfmv_raz08`, NIS `IntroNis05` | rewards: three out-of-jail-free markers; cops spawn at a trigger, bail trigger, 911 call trigger |
| 4 | `16.1.1` | `16_1_1_circuit` | NIS `IntroNis10` | two laps; pays the 10,000 cash value |
| 5 | `16.8.1` | `16_nis_rog_razor` | none | an NIS shell: plays `EndingNis03` (Razor wins the car); no racing |
| 6 | `16.2.1` | `16_2_1_d_day_intro` | movie `storyfmv_pin11`, NIS `IntroNis04`, outro movie `storyfmv_bus12` | the first chase; section 2.2 |

After step 6 the done copy calls `JumpToCarLot` (`Car_Select.fng`): the first real car is picked from the lot.
Then `race_bin_16` runs its entry activity `drive_to_safe_house` (section 7). Steps 2 to 4 pay a 10,000 cash value, steps 1 and 6 pay 0; the reputation attribute is 0 for all of them.

### 2.2 `16.2.1` scripted chase **[verified]**

The player triggers a `call_from_mia` marker: after 15 s the player car engine is sabotaged (`SabotageEngine`, level
5) unless the player reaches the `kill_engine` trigger first, which sabotages at once; a phone call (`ECellCall` 10)
is started on the call trigger. When the engine blows, a 1 s timer fades out and goes to `postrace`. A `cop_spawn`
trigger turns cops on and spawns two mid-size cops; a bail trigger calls `EDispIntroRace`, `EBailPursuit(1, 1)` and
blocks new pursuits.

### 2.3 Resuming a save in the prologue **[exe]**

The resume routine (`0x0057F570`) treats CurrentBin 16 specially: if race `16.2.1` is not yet completed it
restores the car preset and re-engages the saved prologue event, then starts the game flow; if `16.2.1` is
completed the normal front end is used. The special setup hook (`0x00605830`) forces the preset car for races that
ask for a fixed car unless career flag 20 is set.

## 3. Rival gauntlets (boss races) **[verified]**

A rival's `BossRaces` run back to back: each boss race (except the last) names the next one in `PostRaceActivity`.
Winning the last one grants the reward list that is attached to it (bin 1 has none: its last race is the final
pursuit).

| Bin | Order of boss races (event id) |
|---|---|
| 15 | `15.1.2` circuit, `15.1.1.r` circuit reversed |
| 14 | `14.2.4` sprint, `15.2.1.r` sprint reversed |
| 13 | `13.2.3` sprint, `14.2.2.r` sprint reversed |
| 12 | `12.2.1` sprint, `12.1.2` circuit |
| 11 | `11.1.2` circuit, `12.1.2.r` circuit reversed |
| 10 | `10.2.1` sprint, `10.7.3` drag |
| 9 | `9.5.3` speed-trap race, `9.2.1` sprint |
| 8 | `8.2.1` sprint, `8.7.5` drag |
| 7 | `7.2.2` p2p, `7.2.3` p2p |
| 6 | `6.1.1` circuit, `6.5.2` speed-trap race |
| 5 | `5.2.2` sprint, `5.5.4` speed trap, `5.2.1.r` sprint reversed |
| 4 | `4.5.2` speed trap, `4.7.4` drag, `4.5.1.r` speed trap reversed |
| 3 | `3.1.2.r` circuit, `3.2.2` sprint, `3.1.2` circuit |
| 2 | `3.1.1.r` circuit, `9.2.2.r` sprint, `2.2.1` sprint |
| 1 | `1.5.2` speed-trap race, `1.7.3` drag, `4.2.1` sprint, `1.1.2` circuit, `1.2.3` sprint, then `1.8.1` (section 4) |

The last race of each gauntlet carries the reward set: a pink slip (the rival's car; marker kind 0x12 adds it to the
garage), cash bonus (marker kind 0x13 adds the amount to the profile cash), an out-of-jail-free marker, an impound
release or extra impound box, and one or two part or visual rewards. Rewards are applied by the reward request at
`0x0062AA90`: a reward with an upgrade type unlocks that shop part; otherwise its marker kind sets an entry in a global
marker table that is saved in the profile (the 189-dword block after the race records). Counts in the install: 15
pink slips, 13 out-of-jail-free, 16 back-room unlocks, 8 impound boxes, 6 impound releases, 8 cash bonuses and
the part and visual rewards of every bin. **[exe]** (grant) **[verified]** (lists).

Every reward list sits on the last race of the gauntlet; the earlier races list none. Several bins borrow event ids
of other bins (`14.2.2.r` in bin 13, `15.2.1.r` in bin 14, `12.1.2.r` in bin 11, `3.1.1.r` and `9.2.2.r` in bin 2, `4.2.1`
in bin 1): the developers reused tracks, so an event id does not give the bin.

## 4. The final race `1.8.1` ("epic pursuit") **[verified]**

`race_bin_01/epic_pursuit` is a `challengerace` boss race (forced heat 6, rolling start at 40 speed, the car
`e3_demo_bmw`, a scripted cop force, intro movie `Storyfmv_her136`). Its copies:

- 10 s after the start the phone call 95 (`ECellCall`) is placed once; when more than 300 s have passed and the
  player is within 500 m of the barrier area, call 96 is placed, the `SCENERY_GROUP_DOOR` geometry opens and the
  player is guided to the barrier area (GPS). On entering the barrier area the icon and GPS are hidden.
- Win trigger: the player must hit it at 180 km/h or faster with a direction match of 0.8 or better. Then the race
  is won (`PlayerHasWon`), the NIS `EndingNis04` plays as the outro and the pursuit is bailed (`EBailPursuit(0, 1)`).
  After the NIS completes the generic copy calls `ChallengeComplete` and `NotifyRaceFinished`, and the race ends.
- Lose trigger (unless already won): detaches the camera and wrecks the racer.

The native post-race return then sets the career-completed flag and opens `GameWon.fng` (blacklist spec 7.1).
The resume routine forces the player into `1.8.1` on loading a career where race `1.2.3` is completed but the
career-completed flag is not set. **[exe]**

## 5. Engage triggers, gates and cell calls

### 5.1 Bin race engage (15 instances, one per bin) **[verified]**

`bin_race_engage` watches the bin's `engage` child (its `RaceTriggers`). `wait`: when the player enters a race
trigger, engage events are allowed, the target race is unlocked and not completed, remember the trigger and go to
`prompt`. `prompt`: show the engage prompt (`EnterEngagableTrigger`); on `AcceptEnterCareerEvent` run the target
activity and reset; on decline re-arm the prompt; leaving the trigger resets. A completed race therefore never shows
a prompt again; `UnlockRace` on a race whose icon was hidden shows it.

### 5.2 `world_engage` (4 instances, OPM only)

On initial: add the engage trigger to the minimap and to the available-event list, then `wait`. In `wait`, entering
the trigger with the target unlocked opens the prompt; accept runs the target; the exit trigger or a decline
resets.

### 5.3 Gates and cell engage

The `menu_gate` graph is in the first file (section 4). `cell_engage` (a phone call that rings until answered,
then starts its race) has no instance. A `cell_engage` waits for the player to place at most third in a gate
activity, rings for `RingTime`, and on accept runs the target; on decline it waits `DelayTime` and rings again.

## 6. Pursuit challenges and the challenge ladder

### 6.1 `challengerace` and `milestonerace` **[verified]**

`prerace` spawns the cops at `CopSpawnPoints` with `CopSpawnType`. A milestone message
(`MNotifyMilestoneProgress`, name and `ValueReached`) is compared with the race's `MilestoneName` and
`ThreshholdValue`: for "bigger is better" milestones the value must reach the threshold, otherwise it must be at
most the threshold. The first time it does, the HUD shows `HUD_CHALLENGE_COMPLETE` and the race remembers it.
The challenge is won only when the player then escapes the pursuit (`perpescaped`): `ChallengeComplete` and
`NotifyRaceFinished` run and the race ends; escaping without having reached the value, or being busted, ends it as
a loss. `milestonerace` copies keep the values in the instance table and print "Evade the cops!".

### 6.2 The ladder (bins 18 and 19) **[verified]**

Event ids `19.8.1` .. `19.8.30` are the first 30 pursuit challenges, `19.8.66` .. `19.8.69` the last four
(challenge numbers 31 to 34) and `19.9.70` the Collector's Edition challenge. Each lists in its unlock list the
next pursuit challenge and a tollbooth event. The tollbooth chain `19.8.32` .. `19.8.65` is a second ladder.
Initially unlocked: pursuit challenge 1 (`19.8.1`), tollbooth 1 (`19.8.32`) and challenge 35 (`19.9.70`).

| Challenge | Type | Threshold | Reputation attribute |
|---|---|---|---|
| 1 | pursuit length | 60 s | 5,000 |
| 2 | cops destroyed (milestone `bounty_in_pursuit`) | 1,000 | 5,000 |
| 3 / 9 / 16 / 24 / 26 | pursuit evasion time | 300 / 240 / 180 / 240 / 240 s | 5,000 to 500,000 |
| 4 / 14 / 23 / 27 / 33 | cost to state | 25,000 / 50,000 / 100,000 / 150,000 / 200,000 | 5,000 to 1,000,000 |
| 5 / 11 / 17 / 21 / 29 | roadblocks dodged | 2 / 6 / 8 / 6 / 10 | 5,000 to 500,000 |
| 6 / 13 / 18 / 32 | cops hit | 5 / 8 / 12 / 20 | 10,000 to 1,000,000 |
| 7 / 12 / 19 | infractions | 4 / 5 / 6 | 10,000 to 100,000 |
| 8 / 31 | spike strips dodged | 2 / 10 | 10,000, 1,000,000 |
| 10 / 20 / 30 / 34 | pursuit length | 420 / 600 / 900 / 1,800 s | 10,000 to 1,000,000 |
| 15 / 25 / 28 | cops destroyed | 50,000 / 100,000 / 1,000,000 | 50,000 to 500,000 |
| 22 | cops destroyed (milestone `tire_spikes_dodged`) | 6 | 500,000 |
| 35 | cops destroyed | 500,000 | 500,000 |

Challenge 34 (`19.8.69`) sets career flag 0x8000 when finished; when every race of bin 19 except `19.8.31` (the
Burger King tollbooth, bin 18) and the races flagged optional has been completed, flag 0x10000 is set.
The challenge cash value is 0; the reputation attribute's consumer is unknown. **[exe]** (flags) **[unconfirmed]**
(reputation).

## 7. Prologue drive, speed traps, milestones, OPM

### 7.1 `drive_to_target` (bin 16 entry) **[verified]**

`initial`: cops off; auto-GPS if set, else `driving`. The earliest message (when the career intro is not skipped) is
phone message 15. `driving`: timer `cellusage` (0.25 s) shows the GPS to the target (a second hint with a 2,000 m
offset), timer `hurry` (120 s) sends message 121; entering the target trigger goes to `at_target`. `at_target`
stops both timers and calls `DoSafeHouseIntro(RespawnMarker)` (CurrentBin 16, safe-house menu). The marker is
`menu_gate_safehouse/zone_safehouse_1/respawn`. On `done` cops come back and GPS goes off.

### 7.2 Speed traps **[verified]**

One instance (`speedtraps`) lists every speed trap of the game. When the player enters a trap that is currently
active and its `ThreshholdSpeed` is below the player's speed, the trap fires (`NotifySpeedTrapTriggered`) and
`AwardPlayerBounty(trap.Bounty)` is called; otherwise the HUD shows `HUD_TOO_SLOW`. The native challenge table then
marks the bin's trap entry as passed (flag 4). Per bin (thresholds in km/h, bounty per trap):

| Bin | Thresholds | Bounty | Bin | Thresholds | Bounty |
|---|---|---|---|---|---|
| 1 | 225, 241, 320 | 136,000 | 9 | 220, 225, 240 | 16,000 |
| 2 | 220, 266, 313 | 114,000 | 10 | 175, 185, 225 | 8,000 |
| 3 | 225, 255, 285 | 94,000 | 11 | 190, 190 | 6,000 |
| 4 | 245, 265, 290 | 76,000 | 12 | 185, 195 | 5,500 |
| 5 | 210, 290, 300 | 60,000 | 13 | 170, 175 | 5,000 |
| 6 | 173, 180, 255 | 46,000 | 14 | 145, 150 | 4,500 |
| 7 | 180, 230, 255 | 34,000 | 15 | 130 | 4,000 |
| 8 | 214, 238, 238 | 24,000 | | | |

The default speed trap has threshold 88 and bounty 100.

### 7.3 Milestones **[verified]** + **[exe]**

The 55 milestones are not script driven. The eight types and their counts: cops damaged 9, cops destroyed 9, pursuit
evasion time 8, pursuit length 8, cost to state 7, roadblocks dodged 6, tire spikes dodged 4, total infractions 4.
The bounty is the same for every milestone of a bin: bin 1 220,000, 2 180,000, 3 140,000, 4 100,000, 5 90,000, 6
80,000, 7 70,000, 8 60,000, 9 46,000, 10 35,000, 11 30,000, 12 19,000, 13 11,000, 14 6,000, 15 5,000. Each
milestone has three floats: a maximum target (`0x3B9BBFC2`), the starting target (`0x8445AF47`, stored in the save
as the threshold) and a step (`0x006EC903`, negative for evasion time). The saved fifth word is the best value reached
(a float). The use of the maximum and the step (escalating repeat levels, the `PostRace_MilestoneRewards` screen)
is **[unconfirmed]**; the saved thresholds of completed milestones still equal the start value.

### 7.4 OPM pursuit mini game **[verified]**

`race_bin_opm/20_2_1_rival_race/post_race_pursuit` is the only `pursuit_mini_game`: it spawns cops, forces a pursuit
at `ForceHeatLevel` (5) for `TimeLimit` seconds, wins by surviving or by escaping, shows "cops successfully evaded",
and ends with the post-pursuit screens; busted ends it. A test string "You beat the game!" is printed on the post
screen. The rival race `20.2.1` runs it afterwards (`PostRaceActivity`). None of this is reachable in a normal
career (spec: first file section 5, answer 2).
