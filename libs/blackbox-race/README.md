# blackbox-race

The **rules of a race** for EA Black Box games (Need for Speed: Most Wanted today), as pure functions and small
state machines on plain numbers. No I/O, no vehicle, road or rendering dependency.

- `Gate`: an oriented box (a checkpoint or a finish line) with a point test and a segment test.
- `Course` and `Progress`: the ordered checkpoints and the finish line, laps, and the events a racer's movement
  raises (checkpoint reached, lap finished, race finished), plus percent complete along the route.
- `rank`: the ordering of the racers (finished first, then racers still racing by laps and progress, then the
  eliminated; by points when asked).
- `Clock`: the race phases (staging, countdown, racing, over), the time limit and bonus time.
- `knockout_due`: the lap knockout rule. `SpeedTraps`: points from speed-trap passes.
- `CatchUp`: the rubber band controller that turns the gap to the player into a signed skill offset.

The caller supplies all tuning (time limits, laps, gains). Positions are 3D with a caller-chosen up axis.

Specs: `docs/specs/race-mode.md`, `docs/specs/ai-racers-catchup.md`.

License: MIT OR Apache-2.0.
