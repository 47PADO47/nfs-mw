# A career race that was not won: the results, then the retry

What the original does when the player does not win a race the story waits on, and what the rewrite does after the results
screen. The rules are `nfsmw_career::story::loss` (`LossRule`, `loss_rule`), the question is `frontend/post_race/retry.rs`,
the report carries the rule (`viewer::RaceReport::retry`, set in `scenes/world/race/report.rs`).

- **Provenance:** [race-loss-retry](../provenance/race-loss-retry.md). Evidence tags: [docs/README.md](../README.md#evidence-tags).
  The Lua facts are the ones of [story-flow](story-flow.md) 1.1 and [story-flow-2](story-flow-2.md) F-08, F-09.

## 1. The original **[verified]** (Lua) / **[unconfirmed]** (native)

- A race whose placing is not first, whose engine blew, or whose time ran out clears the race's `ShowPostRace` flag and sets
  `ShowPauseMenu` before `raceover`. `raceover` then calls `ShowLosingPostRaceScreen` (a stub in this build), clears the pause
  flag and **stays**: the only exits are `restartrace` (back to `prerace`, the same race again) and `abandon`.
- `done`, and with it the race's `PostRaceActivity`, is therefore never reached after a loss. The prologue chain (16.1.0, 16.2.2,
  16.2.3, 16.1.1, then 16.8.1 and 16.2.1) and the boss approaches go on only from a won race. A loss never moves the story.
- Which native screen offers the retry in the original after the Lua hands over is **not seen** (the summary screen's retry and
  quit choices are **[unconfirmed]**, [race-mode](race-mode.md) section 11).

## 2. Races whose end is scripted **[verified]** (names and timers from story-flow 2.1 and 2.4)

Two races of the install do not end by a finish line, so "not winning" is part of their script and they never ask for a retry:

| Race | Why |
|---|---|
| `16.2.1` (the scripted chase) | The engine is sabotaged; the race goes to postrace on the `killrace` timer (11 s after the call starts) or 1 s after the engine blows. The car lot follows (`scripted_chase.rs`, which ends the race without a results screen). |
| `16.8.1` (the NIS shell) | No racing. Its cutscene is the race and it counts as won. |

No other race of the prologue is scripted to end lost: 16.1.0, 16.2.2, 16.2.3 and 16.1.1 chain on only from a win (their `bail_pursuit`,
`cop_spawn_trigger` and explosion children change the pursuit and the scenery, not the result). **[unconfirmed]** that no
other race's script ends it without a finish: only the Lua of the six prologue races and the final race was read.

## 3. The rewrite **[ours]**

The results screen (standings, statistics, stages) is shown as for any race. Answering **Exit** (Enter, the pad's A, a click on
Exit) then depends on `LossRule`:

| Rule | When | What Exit does |
|---|---|---|
| `Leave` | the race was won, is not a career race, or ends by script (section 2) | the race ends as before (free roam, the series screen, the Quick Race menu, the outro and the chain after a win) |
| `Offer` | a career race, not won, with no `PostRaceActivity` | a question: **Retry** (runs the race again, focused) or **Quit** (leaves it). Back closes the question |
| `Require` | a career race, not won, that names a `PostRaceActivity` (the prologue, the boss approaches) | a notice "You have to win this race to go on"; closing it (its one answer, or Back) runs the race again. There is no way on without a win |

A win is: the player placed first with a finishing time (the same test `answer_race` uses). Restart on the results is unchanged
(it asks OK or Cancel). A retry starts the same race again on the same grid, with no loading screen
([loading-screen](loading-screen.md) section 6).
