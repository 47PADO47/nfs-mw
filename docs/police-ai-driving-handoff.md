# Police driving handoff and remaining parity work

Status: partial implementation, with unresolved user playtest failures.
See [the full coverage report](police-ai-parity.md) for the wider police system.
This handoff covers the accumulated road, reinforcement and SUV follow-up.

## Implemented

- Preserve continuous pursuit road offsets across initialization, reversal,
  primary-target tracking and close snapping. Keep indexed traffic placement separate.
- Correct bend-apex displacement, passed-cookie look distance and section interpolation.
  Select local avoidables before the 32-body cap. Nearest-body retention is our
  adapter; exact native neighbour ordering and formation exclusions remain separate.
- Reinforcements consume the shared target-owned seek cache. Promotion, accepted
  placement and road-action reentry preserve the appropriate physical cursor and
  permit driving through junctions.
- SUV ram entry uses drivability instead of an incorrect target-speed veto.
  Head-on support approaches the current target before ram entry. Pair searches
  advance after success, walk the road and use separate physical spacing.
- Roadblock lookup uses zero heading weight and crosses junctions before checking
  its destination. Accepted heavy support, including RAM, postpones ordinary
  roadblock rolls by the native 15 seconds.
- Pursuit status exposes support delays, sight/busy state, attached roadblocks,
  helicopter eligibility/slot, order and dispatcher latch. Pending support expiry
  warns with placement counts and denials.

The physical BMW fixture now uses production AI control mapping on the installed
racer chassis. Its route and police thresholds were not relaxed. Earlier invalid
target-pilot runs remain invalid.

## Validation and limits

The implementation snapshot was checked with GNU Rust 1.99.0, default features
disabled, `nfsmw/multiplayer` enabled and the deprecated Bevy renderer excluded:

- Leak/size guards, full formatting, Python and license checks passed.
- 8,807 CPU tests passed across 139 suites; 850 installed/GPU cases were ignored.
- Thirteen selected driving checks passed: catch-up (4), road course (4), seek
  cache (2), automatic reinforcement driving (1), installed SUV pair physics (1),
  automatic installed-row support selection (1).
- Release compilation passed. Strict Clippy is not clean: eleven diagnostics
  occur in files byte-identical to fetched main. No lint category was suppressed.

Automatic support coverage uses three fixed seeds, installed rows/vehicle data,
stand-in rigs and synthetic collision. It observes SUVs at heats 3-5 and helicopter
selection at heats 4-5. The reference heat-three wave has no helicopter slot.
This does not validate visual model loading or complete live city deployment.
No final renderer, matched original/remake run or manual gameplay pass is claimed.
User playtests reported only slight improvement and the failures below.

**The independent physical city pursuit still fails.** Target validity passes:
67.018 m/s mean, 2,365.34 m progress, maximum route error 4.251 m, ground support on
2,090/2,100 ticks. The same cop remains for 2,100 ticks, closes from 92.724 m to
54.410 m, reaches 81.481 m/s and finishes 87.718 m away. Late moving engagement is
**146/600**, below the unchanged minimum **200**. Group contact is 367/600.
Passing narrow speed or spawn checks does not close this failure.

Recorded-target city and adverse-entry catch-up stresses also remain failed;
their assumptions differ from the physical BMW case. City-route cases require
locally supplied traces. Captured data is not embedded in the repository.

## Next milestones, in priority order

### 1. Sustain existing cops through roads and obstacles

Open report: cops hit traffic, roadblocks and walls too often, then fall behind
and make escape easy. Do not mark this resolved by the local-list correction.

- Diagnose the valid city failure by unit identity, actual/requested speed,
  road/close action, cursor, occlusion, contacts, sight and lifetime.
- Verify native path refresh scheduling before changing the remake's one-second
  replan period. Asynchronous pathfinding may matter.
- Replace the governor's approximate target steering input only after establishing
  the correct equivalent of the original target drive navigator.
- Add populated traffic, static roadblock vehicles, closed-region barriers and
  walls to integrated scenarios. Separate route, avoidance, steering and physics.
- Keep city thresholds unchanged. Do not hide retirement behind replacements or
  conceal route failures with a blanket power increase.

Complete junction/offset blending, collision query masks, pitched capsules,
airborne behavior and police tire-model equivalence remain unproven. Racing and
secondary/co-op targets have additional adapter/ownership boundaries.

### 2. Finish deployment and roadblock lifecycle acceptance

Open reports: roadblocks sometimes appear behind the player; SUVs and helicopters
were not observed in live higher-heat testing.

- Check actual travelled roads, junctions, bends and stacked roads. A curved road
  can legitimately lead behind the player; a global heading-dot rejection is not
  a parity rule.
- Exercise RAM and ordinary roadblocks together after the 15-second interlock
  correction. Separate selection, loading, admission, activation and lifetime.
- Check roadblock detach/recycling and pending dispatcher latches. The original
  suppresses heavy support while a roadblock is attached; identify unintended
  blocking state rather than removing the rule or raising chances.
- Complete native batch acquisition/activation, SUV chicken-state entry, exact
  admission queries and attachment/count/timer integration.
- Verify helicopter model loading, search/fuel/respawn transitions and visible
  flight. Synthetic selection/interpolation checks do not prove flight parity.

### 3. Fix hazards and pursuit-state feedback

- Ground spike strips using supported road surfaces and model pivots. Prevent
  overlapping or poorly fitted roadblock cars and props.
- Reproduce cooldown/radio gaps followed by unexpected roadblocks. Check sight,
  last-known position and phase restrictions on search deployments.
- Check initial high-heat spawn bursts, backup triggering/countdown and its HUD.
- Correct minimap visibility and white/wreck icons; add destroyed-cop text and
  the matching escaped-pursuit statistics presentation.

The wider offence, arrest/payment/impound, escape/restored-world and radio gaps
remain in the full coverage report. Preserve reported failures until relevant
integrated evidence establishes their resolution.

## Evidence to reuse

- [Shared prediction and road steering](specs/ai-pursuit-reinforcement-prediction.md).
- [Target offsets, junction handoff and SUVs](specs/ai-pursuit-road-followup.md).
- [Obstacle selection and roadblock interlock](specs/ai-pursuit-obstacle-selection.md).
- [Physical city acceptance](specs/ai-pursuit-physical-target-acceptance.md).
- [Support lifecycle](specs/ai-pursuit-support.md).

Original executable analysis, captures, exact local manifests and isolated bundle
identities remain private. Reuse evidence only for matching source/configuration;
do not publish installed assets or native code in follow-ups.
