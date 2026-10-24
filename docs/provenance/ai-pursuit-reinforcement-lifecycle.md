# Provenance: pursuit reinforcement lifecycle

Date: 2026-10-23. Spec:
[reinforcement lifecycle](../specs/ai-pursuit-reinforcement-lifecycle.md).

The implementation is independently written from the spec. Restricted inputs
were read only for behavior; no decompilation, game assets or raw capture records
are included in the repository.

| Fact | Evidence | Limit |
|---|---|---|
| Successful ordinary spawns retain the selected road navigator; the seek-walk branch reverses it before the vehicle reset | CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), `AI/Activities/AICopManager.cpp`, `AI/Common/AIVehicle.cpp`, `World/WRoadNav` | Static behavior; the remake has its own ground and navigator adapters |
| Ordinary reinforcement body speed is zero at the spawn-state reset | Private passive PC runtime capture: 48 ordinary activations, separately identified by native caller and request lineage | One modded installation; not a measured post-spawn acceleration rule |
| Spawn-state reset does not move the body between its entry and return | Same capture, all 94 accepted activations including separate support roles | Placement occurred before the observed reset; phase names do not imply pre-placement body state |
| Requests retain the missing contingent count; later joins alternate the row delay with 0.2 seconds while that count exceeds two | PC native request and attachment instructions, cross-checked against `AIPursuit::CopRequest` and `OnAttached` reconstruction | Full native function bodies match between the two owned executables; capture pair timing is corroboration, not the source of the constant |

The executable function bodies used for lifecycle classification match between
the owned stock and modded PC installations. Runtime modifications, traffic
limits and route choices can still change behavior. In particular, the modded
capture's count limits and timing are not adopted as defaults.

Checks are separate: a synthetic production-placement regression proves cursor
and initial-speed contracts; installed encounter tests cover their integration.
Neither by itself establishes complete original-game pursuit parity.
