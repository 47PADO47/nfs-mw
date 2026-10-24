# Multiplayer: game modes, fairness rules, results and replays as built

Status: **built and unit-tested** in `crates/nfsmw-net` (modules `modes`, `fairness`, `results`, `record`) on 2026-10-22 and driven by `nfsmw-server`
(`server/phase.rs`: the world's events and every car's sample go in, the mode's events go out as messages). The catalogue and priorities are in [features.md](features.md); the messages in
[protocol-messages.md](protocol-messages.md). Values marked **[ours]** are this build's choice where the docs leave one open; the
module docs repeat them next to the code.

## 1. How the server drives a mode

The server builds a mode from a `ModeSetup` (mode, custom rules, players, start tick, carried heat, speed-trap thresholds, tag round)
with `AnyMode::build`, which validates the rules and refuses unsupported modes. Each tick it feeds every `ModeInput` of the tick
(the world's `SimEvent`s, one `CarSample` per player car, speed-trap passes, the shared heat, granted resets, departures), then ends
the tick. The mode returns `ModeEvent`s: events for clients and actions for the server (`Spectate`). `modes::messages` turns an event
into the gameplay messages of protocol-messages.md section 7.6. When the mode says `Over`, its placings, a `Tally` of session stats
and the `Roster` build the result record. Inputs before the start tick are ignored; time is the 60 Hz tick, never a clock.

## 2. Modes

| ID | Mode | Rules as built | Over when |
|---|---|---|---|
| MP-01 | Free roam | Pursuits on or off. On: one shared pursuit (section 3); a busted player respawns and plays on. Off: the world runs no police, pursuit events are ignored. Placings are unranked | never; the lobby ends it |
| MP-02, 03 | Circuit, sprint | The world raises laps and the finish. The server keeps the best lap and adds reset penalties to the finish time. Finishers rank by that time, then racers on the course by laps and progress, then racers who are out (the later out, the better). The finish event's place is provisional; the record is final | nobody is on the course |
| MP-05 | Speedtrap | A pass pays when it beats the trap's threshold and the racer's best at that trap; points are the sum of trap speeds (m/s). Points rank first, the finish time breaks a tie. No time limit or overtime penalty **[ours]**; a racer who left or was busted ranks behind every racer still in | nobody is on the course |
| MP-06 | Knockout | When every racer still in but one has completed a lap, that one is knocked out and spectates (`knockout_due`, one per lap). Every player is human, so the rule applies to all **[ours]**. A racer who leaves or is busted before the lap's knockout is that lap's knockout **[ours]**. The rule also runs on the last lap | one racer left (who wins at that tick), or nobody on the course |
| MP-07 | Pursuit co-op | One pursuit, several targets, ADR 0010 rules (section 3). A busted player spectates the rest of the round **[ours]** | the pursuit ends (evaded, all busted, abandoned), or every player left |
| MP-09 | Tag | One holder; each tick as "it" adds to the holder's timer. Contact: centres within 4.0 m and a relative speed of at least 1.0 m/s, both inclusive **[ours]**; the nearest qualifying player is tagged, the lower id on a tie. No tag-back for 3 s **[ours]**. First holder: the setup's choice, else the lowest id. The least time as "it" wins. A holder who leaves hands the tag to the player with the least time as "it" | the round's time (default 5 min **[ours]**) is up, or one player is left |
| MP-11 | Custom rules | `CustomRules`: laps (circuit and knockout need 1 to 10; other modes get 0 instead of a refusal **[ours]**), traffic density 0 to 100, cop level cap 0 to 5, time of day under 1,440 minutes, nitrous, pursuits (pursuits need a cap of at least 1; pursuit co-op needs pursuits). Knockout and tag need two players | |
| MP-04 | Drag | **Refused** (`Unsupported::Drag`, reason code 1): the drag race is not modelled in single player (ADR 0011 section 5) | |
| MP-08 | Cops vs racers | **Refused** (`Unsupported::CopsVsRacers`, reason code 2): open question 2 of features.md | |
| | Mod modes (128 to 255) | **Refused** (`Unsupported::ModMode`, reason code 3): needs the mod API | |

**Common race rules [ours].** A busted racer is out of the race (busted and DNF in the result) and spectates. A player who leaves is
out with the left flag, but keeps a finish already made. Of two equal finish times the lower player id is ahead, so places are unique.
Everyone busted ends the race with `AllBusted`; everyone gone ends it `Empty`.

## 3. The shared pursuit

`PursuitTracker` over `blackbox_pursuit::Targets`, as ADR 0010 decision 4 amends MP-07:

- **Shared heat.** The chase starts at the higher carried heat of the cars involved (at least 1); a car that joins later raises it to
  its own carried heat. The world reports changes. Every value is capped at the session's cop level cap.
- **Per-target bust.** A bust ends that car's chase only.
- **Shared escape.** The cool-down is shared, so an evade ends the chase for every target still chased.
- **End.** No target still chased: evaded if the team got away, else busted if a target was busted, else abandoned. A departure, or
  a chase the world drops without a bust or an escape, ends that target's chase as abandoned. A car whose chase ended does not rejoin
  the same pursuit; the next one can take it **[ours]**.

## 4. Fairness rules

| ID | Rule | As built |
|---|---|---|
| MP-32 | Catch-up | **Off by default.** On: every racer behind the leader runs a `blackbox_race::CatchUp` controller (the `basicrace` gains) over its gap to the leader, sampled once a second; the positive skill times 10 % **[ours]** is an extra fraction of engine power. Nobody is slowed |
| MP-33 | Collisions | `pair_collides(mode, a, b, ticks_since_start)`: full, ghost (racers pass through each other) or ghost start (ghost for the first 8 s **[ours]**, also on the grid). Traffic, police and the world always collide. A ghost body (a reconnecting player's parked car) touches only the world |
| MP-34 | Nitrous and damage | Nitrous off clears the nitrous button of every input before the world steps. Damage visual or off. Repair in the lobby on by default **[ours]** (the settings have no field yet) |
| MP-35 | Reset to track | One granted reset per 10 s per player in every mode (a refusal does not restart the wait). Refused while busted or under arrest, spectating, in the countdown or after the finish. A 5 s penalty **[ours]** in a race, free elsewhere. The server picks the spot |
| | Wrong way | Shown after 2 s against the route (cosine below -0.2) at 3 m/s or more, cleared after 0.5 s the right way; below the speed the state holds **[ours]**. A notice only: no automatic reset |

## 5. Result record (MP-70, MP-74)

`RaceRecord`, format 1: the magic `NMRR`, a format byte, then in the reliable messages' wire format: mode, track, laps, start and end
tick, and at most 32 rows of slot, entity id, place (0 when unranked), flags, time, best lap, top speed (cm/s), damage (0 to 255),
busted count, pursuit time (ms), resets and points. Flags: DNF, busted, knocked out, left. A later format appends fields to the body
(a new per-row field is a new list appended to the body); a reader skips what it does not know. `to_message` gives `RaceResults`,
folding knocked out and left into the wire's DNF bit. Slots are unique and places other than 0 are unique, or the record is refused.

## 6. Replays (MP-71, MP-72)

A header (magic `NFSMWRPL`, format, header length, server or client, the recording player, entity schema id, protocol minor, sim
build, data id, start tick, tick rate, CRC-32), then frames: sync `A5 5A`, kind (snapshot, event, input, predicted state; a key
flag for a snapshot without a baseline), tick, length (at most 256 KiB), payload, CRC-32. Ticks never go back. `finish` adds an
index of key snapshots (one per second at first; at 4,096 entries every other one is dropped and the stride doubles) and a 20-byte
trailer. The recorder holds at most its bound of undrained bytes (4 MiB by default) and drops and counts a frame that does not fit;
the caller does the file I/O. The reader seeks through the index or by a scan, skips a damaged frame to the next valid one and counts
the damage; only a damaged header makes a file unreadable. A client recording also holds its own input frames and predicted states.

## 7. Gaps

- A knockout, a trap score, a tag pass and a penalty are the messages `KnockedOut`, `TrapScore` (speeds in cm/s), `TagPassed` and
  `Penalty` ([protocol messages](protocol-messages.md) section 7.6; `modes::messages`, drawn as feed lines by the LAN HUD).
  Spectating travels as `RoleChanged`, the round's end as `RaceResults` and the lobby's phase change. A car joining a running
  pursuit is a `PursuitStart` for the known id.
- The real world raises laps and finishes for the race modes, but not yet speed-trap passes: a speedtrap round over the real world
  scores nothing until the world emits them (the toy world does not either).
- Drag, cops vs racers, mod modes, last standing by damage (MP-10) and handicaps (MP-36) are not built.
- No replay viewer (MP-73); the replay is not written as telemetry's JSON lines (F-33 asks for those first).
