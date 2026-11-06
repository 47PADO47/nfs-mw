# Racer catch-up provenance

The inspected PC 1.3 executable has SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
Program identity was confirmed before querying its saved analysis.

`GRaceStatus::ComputeCatchUpSkill` at `0x005ff990` reads quick-race difficulty,
rival completion, current error, finite integral and averaged derivative. Its
ordinary quick-race branch interpolates spread and amplitude tables. It clamps
normalized output before multiplying by amplitude. Four five-point tables span
completion 0..100. Coefficients are 2 divided by spread, 0.00005 integral and
0.01 derivative. Difficulty factors are 0, 0.5 and 1; off-world forces factor one.
These claims come from bounded instruction and constant-data inspection. No
translated code is included in public sources.

`PidError::Record` at `0x00653e20` confirms trapezoid area and error slope. The
existing CC0 reference declarations in
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), `Misc/Table.hpp`,
`Misc/Table.cpp` and `AI/Common/AIVehicle.cpp`, establish ten integral and five
derivative samples, frequency one, recording interval and driver scaling. Read
for understanding; no code copied. Reuse the existing `blackbox_driver::PidError`.

A retained modded PC stock-Carrera-GT capture uses Hard, catch-up On, six rivals,
no traffic and police Off. Its executable SHA-256 is
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
Guarded calls record normalized output and raw skill offset. They independently
show raw saturation at 0.75 early in the race, with both offset signs. Hook
outputs precede base-skill scaling; the sampled driver field follows it.

Source inspection found the prepared switch ignored `RaceOptions::catch_up`.
Off changed base skill but left the estimated rubber band enabled. Resolve the
selected switch before preparing a race; retain the split-screen policy.

The replay clock audit covers 120 external seconds and 119.994484 race seconds,
with zero paused updates. Pending human placements occurred, so this is a
diagnostic input, not an exact replay. Opponent source keys refer to runtime
attribute collections. The later stock-race captures resolve their model parents;
see [race-stock-pace](race-stock-pace.md). The native selection algorithm remains
unresolved. This correction does not establish complete pace or career catch-up parity.

## Career branch (2026-11-06)

Same PC image, identity confirmed. The career branch of `ComputeCatchUpSkill`
selects race context 2, derives its Easy-to-Hard factor from the race status
adaptive bonus and halves the remaining distance to Hard for `BossRace` races
(`GRaceParameters::GetIsBossRace` at `0x005fbd90`). `CatchUpOverride` at
`0x005fc800` selects race-authored tables. The race status constructor and
`SyncronizeAdaptiveBonus` at `0x005dc1a0` read the profile's signed 16-bit value
scaled by 1/32767 and clamped to -1..1. Base skill for career racers reads
`SkillLevel` and the adaptive up/down tables (0.5/0.75/1 and 0.5/0.375/0.25).
Bounded instruction and constant inspection only; no translated code.

An installed probe of `GLOBAL/gameplay.bin` found `CatchUpOverride` false for every
race; 38 gameplay records resolve `BossRace` true. The CC0 reference front-end database stores the
adaptive value through a setter clamped to 0..1, read for understanding only.
Unit checks cover the factor endpoints, boss interpolation and identical career and
Medium quick-race outputs. This does not establish full career pace parity.
