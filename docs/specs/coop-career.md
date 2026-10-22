# Co-op career and free roam

Two players share one career save and one open world on one machine. Player 1 (the lead) and player 2 each drive a car,
each with a half of the screen and a HUD, and both play the same career: one garage, one wallet, one pursuit, one set of
events. The rules are written so that LAN players use the same rules later ([multiplayer/integration.md](multiplayer/integration.md),
[ADR 0009](../decisions/0009-multiplayer-netcode-revisions.md)).

- **Where the rules come from:** the original has no co-op career. Its two-player mode is the split-screen quick race only
  ([split-screen.md](split-screen.md)). So every rule here is **[ours]**, except where the text names a single-player rule it
  reuses. A reused rule keeps its own tag in the spec that owns it, and the text says which part is new.
- **Evidence tags:** as in [split-screen.md](split-screen.md). **[ours]** this rewrite's choice. **[exe]**, **[decomp]**,
  **[verified]** only where a reused single-player rule carries them.
- **Status:** built on the branch `feat/coop-career` (waves 0 to 8 of the plan); not yet checked in a long play session.
  Section 16 maps the rules to the code. Where the build settled a rule the design left open, the text says so.
- **Related:** [split-screen.md](split-screen.md) (halves, devices, seating), [career-flow.md](career-flow.md),
  [career-safehouse.md](career-safehouse.md), [ai-pursuit.md](ai-pursuit.md), [busted-sequence.md](busted-sequence.md),
  [busted-aftermath.md](busted-aftermath.md), [race-mode.md](race-mode.md), [gps-route.md](gps-route.md),
  [world-map.md](world-map.md), [career-phone.md](career-phone.md), [story-flow.md](story-flow.md),
  [multiplayer/features.md](multiplayer/features.md) (MP-07 and MP-54).

## 0. Summary

1. Co-op is a session state. Player 2 joins from the career menu, takes a second car from the shared garage, and leaves
   by "End co-op" or by going to the main menu. The save file holds no co-op state. **[ours]**
2. There are two players, `PlayerId(0)` (P1, the lead) and `PlayerId(1)` (P2). They share the career record, the garage,
   the wallet, the milestones and the phone.
3. There is one pursuit for both cars. Heat, cool-down, helicopter, roadblocks and support are shared. The busted meter,
   infractions, hiding and sighting are per car. Cops are split between the chased cars.
4. A bust takes only the busted car out of the chase. The chase goes on for the other car and ends busted when none is left.
5. A bounty is paid once. Each car keeps its own heat in its own car record.
6. Safe house, events and the Blacklist race take both players in. The rival race credits the best human place once.
7. Each player has a residency centre. Free roam loads the union of both areas, and a soft prompt offers to catch the
   trailing player up. No forced warp.
8. In the prologue there is no co-op. In story cutscenes P2 is parked and the screen shows one view.
9. The world has one clock: either player's Speedbreaker slows it for both, and each keeps their own meter. **[ours]**

## 1. Scope and status

- **In:** two local players on one machine, one career save, free roam, pursuit, the safe house, the Blacklist and other
  events, GPS, the map, the phone, streaming, traffic and the story after the prologue.
- **Out, for now:** online and LAN co-op (the same rules later, section 14). Human-driven cops (section 15). The
  split-screen quick race is unchanged; its spec is [split-screen.md](split-screen.md).
- **Prologue:** co-op is hidden until stage 16 is done (section 12).

## 2. Players and names

- **Ids.** `PlayerId(0)` is P1, the lead. `PlayerId(1)` is P2. The lead is the player whose car the single-player code path
  and the mod API's `player_car` point at. Single player is the same as a co-op session with P1 alone. **[ours]**
- **Names.** With an active alias `A`, the names are `A 1` (P1) and `A 2` (P2). With no alias, `Player 1` and `Player 2`.
  The alias is the one the alias manager holds ([career-parity-menus.md](career-parity-menus.md)). The names show on the
  race card, the pursuit and bust screens, and wherever a player is named. **[ours]**
