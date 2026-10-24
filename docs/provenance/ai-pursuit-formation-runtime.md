# Provenance: formation assignment and runtime integration

Date: 2026-10-23.

Specs: [formations](../specs/ai-pursuit-formations.md), sections 1-5 and 8;
[heat](../specs/ai-pursuit-heat.md), section 2.7;
[goals](../specs/ai-pursuit-goals.md).

## Sources and method

Implemented from the existing written specifications, without decompiled code
open. The formation specification credits
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
`AIPursuit::UpdateFormation`, `EvenOutOffsets`, `AssignClosestOffsets`,
`UpdateOutOfFormationOffsets` and `SetupCollapse`. Their GameCube origin remains
a limit on claims of exact PC equivalence. No source code was copied.

Tightness, duration and stagger fallbacks use the install-verified rows already
recorded in the heat specification. Production selection and execution now take
the current escalation table's installed attributes, including race rows.
These runtime reads preserve heat indices; they do not embed assets.

## Implementation

- `formation/assignment.rs` selects priority slots by aggregate candidate
  distance, performs lateral-weighted matching and spreads extra cars on the
  documented grid. PIT admits one attacker and rolling block four, independently
  of the authored alternate slots.
- `cop_aims/membership.rs` applies the same support/flee/range/world-ray filters
  to readiness and execution. Entity-to-slot bindings survive a running attack.
  Approach tightening uses the live arming timer. Herd crowding uses the road
  centre and outer traffic lane in the target's direction of travel.
- `police/formations.rs` holds selection during attacks, bust/ended outcomes
  and speech setup. Muted speech and pursuit-event context bypass speech gating.
  Re-selecting the active formation preserves its timers. Runtime row tuning
  controls both arming duration and the actual goal offsets.
- Collapse requires real eligible members. Circular ring assignment starts at
  the member closest to the front, including the left side of the angle seam.
  Normal driving clears former pull-over goals when collapse ends.

## Validation and limits

Regression coverage includes weighted lateral matching, alternate PIT side,
formation capacity, unique extra-car grid slots, the ring's angle seam,
selection interruption, speech delay, per-heat run lengths and closure progress.
Multi-car world regressions cover blocked rays, support exclusion, readiness,
stable attack slots after a target turn, and collapse-to-chase goal handoff.

The parent implementation run records executed checks and their results.
This change has no new matched original-PC driving recordings. Physical chase
results, exact radio timing, road-profile interpolation on changing lane counts,
and original/remake route-level acceptance remain unverified. Deterministic
state hashes now include picker/finisher/speech timers and retained membership.
