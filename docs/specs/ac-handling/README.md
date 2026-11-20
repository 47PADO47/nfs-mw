# AC handling: specs

Part of [ADR 0016](../../decisions/0016-handling-hooks.md) and [the plan](../../plans/ac-handling.md). These specs describe
an optional handling model, in our own words, with enough maths and numbers that the mod `mods/ac-handling` can be built
and tested **without** an Assetto Corsa install or a checkout of any other project. None of them exists yet; this page
is the contract they will be written to.

## The firewall

The spec is the interface between two sessions.

1. The **spec author** may read any source that allows it ([licensing.md](../../licensing.md)): public vehicle-dynamics
   literature, observed behaviour of the reference game on their own install, and GPL or decompiled projects *for
   understanding only*. They write what the behaviour **is**: inputs, outputs, state, equations, constants, limits. They
   never paste code, translate line by line, or quote comments.
2. The **implementer** builds from the spec alone, with no restricted source open, and records the session in
   `docs/provenance/ac-handling.md`.
3. A **reviewer** checks the spec for copied expression and the code for anything the spec does not say.

If a spec is not enough to implement from, the answer is to fix the spec, not to open the source.

## Template

Every spec file starts with:

```
# <topic>
Sources: <name, URL, license, what was taken: facts or understanding> (one line each)
Written by: <session or person, date>   Implemented by: <session or person, date>
```

then these sections, in this order:

1. **Purpose and scope**, and what is out of scope.
2. **Inputs and outputs**, with units and sign conventions (the game's axes: x right, y up, z forward in physics space).
3. **State**, what persists between calls and its initial values.
4. **Equations**, in plain maths and, where useful, pseudocode that is not a transcription of anyone's code.
5. **Constants**, as a table with units, ranges and the reason for each value ("fitted to target T3", "from a published
   tyre", "a design value").
6. **Test vectors**, input to expected output, to a stated tolerance. Enough to pin every equation and every branch.
7. **Limits**, what the model does not do and where it is known to be wrong.

A spec stays under 500 lines; split by concern.

## Planned specs

| File | Covers | Stage |
|---|---|---|
| `tyre-force.md` | slip curve with peak and fall-off, load sensitivity, combined slip, friction ellipse, surface scaling | 0, used by 2 |
| `wheel-dynamics.md` | wheel spin, slip ratio and angle relaxation, low-speed handling, the substep rule | 0, 2 |
| `tyre-thermal.md` | core and surface temperature, pressure law, wear and grip as a function of each | 0, 5 |
| `suspension.md` | spring, damper (bump and rebound), anti-roll, camber gain, roll centre, hub motion | 0, 3 |
| `aids.md` | traction control, ABS, stability control, brake bias; levels and their meaning | 0, 4 |
| `steering.md` | steering ratio, speed-sensitive lock, rate limit, self-aligning torque to a force-feedback signal | 0, 4 |
| `parameter-mapping.md` | which NFS attributes feed which parameters; presets per compound class and car class; per-car override keys | 0, 2 |
| `targets.md` | the reference scalars and tolerance bands, with the scenario each comes from | 0 |
| `scenarios.md` | the scripted inputs behind `targets.md`, runnable by the harness and by `--drive-script` | 0, 2 |

## About `targets.md`

It holds **scalars only**: for each scenario a value, a tolerance band and the date it was measured, for example "peak
lateral g of the road-compound class on the 100 m skidpad". No recordings, no per-sample data, no game files, nothing the
leak check would reject ([licensing.md](../../licensing.md) rule 1 allows measured counts and values). A maintainer with
their own install of the reference game refreshes it with the harness in `tools/`; nobody else ever needs to. If a band is
unknown, the line says `unmeasured` and the matching test is skipped, not guessed.

## Where each thing lives

| Thing | Place |
|---|---|
| Why and how | [ADR 0016](../../decisions/0016-handling-hooks.md), [the plan](../../plans/ac-handling.md) |
| What the model is | this folder |
| The model | `mods/ac-handling/src/` (pure functions, unit tests from the test vectors) |
| Presets and per-car overrides | `mods/ac-handling/presets/`, `mods/ac-handling/cars/` (TOML) |
| What was read and when | `docs/provenance/ac-handling.md`, `NOTICE` |
| The host interface | `docs/modding/api.md` (`bb.handling`), added in stage 1 |