- **One save.** The save file is the single career. Its format is unchanged. P2 has no profile of their own. **[ours]**
- **Session only.** The co-op state (P2's car, seat and name) lasts for the session. A saved file does not hold it: a
  load starts single player, and the player joins again to play co-op. Open question 2 covers keeping P2's car. **[ours]**

## 3. Entry and exit

### 3.1 Joining

1. **Career menu.** The "Co-op Career" icon sits on the career menu next to the single-player entries
   ([career-parity-menus.md](career-parity-menus.md)). It is hidden while the prologue (stage 16) is on. **[ours]**
2. **Press Start, twice.** "Player 1 press Start", then "Player 2 press Start" (the screens and labels of
   [split-screen.md](split-screen.md) section 2). Each player is seated on the device they pressed Start on, and keeps it
   for the session: a pad, the keyboard or the arrow block, in any pairing (pad + pad, pad + keyboard, the two keyboard
   blocks). P2 cannot take P1's device. Back on P2's screen returns to P1's; Back on P1's returns to the career menu with
   co-op off. **[ours]**
   - **Single player** seats nobody: the keyboard and the last pad used both drive P1, as before. The seats exist only
     while a co-op session has the world.
   - The console's `coop on <CAR> [arrows|pad]` has no Press Start: it puts P1 on the keyboard and P2 on the arrow block
     or the first pad connected.
3. **Second car.** The car select shows the shared garage of the loaded profile. Not offered: P1's current car, and any
   impounded car. With no other car free to drive, P2 shares P1's car (its record takes both players' bounty and heat),
   so co-op can start with a one-car garage. Accept seats P2's car, found as P1's is (the lot's name, a preset ride or
   the car type). Back returns to the career menu. **[ours]**
4. **Resume.** The game then resumes the career as [career-flow.md](career-flow.md) section 0 item 4 describes, with P2
   in the session. Resume Free Roam from the safe house menu starts the world for both. **[ours]**

### 3.2 Leaving

- **"End co-op"** is in the pause menu. P2's car leaves the world, P2's device is unseated, and P1 carries on
  alone. The save is not touched. **[ours]**
- **Refused** while P2 is in a race or is a pursuit target: the row is shown greyed out and cannot be chosen
  (`coop_end_locked`). **[ours]**
- **Main menu.** Going to the main menu ends co-op for the session. **[ours]**
- **Saving.** A manual save or an autosave while co-op is on writes the career as usual. P2's car is not written.
  [career-flow.md](career-flow.md) section 0 item 6 gives the autosave triggers. **[ours]**

## 4. Garage

- **One garage.** Both players buy, sell and pick cars from the same garage: the car lot and the safe house Car Select. The
  car list and the wallet are shared. **[ours]**
- **One car, one driver.** A car drives for one player at a time. A car in use by the other player cannot be sold or
  picked in the lot. **[ours]**
- **Cap.** The co-op cap is `Limits::for_players(2)`, which gives all 25 career record slots (`CAREER_SLOTS`). A true
  unlimited garage would change the save format, which stays as it is. The single-player cap is unchanged. **[ours]**

## 5. Pursuit

Reuses the single-player rules in [ai-pursuit.md](ai-pursuit.md) (tracking, sighting, busted meter, cool-down) and
[ai-pursuit-heat.md](ai-pursuit-heat.md) (heat). The two-car parts are **[ours]**. This amends MP-07 in
[multiplayer/features.md](multiplayer/features.md): MP-07 has heat and busted per player; here heat is shared and only the
busted meter is per car.

- **One chase.** One pursuit holds every chased car. A pursuit starts when a cop sees either car. **[ours]**
- **Shared** (one value for the chase): heat level, cool-down (evade meter), helicopter, roadblocks, support units, rep.
- **Per chased car:** busted meter, infractions, hiding state, last sighting, and impound strikes taken on that car.
- **Start heat.** The chase starts at the higher carried heat of the two cars. **[ours]**
- **Cops split.** `targets::assign` (balanced greedy): each chased car gets at most ceil(cops / targets) cops, and the
  nearest cop and car pairs are taken first. Re-run whenever the set of chased cars changes (a car is added, busted or
  leaves). **[ours]**
