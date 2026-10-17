# Scenery sun-caster categories

The colour and shadow feeds have independent eligibility. Background panorama
models (`PAN_`, case insensitive), sky art and authored shadow overlays are not
physical sun occluders. Keep their normal colour/reflection placement intact.
Ordinary terrain, buildings, trees and props remain eligible; texture alpha and
renderer material rules decide which triangles contribute.

This is a remake content policy, not a recovered original-game flag decoder.
Installed panorama geometry can span and overlap playable roads; treating every
opaque range as an occluder creates broad false shadows after off-PVS streaming.
Do not infer the category from world position or size. Classification belongs in
the game scene, not in the generic rendering library.

Original and Enhanced styles share authored world material families. Simple
world shading remains an internal fallback for unsupported effects/backends.
Old profiles migrate to this baseline; quality changes preserve it. Custom sun
maps, contact detail, ambient occlusion and optional car shading stay independent.

Validate that installed panorama models still have colour meshes but never enter
the sun feed, and ordinary terrain/buildings/foliage retain their eligibility.
