# Tire smoke and skid marks

## Sources and scope

- **[verified source]** This repository's `blackbox-vehicle::WheelState` and
  `Vehicle::wheel`, MIT OR Apache-2.0: loaded contact patch, radius, steer angle,
  slide speed and normalized `skid` / `smoke` outputs. Existing thresholds are
  `SKID_RANGE` (2–5 m/s) and `SMOKE_RANGE` (3.5–9 m/s).
- **[verified source]** This repository's `WorldGround` and `CollisionWorld`,
  MIT OR Apache-2.0: resident faces, surface exclusions, hit point, normal and
  collision section identity.
- **[new design]** Everything below describes this rewrite's visual layer.
  No original binary, decompilation or external implementation was consulted.
  The appearance, emission, lifetime and budgets are not claims about the game.

## Inputs and timing

After each fixed physics step, read all four loaded wheel patches and their
existing intensities. Do not derive another slip threshold. Project each loaded
patch down a short segment onto faces with the same exclusions as `WorldGround`.
If there is no nearby ground, produce no effect for that wheel. Convert hit
positions, normals and the steered wheel forward vector into render space.

Effects advance by the fixed step in deterministic screenshot scripts. During
interactive updates without physics steps (free camera, waiting for a road),
they age by frame time without emitting. Lost contact, disabled marks, respawn,
car replacement and discontinuous motion break each wheel's track anchor.

## Smoke

Emission accumulates `smoke * 35 * dt` per wheel, independent of travel speed.
This permits stationary driven-wheel burnouts. Each emitted particle starts just
above the contact plane, inherits a small part of car velocity, spreads sideways
and rises. A deterministic sequence varies size, spread and lifetime.

Keep at most 512 live particles, replacing the oldest on saturation. Lifetimes
are 1.4–2.0 seconds. Size expands, opacity ramps in briefly and fades to zero.
Build camera-facing quads sorted back to front. A procedural radial mask gives
soft edges; no copied textures are required. Draw after world geometry with
normal alpha blending, scene depth testing and no depth writes. The UI/HUD pass
remains last and uses its existing shader.

## Skid marks

Opacity follows `skid` directly. Store the exact contact point, normal and section
identity; use a procedural tire strip 0.24 metres wide. Join contact cross-sections
only while both samples are loaded, marking and in the same section, at most
3 metres apart, with normals having dot product at least 0.8. Sample at least
0.18 metres apart. A join below that distance updates the active burnout stamp's
opacity instead of allocating overlapping quads every tick. A stationary marking
wheel leaves one small grounded stamp, oriented along its steering direction.

Keep at most 2048 quads. Marks last 45 seconds and fade during their final
5 seconds. Remove marks and anchors immediately when their collision section
is unloaded. Use per-end contact normals for ground placement, a 0.012-metre
normal offset and reverse-Z polygon bias to prevent depth fighting. Break rather
than bridge a sharp ground change or a teleport. Existing finished marks may
remain after reset, within the lifetime and budget; anchors never join to them.

## Controls and verification

`tire_smoke` and `skid_marks` default on and can be independently disabled.
Disabling clears that resource and its emission/track state. The world console
offers `tire-effects` status (counts, limits, oldest age and emission totals),
`tire-effects clear`, and per-effect on/off commands. Layered settings use the
same scene seam as these toggles.

Synthetic tests exercise normal rolling/no contact, moving skid tracks,
stationary burnout saturation, lifetime expiry, disabled effects, unloading,
teleport and normal changes, and deterministic output. Driving captures use
existing scripts and actual install collision; interactive checks verify depth,
HUD separation, smoke expiry in free camera and reversible toggles. Resource
geometry is rebuilt into reused CPU/GPU buffers, with no per-particle mesh or
texture allocations.
