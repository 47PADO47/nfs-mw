# Driving state audit

This audit follows the shared collision-phase change. It checks which tick and
which stage each reader consumes. It does not establish complete OG behavior.
See [consumer contracts](specs/driving-state-consumers.md) and
[prop/articulation phases](specs/prop-articulation-phases.md).

| Reader | Required state | Local correction or retained contract |
| --- | --- | --- |
| Engine and suspension | Post-contact body state | All body contacts and articulations finish first |
| Collision damage, sound severity and striker causality | Incoming contact speed and resolved impulse | Preserve per-contact snapshots; do not substitute final speed |
| Property infractions and pursuit watch | Completed contacts and step memo | Observe after current car response; drain each event once |
| Spike checks | Completed wheel patches | Observe after suspension writes this tick's patches |
| Traffic sound frame | Completed body motion | Publish after contacts, articulation and response |
| NOS contrails, flames and tail trails | Completed tick's boost state, pose and velocity | Capture one completed snapshot before observers |
| HUD gauges and visible pose | Interpolated completed endpoints | Keep interpolation separate from current discrete gear/NOS state |
| Car audio | Current physical/display RPM and its existing driver-intent inputs | Preserve raw pedal semantics pending original producer comparison |
| Camera | Completed body plus before/after acceleration | Retain incoming velocity for acceleration; publish completed pose |
| Prediction rollback | Saved physical state and terrain selection histories | Restore body/wheel face history; rebuild pure ray acceleration |
| Speedbreaker dilation, assists and NOS recharge permission | One frame-selected active state | Resolve frame depletion before choosing deltas and effects |
| Passive Speedbreaker recharge | Completed tick speed and simulation delta | Charge per completed tick rather than a future frame batch |

## Regression boundaries

Synthetic tests cover NOS press/release through actual trail emission, audio,
HUD, camera and pose endpoints; same-tick property contact reporting; constraint
impulses before driving response; prop collision before drag; and rollback
under overlapping road planes where a cold terrain search changes forces.
Speedbreaker tests separate frame-edge/depletion policy from completed-tick
passive recharge, including zero-tick frames and grouped threshold crossings.

These tests do not replace a matched original/remake route or manual countdown
acceptance. The public specs describe implementation targets; private validation
records identify the exact checked source and executable.

## Remaining boundaries

- Raw versus shaped audio/exhaust pedal sources need original producer evidence.
- Original unified mixed-body pair traversal and trailer-release task priority
  remain unverified; ordered local passes are not a claim of identical traversal.
- Exact OG within-frame Speedbreaker exhaustion timing remains unverified. The
  existing documented frame-level control/depletion policy is retained.
- Dynamic geometry, streaming/group transitions and complete route closure need
  separate acceptance; retained cache replay proves a narrower boundary.
- Complete sleeping-suspension wake and tire-heat consumers remain unverified.
  Landing-state updates agree with the retained boundary observations; exact
  creep/jump helper outputs do not prove every producer or full chassis tick.