- **Cool-down.** The evade meter runs while no cop sees any chased car. Any cop seeing a chased car stops it. **[ours]**
- **Evaded.** When the evade meter fills, the chase ends for every chased car at once. Section 5.1 gives the bounty.
- **Busted.** Each car has its own busted meter, filled by the cops on that car. A full meter busts that car; the chase
  goes on for the others.
- **How the police run it (as built).** The police follow one car, the **primary**, through the single-player rules
  (sighting, infractions, the busted meter, the pursuit camera). Outside a chase the primary is the player nearest a cop,
  kept until another player is clearly nearer (20 %). In a chase it is the first chased car; when it is busted the next
  chased car takes over. The other chased cars are **secondary** targets:
  - a player joins a running chase as a target when a cop comes within 50 m;
  - a cop within 60 m sees a secondary target, which keeps the cool-down from running;
  - while two or more cars are chased the police's own busted meter holds and each target has its own meter (the
    single-player bust rule, with the 60 m sight for a secondary target);
  - the last chased car is busted through the police's own flow. **[ours]**

### 5.1 Evade

- The bounty is banked once into the shared career, credited to the lead evading car: P1 if P1 evaded, otherwise P2.
  Blacklist bounty progress therefore counts it once. **[ours]**
- Each other car that evaded gets evaded +1, and its unserved infractions are written to its own record. **[ours]**
- Each car's heat is written to its own car record. **[ours]**

### 5.2 Bust

- The busted car's half shows the arrest camera ([busted-sequence.md](busted-sequence.md) section 4). In co-op the arrest
  NIS scene does not play (it would freeze both halves) and there is no ticket screen: the ticket is paid in cash. **[ours]**
- The fine is paid from the shared wallet automatically. The impound strike goes on that car
  ([busted-aftermath.md](busted-aftermath.md)). Open question 5. **[ours]**
- The busted player respawns at the safe house (section 6). The chase goes on for the other car. **[ours]**
- The chase ends busted when no chased car is left. **[ours]**

## 6. Safe house

Reuses [career-safehouse.md](career-safehouse.md) section 3 (entering and engage). The two-car parts are **[ours]**.

- Either player's entry to the safe house zone starts the engage check. The check reads the shared pursuit, not one car.
- Both players are moved into the safe house.
- On exit P1 is placed at the exit marker and P2 behind P1.
- If the engage rules end the chase, they end it for every chased car.

## 7. Events and races

Reuses [race-mode.md](race-mode.md) and the Blacklist rules in [career-blacklist.md](career-blacklist.md). The two-human
field is **[ours]**.

- **Entry.** Either player's interact enters the event for both. Both are on the grid: P1 on slot 0, P2 on slot 1.
- **Seat.** P2 races in the human racer seat a split-screen race uses ([split-screen.md](split-screen.md) section 3).
  P2's free-roam car is parked for the race and comes back 12 m behind P1 when it ends. **[ours]**
- **Field.** A Blacklist race has P1, P2 and the rival; any other race has P1, P2 and its own opponents. Police follow
  the race's own rule (a split-screen race calls them off; a co-op race does not).
- **Credit.** The rival race's place credit is the best human place (`credited_place`), applied once.
- **DNF timer.** When one human finishes, the other has 30 s to finish on the race clock. At 30 s the other is
  knocked out (a DNF).
- **Neither finishes.** The race ends as a single-player DNF does ([race-mode.md](race-mode.md)).

## 8. GPS and map

Reuses [gps-route.md](gps-route.md) and [world-map.md](world-map.md). The two-car parts are **[ours]**.

- One route per player. Each player's arrow is drawn only in their own half.
- A map target set by either player applies to both.
- The map shows one mark per player, plus the target.

## 9. Phone, speed traps and milestones

Reuses [career-phone.md](career-phone.md) and [career-milestones.md](career-milestones.md). The sharing is **[ours]**.

