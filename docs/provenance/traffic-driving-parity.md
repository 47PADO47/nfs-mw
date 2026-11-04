# Traffic driving parity evidence

Specification: [traffic-driving-parity](../specs/traffic-driving-parity.md).
Analysis date: 2026-11-03. Original program identity was confirmed through
Ghidra `get_program_info`: PC x86 `speed.exe`, image base `0x00400000`, SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.

Sources are the original executable (read-only) and the CC0-1.0
[reference reconstruction](https://github.com/dbalatoni13/nfsmw), already
credited in NOTICE. Implementation follows the written behavior specification.

- PC steering routine `0x00415290` tests driver class 1, refreshes road
  occlusion through `0x00776600`, reads the resulting target and passes it to
  steering. Its zero-target-speed/low-horizontal-speed guard precedes this
  refresh. This is static dispatch evidence, not a running-game route trace.
- PC traffic speed routine `0x00411820` consumes the stopping table at
  `0x008eb394` through generic lookup `0x006533b0`. The table's range is 80
  and its multiplier word is `0x3c4ccccd`, approximately 0.0125. The older
  obstacle document misread this as 0.05. Ten isolated original executions
  cover negative, interior, boundary and saturated inputs; the documented
  range divided by 80 matches within 0.00001 m. The original-style reference
  executable was hashed independently and matches the analysis program.
- Event density and pattern precedence reuse the documented PC manager
  evidence in [traffic provenance](ai-traffic.md) and
  [traffic spawning](../specs/ai-traffic-spawning.md). The current adapter
  converted percentages to actor caps and did not consume `TrafficPattern`.

## Lane investigation and reference captures

- `RigidBody::GetSpeed` at `0x006711b0` measures the complete velocity norm.
  `AIVehicle::OnUpdateAvoidable` at `0x00408240` uses the larger of its stored
  radius and twice that speed. Six isolated speed vectors and twelve sweep
  cases verify forward, backward, sideways and vertical motion with retained
  radii. All complete targeted function bodies, including the square-root
  helper, match both reference executables. These checks do not operate the
  running game. The traffic update stores its decision's look-ahead through
  `SetAvoidableRadius`; the existing remake recomputed it on steering steps.
  Initial 20 m radius is sourced from `AIVehicle.cpp` in the credited reference
  reconstruction. Patrol controller behavior is outside this correction.

- `WRoadNav::ChangeLanes` at `0x00778250` supports immediate offsets and
  distance-based interpolation. Its ten direct call sites in nine examined
  functions all pass zero distance. The complete nine caller bodies match
  between the analysis executable and the modded reference executable,
  SHA-256 `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  This does not cover indirect callers or injected mod code.
- Six isolated executions of `WRoadNav::UpdateLaneChange` at `0x007660a0`
  verify interior interpolation, completion and inactive transitions. These
  are routine-level checks, not live driving observations.
- Passive modded-reference recordings on 2026-11-03 verified executable
  identity and complete probe-target bodies before attachment. The road
  networks in the original-style and modded reference files match. A
  post-reload recording contains 24 ordinary segment transitions. Four
  change lane index: three enter junction decision segments, and one
  renumbers a lane without changing its lateral offset.
- A subsequent 50-second stationary-blocker recording contains 13 ordinary
  traffic actors and 7,819 samples inside the video interval. All sampled
  accident states are zero. Its 34 ordinary segment transitions include
  nine lane-index changes, all entering junction decision segments. All
  660 changed-offset calls inside that interval belong to unclassified spawn
  eligibility navigation, not a tracked driving target. No positive-distance
  request was observed. No dropped events, layout rejections or probe faults
  were reported; the hooks and session detached cleanly.

The earlier pile-up recording is warm-up evidence only. Another recording
with no civilian samples is excluded. The later stationary-blocker recording
does not establish a pull-away/restart sequence. The added human-position
probe produced no samples, so player distance and displacement are unavailable.
Video and native clocks are anchored, but frame receipt is not simulation time
and instrumentation overhead has not been calibrated.

Raw executable bytes, decompiler output, instrumentation and recordings remain
private. The user supplied every gameplay input. No installed files were
modified. No matched live original/remake driving comparison has been completed
for this contribution. Tests and final validation are separate from these facts.
