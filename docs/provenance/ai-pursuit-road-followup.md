# Target road offsets and SUV action entry

Date: 2026-10-25.

Specification: [road follow-up](../specs/ai-pursuit-road-followup.md).

- Original supported PC `WRoadNav::SnapToSelectableLane` confirms continuous
  offsets inside the chosen selectable lane, followed by the inset corridor
  clamp. Earlier lane-index adapters omitted this retained lateral position.
- Original head-on ram action vtable and eligibility routine confirm interface,
  drivability and chicken checks without reading target speed. Its begin routine
  caps launch speed at 60 mph. Its update requests the fixed ram drive speed.
- Previously inspected cop navigator construction distinguishes the cop's false
  decision filter from human/racer initialization. Placement cursors and a live
  cop's own drive navigator must not share a placement-only junction exclusion.
- Native support placement increments its distance on every trial, including
  success, with eight attempts per car. Its road helper initializes at the
  target, walks the signed distance and supplies the road heading; deployment
  reverses that heading. Straight-line projection and a shared seven-index cap
  were reconstruction errors. Support sphere clearance is explicitly our safety
  adapter; exact native admission-query thresholds and batch activation remain open.
- Native road-action begin code compares the head-on support goal and writes its
  disable-seek-ahead flag. The unregistered entry was inspected from saved PC
  instruction bytes rather than treating failed decompilation as evidence.
- The CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reconstruction,
  `WRoadNetwork`, `AIVehicle` and `AIActionHeadOnRam`, supplies corresponding
  behavioral context. The existing target-speed eligibility was explicitly
  marked unconfirmed and confused launch speed with action eligibility.

Implementation follows the plain-language specification. No native code, private
capture contents or installed assets are included. Synthetic rule coverage and
installed pursuit outcomes must remain separate from full original-game parity.