- One SMS inbox for the career, shown full width in the popup.
- A speed trap is credited to the car that tripped it. The milestone stat it adds to is shared.
- Milestones and the wallet are shared.

## 10. Streaming and distance

- **Residency.** Each player has one residency centre around their car. The zone list loads the union of both. **[ours]**
- **Sim wait.** The simulation does not step any car until both areas are resident. **[ours]**
- **Memory.** Two areas can roughly double resident scenery. Residency bytes are logged. Open question 1. **[ours]**
- **Catch-up prompt.** When the two cars are more than 500 m apart, the non-anchor player gets a "Catch up" prompt. The
  anchor is the player in a pursuit or a race; if neither, or both, it is P1. Accepting warps the non-anchor player behind
  the anchor. The prompt shows in that player's HUD; pressing Reset accepts it (12 m behind the anchor, its heading). **[ours]**
- **No warp** while the non-anchor car is a chased car. **[ours]**
- **No forced warp.** Declining leaves the player where they are. The prompt returns once the cars are within 500 m again
  (open question 8). **[ours]**

## 11. Traffic

Reuses [ai-traffic-spawning.md](ai-traffic-spawning.md). The split is **[ours]**.

- Traffic spawns around each player's focus: the population manager spawns around each player in turn, so the car budget
  is shared out between them.
- A car despawns only when it is out of range and out of sight of every player. The cars steer around every player's car.
- Police spawns and respawns follow the primary (section 5).

## 12. Story

- **Prologue.** Co-op is hidden until the prologue is done. The prologue chain `16.1.0` to `16.2.1` is in
  [career-blacklist.md](career-blacklist.md) and [career-scripts-events.md](career-scripts-events.md). **[ours]**
- **Cutscenes.** During an NIS or a story movie, the scripted chase `16.2.1` and the final race, P2 is parked: hidden,
  with no physics, and not a pursuit target. The story races are P1's alone. The screen goes to one view, P1's. [nis-events.md](nis-events.md) gives the NIS timing. **[ours]**
- **After.** When the cutscene ends, P2 respawns behind P1, placed as in section 6. **[ours]**

## 13. Pause

- **Single machine.** Either player's Start pauses the session, as in [split-screen.md](split-screen.md) section 3. The
  pause menu has Resume, Restart, Quit and End co-op (section 3.2). Restart follows [race-restart.md](race-restart.md). **[ours]**
- **LAN.** The server never pauses (MP-54, [multiplayer/features.md](multiplayer/features.md)). A pause on a LAN session
  does not stop the sim. Whether LAN gets an "everyone waits" vote is open in
  [multiplayer/features.md](multiplayer/features.md) (its open question 5). **[ours]**

## 14. LAN mapping

What changes when a seat is remote. The rules above do not change.

- A seat is `Seat::Remote{peer}` instead of `Seat::Local{slot}`. Its inputs come from the network, not from a device.
- The server runs the same pursuit, events, streaming and traffic. The clients draw.
- There is no seating screen on LAN. The lobby seats players, and a remote player's car select is the lobby's.
- Pause is off (section 13). Co-op is joined in the lobby, not from the career menu.
- The step that feeds each player's controls is `Inputs` per `PlayerId` ([multiplayer/integration.md](multiplayer/integration.md)
  section 4). **[ours]**

## 15. Open questions

1. **Forced warp.** If memory runs out with two resident areas, the fallback is a forced warp past about 1500 m. The distance
   and the choice are the owner's. The byte cost of two areas is logged, not measured yet.
2. **Persisting P2's car.** A settings key `coop_second_car` could keep P2's car across sessions. Not in this design.
3. **Mod API.** `player_car` points at P1. A `players()` call for mods is a follow-up.
4. **Human cops.** A player driving a cop car is not in this design.
5. **Short wallet at a bust.** The fine is paid from the shared wallet. What happens when the wallet is short is not
   settled here.
6. **End co-op in a race or chase.** Refused (section 3.2): the row is greyed out. Whether the menu should also say why
   is open.
