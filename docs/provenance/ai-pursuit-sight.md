# Cop sight: occlusion, vertical gate and reversing status (epic E7)

## Query flags against the binary and a recording, 4 November 2026

The sight ray is the world query with both primitive kinds (faces and barriers) and a zero surface exclusion mask,
from the cop's body position plus 0.5 m to the target's plus 0.5 m. This was read in the decompiled `CanSeeTarget`
and `WCollisionMgr` sources and confirmed in the retail PC executable's disassembly (SHA-256 `80774c2e...`): the call
passes both primitive bits, and the segment's candidate filter skips any instance whose scenery group is switched off
before the exclusion mask is tested. The barrier test is a plain segment crossing with no facing test, so a one-sided
barrier blocks the sight from behind as well. No face rule beyond the general query applies.

A private recording of the remake (the F5 jump to Riverfront Stadium, the player parked, one chase cop 74 m away)
was cast offline against the install's collision packs with the rewrite's query rules. The ray reproduced every
recorded result. From 74 m to 56 m two building-outline barriers of group 0 block it, as they do in the stock game.
From 50 m to 35 m only the barriers of two scenery groups block it, and those groups are switched off in free roam:
in the matching stock recording a cop on the same line took the pull-over goal at 54 m, which needs a clear barrier
ray to the target. Over the whole remake recording (cops within 151 m, sampled each second), 42 of 650 rays were
blocked only by barriers of scenery groups (instances with a group number), none of them by a grouped face. No addresses, code
or capture data are in the repository.

Rewrite: `drive/sight.rs` now skips switched-off scenery groups and blocks on barriers from either side.

## Per-unit clock integration, 23 October 2026

Re-read reconstructed `AIVehiclePursuit.cpp` constructor, `ResetInternals` and
`Update`, plus `AIPursuit.cpp` observation aggregation. Each real vehicle AI task
now advances its own visibility timer and time since seen. Queries subtract the
0.25-second interval; success sets the unit age to -0.25. The pursuit takes the
minimum active-unit age after advancing its shared clock. The constructor and
promoted-patrol initial timer states remain distinct. No visibility random phase
or replacement scheduling rate was introduced.

Patrols retain their existing shared schedule. Roadblocks, hiding latches and the
cooldown display gate remain independent. Released/fleeing units do not provide
observations to their former pursuit. Helicopter and secondary-player sightings
remain independent current observations.

Spec: [per-unit clocks](../specs/ai-pursuit-visibility-clocks.md). Regressions cover
actual staggered task execution, recent sight after a failed ray, no duplicate
clock advance from repeated sensing calls, and accurate shared observation ages.
This does not claim matched original-PC recording evidence. The following older
notes describe the evidence available during the earlier investigation.

- **Source for the rule:** the decompiled pursuit sources cited in [ai-pursuit-cop-cars.md](../specs/ai-pursuit-cop-cars.md)
  section 2 and [ai-pursuit.md](../specs/ai-pursuit.md) section 4.1: `CanSeeTarget`, the 0.25 s timer, the heat ranges
  (151/151 heat 1 to 7, 201/101 heat 8 to 10), the rear cone `dot(cop.forward, unit(target - cop)) <= -0.3`.
- **Ghidra (`/speed.exe`, image base 0x400000):** the binary has no debug symbols. `CanSeeTarget` and `AIVehiclePursuit::Update`
  are not named. Class-name strings exist (`AIVehicleCopCar` at 0x0089304c, `AIVehiclePursuit` nearby in `.rdata`), but the
  xref lookup returned generic data items, not the referencing functions, so the vtable and method were not found. The
  occlusion query flags, the vertical test and the 7 s grace were **not** confirmed in the binary.
- **Tools requested but unavailable in this session:** `search_instructions`, `search_byte_patterns`, `get_function_pcode` and
  `analyze_control_flow` did not load (ToolSearch found none). Only `list_data_items_by_xrefs` loaded.
- **Occlusion choice (rewrite, superseded 4 November 2026):** every face blocked, including floors, and so did grouped
  scenery that was switched off (`skip_groups` off). The one pass-through was a one-sided barrier from behind. The query
  flags section above replaces both choices.
- **Vertical gate:** `COP_LEVEL_GATE` 8 m is a design value (**[unconfirmed]**), chosen so a cop 20 m above or below is off level.
