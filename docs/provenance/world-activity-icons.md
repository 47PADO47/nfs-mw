# World activity icon provenance

The installed retail PC 1.3 executable (SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`) contains
a consecutive 17-entry table of icon type, model and emitter-group hashes.
The same type order and model/particle structure are declared in
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
`Gameplay/GIcon.h`. Installed InGameA supplies matching model names and the
installed attributes database supplies the matching emitter group names.
The executable was inspected for behavior; no executable code or assets are
copied.

2026-10-28: retail PC static inspection added placement, ground-snap and
pursuit-phase rules to the spec. The reference's PC symbols identify
`GZoneTrigger` construction, `RefreshZoneIcons`, `UpdateTriggerAvailability`,
`UpdatePursuit`, `UpdateIconVisibility`, `GIcon::SnapToGround` and
`GIcon::SetPosition`; its generated gameplay header identifies TargetActivity,
engagetrigger and BinIndex. Ghidra program identity matched the hash above.
Synthetic tests check phase transitions, authored positions and ground-snap
eligibility. No matched original runtime capture was performed for this pass.

A read-only installed-data census found 146 non-template engage triggers,
all with radius 25 m and nonzero elevation. Their 146 distinct TargetActivity
keys all resolve to races with a start grid. TargetActivity hashes to
0x277566F3, matching the constructor's field access. Positions round-trip
through map-to-physics and physics-to-render conversion without displacement.