7. **Other events.** The Blacklist field is set. Other events with a rival (and any event with a fixed field) need their
   own rule.
8. **Catch-up after a declined prompt.** This design re-offers the prompt only after the players close to within 500 m.
   The owner may want a longer wait.

## 16. Code map

| Rule | Code |
|---|---|
| Ids, seats, roster | `libs/blackbox-net-core/src/session/party.rs` |
| Pursuit targets and the cop split | `libs/blackbox-pursuit/src/targets.rs` |
| Names, race credit, garage cap, per-car settlement | `crates/nfsmw-career/src/coop/`, `carlot/mod.rs` (`Limits::for_players`) |
| The players list, input per player, parking | `crates/nfsmw/src/scenes/world/players/` (`mod.rs`, `inputs.rs`) |
| Session, join, end, console `coop` | `players/coop.rs`, `players/spawn.rs`, `players/command.rs` |
| Guests' frame work, contacts | `players/step.rs`, `players/contact.rs`, `ai/collide_guests.rs` |
| The chase: primary, targets, meters | `players/chase.rs`, `ai/coop_chase.rs` |
| Busts and pursuit end in the career | `career/coop_bust.rs`, `career/pursuit.rs`, `career/respawn.rs` |
| Streaming centres | `focus.rs` (`foci`), `residency.rs` (`update_centres`), `zone.rs` (`update_many`) |
| Traffic | `ai/population/guests.rs`, `ai/scene.rs` |
| Catch-up | `players/tether.rs`, `players/catch_up.rs` |
| Races | `players/race.rs`, `race/start.rs`, `race/coop_finish.rs` |
| Story | `players/suspend.rs` |
| Halves and HUD | `split/view.rs`, `players/view.rs`, `hud_pursuit.rs`, `hud/split/pursuit.rs` |
| Front end | `frontend/flow_coop.rs`, career menu, career car select, pause menu |

## How to check it

- Offscreen, never with window focus: `view-world --drive --exec "coop on <CAR> arrows" --screenshot coop.png --screenshot-count 4`.
- A pursuit run in which one car is busted while the chase goes on for the other.
- A `--ui-script` career path: Co-op Career, Press Start, car select, resume.
- Residency bytes logged for two centres, and a `--bench-seconds` run with both halves at 30 Hz or above.
- Measured 2026-10-22, release build, `--bench-seconds 30 --no-vsync --offscreen` (the window stays hidden and
  unfocused), RTX 4070 SUPER, blackbox renderer, free roam at the default start:

  | Run | fps | p50 ms | p95 ms | p99 ms |
  |---|---|---|---|---|
  | Single player | 241.4 | 4.08 | 4.42 | 4.52 |
  | Co-op (`coop on 911TURBO arrows`) | 203.1 | 4.74 | 5.62 | 5.83 |
  | Control: `coop on` with a car that does not exist | 244.6 | 4.04 | 4.26 | 4.57 |

  Each half is captured every other frame, so each half refreshes at about 100 Hz in co-op: well above the 30 Hz the plan
  set as the bar for a second capture slot, which is therefore not needed.
- The career-menu path was run offscreen with a scratch profile past the prologue: Co-op Career, Press Start for
  player 2 (`key NumpadEnter`), the car select, the safe house, Resume Free Roam, and the split-screen world. That
  profile's only other car (0x1) is impounded (state 2), so the run went through the shared-car rule of section 3.1; its
  log reads `co-op: player 2 drives BMWM3GTRE46 (car 0x2, shared with player 1)` and `co-op: bust 2 joins`. With car
  0x1 released (state 0, no strikes) the same run gives player 2 their own car: `co-op: player 2 drives BMWM3GTRE46
  (car 0x1)`. Screenshot runs read profiles from `%TEMP%
fsmw-career-screenshot`.
- The benchmark joins co-op for real: release builds compile info logs out, so the same `--bench-seconds --offscreen`
  command on the debug build logs `co-op: Player 2 joins in 911TURBO` before its report.
