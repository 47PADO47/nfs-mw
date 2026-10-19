# Optional exact shadow cache

F8 Lighting > Custom cascades > **Cache unchanged shadow cascades** enables a
saved, independent option. It is off by default. The original redraw-every-frame
path remains available and the original projected shadow system is unchanged.

A unique persistent object key with an unchanged mesh/transform may enter static
depth. Unkeyed, duplicate-key and moving instances remain dynamic. Static depth
is reusable only with the exact same cascade projection and caster/resource
fingerprint. Changes to geometry, resident resource identities, cutout texture
bindings or redirects, material interpretation, light/projection, map size and
caster membership invalidate reuse. Camera cuts and disabled shadows reset it.

When dynamic casters exist, cached static depth is copied into the working map
and dynamic casters rasterize over it every frame. With no dynamic casters, an
unchanged final map can be kept without a copy or draw. Static/dynamic and legacy
draws share the same material, alpha-test, bias, viewport and pipeline code.

The implementation preserves the existing density, coverage and filtering. It
does not freeze the sun, throttle dynamic shadows, enlarge cascades or reduce
resolution. Consequently, camera/sun movement can prevent reuse; this first cache
primarily helps stable projections. The F8 reused-cascade count exposes this
limit. It is not a scrolling world-space shadow cache or a moment-shadow method.

Cached maps increase VRAM use, included in the F8 atlas memory count. Copying large
maps also costs bandwidth, so reuse with many dynamic casters may cost more than
redrawing. Measure GPU sun time and base-frame time with both options on the same
route before selecting it as a performance default.

Synthetic coverage compares pixels against fresh redraws, verifies actual static
reuse, dynamic movement/removal, changed sun direction, resizing and live toggles.
