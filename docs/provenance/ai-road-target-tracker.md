# Persistent target-vehicle road tracking

- **Spec:** [ai-road-target-tracker.md](../specs/ai-road-target-tracker.md).
- **Sources read:** Original owned PC v1.3 executable, inspected read-only through
  Ghidra and bounded x86 disassembly; the CC0-1.0
  [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `src/Speed/Indep/Src/AI/Common/AIVehicle.cpp`: `UpdateRoads`,
  `GetSeekAheadPosition`, `path_spot`, and `road_walker`.
- **Spec written:** 2026-10-24, before implementation. No decompiled text, game
  data, capture coordinates, or trajectories are included.
- **Implemented:** 2026-10-24 from the written specification, with the decompiled
  reference closed. Generic target tracking and its bounded walker live in
  `blackbox-roads`; game ownership and observation selection remain separate.
- **Validation status:** Eight generic tracker regressions, nine game ownership
  and observation regressions, and two copied-driver-state regressions pass.
  The stronger recorded-target city stress still loses its initial unit;
  this implementation does not claim complete driving parity.

## Evidence and confidence

The Ghidra program identity was checked before queries: PC executable SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`,
32-bit little-endian x86. Bounded inspection covered `UpdateRoads`,
`GetSeekAheadPosition`, `road_walker::walk_road`, `walk_all_paths`, and
`evaluate_end`. All five inspected code ranges also matched a second locally
installed executable. That byte comparison does not establish runtime policy,
data, or gameplay equivalence between installations.

Native instructions and constants corroborate retained current/future cursors,
the 20 m validity test, 0.02/0.33 s clocks, 0.05 m increment thresholds,
previous-future identity, bounded walker scores, and the separate copied-current
seek cache. The source reference supplies names and readable structure. Its
platform-specific structure offsets are not used as PC layouts.

The seek getter had no Ghidra function object at its symbol entry; its bounded
native instruction range was inspected directly. No function or database
metadata was created to produce a result. The driving reset was also inspected:
it does not write the retained target-road state or its clocks. Constructor
clock initialization was read from the reference and is not independently
claimed as a native runtime measurement in this pass.

The previous remake adapter constructed target road inputs from new weighted
closest-road queries for each cop think. Installed geometry checks showed that
this weighted navigation selection can differ substantially from physical lane
association. A valid-geometry city chase still lost its original unit. These
observations motivated the persistent input-state work but do not isolate it as the
only cause. No matched original/remake runtime experiment is claimed.

## Deliberate adapter boundaries

Simulation clocks age even without an authorized body observation, but no
inactive snapshot is exposed. A single primary tracker is shared before the
car loop; secondary assignments retain an explicit separate fallback until
their target vehicle identities own equivalent state. Existing hidden-target
selection remains authoritative. Continuous selectable offsets are corrected by
the [road follow-up](ai-pursuit-road-followup.md). Complete junction splines,
broad-phase grid, race and barrier adapters still limit exact geometric parity.

The current adapter retains primary vehicle history before a pursuit and across
pursuit starts. Production-step regressions cover missing bodies, vehicle/seat
replacement and remembered search targets without revealing hidden live motion.
