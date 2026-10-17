# Driving car detail

Source: matched retail PC static inspection, with the shared CC0 car-render
reference used for names and cross-checking. See [provenance](../provenance/car-detail.md).

Car projected size uses radius times the view scale divided by centre distance
minus radius. Objects wholly behind the camera return zero. When that remaining
distance is at most half the radius, return twice the scale instead. This close
rule differs from the scenery selector; truncate the result to whole pixels.

- The PC ordinary body thresholds are 120, 25, 20, 10, 0 projected pixels.
  The PC traffic thresholds are **80, 40, 20, 10, 0**, differing from the shared
  console reference. Equality retains the more detailed level. The number of
  thresholds above the projected size is added to the allowed minimum and
  clamped to the maximum. Body and tire start with the same distance index;
  view-facing tire substitutions are a separate rule not yet reconstructed.
- Ordinary ride initialization permits A..E at High and B..E at Low. Usage
  overrides and reflection ranges exist; not all are classified. The remake's
  first adaptive route uses these ordinary ranges with the traffic threshold
  table chosen by the asset's usage type. This is explicitly partial parity.
- Always highest and a player-only highest override are remake enhancements.
  Other views retain their existing independent geometry policy.
- Assemble eligible driving LODs while the model loads. Retain canonical A
  placements for physics, damage, attachments, fragments and lights. Additional
  assemblies share the model's texture/material pool. View selection must
  neither read installed files nor upload meshes each frame.
- Missing lower-detail assemblies fall back to the closest available more
  detailed assembly (canonical A is always retained). Within an assembly,
  respect absent parts; do not fill slots with unrelated geometry. This whole
  assembly safety fallback is a remake choice, not complete original per-part
  retry semantics.
- Use canonical slot/corner identities across mesh transitions and unchanged
  physical wheel transforms; damage visibility applies by slot at every LOD.
  Material/light/cracked-glass changes refresh every cached LOD together.

The radius is reconstructed from canonical model bounds and projected with the
existing 480-line reference. Live PC projection, usage overrides, tire-facing
substitutions, part-specific maximums and original model retry semantics remain
validation gaps; UI and documentation must retain that qualification.
