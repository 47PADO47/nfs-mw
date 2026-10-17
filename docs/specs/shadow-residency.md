# Bounded shadow scenery residency

The colour view uses the authored zone draw/load lists. Custom cascaded sun shadows also use
previously loaded scenery outside the current colour view. Retiring that scenery exactly when
the player crosses a zone boundary can abruptly remove shadows from casters behind the camera.

Both shadow modes maintain a separate support set outside the authored colour PVS. Its receiver
domain is an orientation-independent disk around the streaming focus: caster distance times
`sqrt(1+tan(fovY/2)^2*(1+aspect^2))`, plus the camera's horizontal offset from the focus. This
contains perspective far corners rather than just the camera's axial distance. Sweep that disk
180 world units upstream along the sun's horizontal direction. A section is eligible when its
authored XY circle intersects this capsule. Unknown sunlight uses a conservative disk fallback.
The region is reevaluated every update, including settings changes within the same zone.

At most 32 extra spatial sections are selected. The normal zone's wanted sections never count
towards this budget and are never evicted to make room for shadows. Extra sections are prioritized
by distance to their bounding circle; existing/previously selected sections receive a 32-unit
preference and a 25 percent larger exit domain. This reduces replacement from small movements.
Retirement obeys the same selected budget, preventing visited tiles accumulating inside the exit
region. Colour loading handoffs may temporarily retain additional old core scenery until the
new core is ready, as they did before shadow support. Disabling shadows removes extra support.

Retention does not change colour draw lists, request priority, collision readiness, or the
normal loading handoff. Retained tiles retain their existing resources, collision and prop
state until retirement. Missing support sections load after the colour set, with at most two
shadow-only requests in flight out of the shared eight-request budget. Shadow support does
not gate colour readiness. This bounds the geographic region rather than pinning every
previously visited section for the scene lifetime.

This is a remake streaming policy, not recovered original-game behavior. Budget exhaustion is
an explicit best-effort coverage limit, especially at maximum distant coverage or wide fields
of view. Eligible/selected counts are exposed and changes that hit the limit are logged at debug
level. The finite upstream range cannot cover arbitrarily long low-sun shadows. Section circles
can conservatively load geometry beyond the domain. The 180-unit upstream range, 32-tile budget
and hysteresis are remake choices. This bounds extra section count, not exact VRAM: authored
section sizes and decoded resource costs differ. Shader/GPU cost still requires route testing.

Synthetic regressions call the real residency update across two zone polygons, verify the old
caster survives the completed loading handoff, and verify colour draws do not grow.
They also cover retirement within the same zone, shadows off, original projection, distant
coverage, nonfinite metadata, receiver far corners, a crowded extra-section budget and small-motion
hysteresis. These tests require neither game assets nor a GPU.
