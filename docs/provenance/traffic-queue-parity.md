# Civilian queue eligibility evidence

Specification: [traffic queues](../specs/traffic-queue-parity.md).
Analysis dates: 2026-11-03 and 2026-11-04. Sources are read-only PC executables and the CC0-1.0
[reference reconstruction](https://github.com/dbalatoni13/nfsmw), credited in
NOTICE. No original code or assets are included.

Ghidra `get_program_info` confirmed PC x86 `speed.exe`, base `0x00400000`,
SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
The modded reference executable hashes to
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.

## Native eligibility gate

`WRoadNav::HolePunchAvoidables` starts at `0x00775be0`. Its complete body matches
both executables. The branch region beginning at `0x00776172`, after the
closest-approach calculation, checks the inside flag, traffic navigator class,
signed closing speed, longitudinal/side distances and approach time. It reaches
the first-cut calculation at `0x007761f1` or rejects through `0x007765b8`.

Ninety-six isolated executions cover navigation classes, inside flags,
stopped/closing states, three-second boundaries, longitudinal range, side
clearance and speed-expanded range. They confirm that forward traffic obstacles
remain eligible beyond three seconds while inside obstacles retain the cutoff.
These executions use synthetic post-approach state; they do not execute the
entire hole-punch routine or observe live driving. Inputs, outputs, complete
function hashes and the original bytes remain in private evidence.

The previous remake applied the three-second closing-obstacle cutoff to every
class. It also used full centre distance instead of traffic's longitudinal and
side-clearance tests. This correction is restricted to the traffic branch.

## Restart and reproduction limits

`ComputeSpeed` at `0x00411820`, normal pedals at `0x00408430` and simplified
movement at `0x00422780` establish distinct request, pedal and direct-motion
rules. Simplified movement requires vehicle physics mode 2, read by the getter
at `0x00688210`. Fifteen complete relevant function bodies match both copies.
Bounded caller searches did not establish ordinary nearby traffic's mode
selection. The rigid-body status byte in earlier captures is a different field.

Before correction, synthetic feedback with a stationary oncoming player
reproduced attempted footprint overlap from both 14 m and 30 m starts. The latter
included a stop followed by renewed gas. Player pair-contact response was omitted
and observation stopped at first overlap, so accident response could not hide
the driving request. These are remake observations, not native stopping distances.
That first diagnostic used Regularized integration. Follow-up comparisons
separate it from the Reference integration selected by installed-car readers.
The retained queue regression uses Reference with synthetic vehicle parameters;
live asset-specific brake behavior and exact native gaps remain unverified.

The user supplied all reference-game controls. No new runtime attachment or
installed-file change was required. Raw code and diagnostic logs remain private.
