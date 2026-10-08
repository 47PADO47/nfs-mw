# blackbox-render

Backend-neutral renderer for EA Black Box game reimplementations: meshes in the games' common 36-byte vertex
format, DXT or RGBA textures, instanced draws, opaque/alpha-test/blend/additive passes, lit or pre-lit
shading, distance fog, a 2D UI layer (textured, clipped, premultiplied-alpha triangles for consoles, overlays and menus), off-screen capture. Runs on wgpu (Vulkan, Direct3D 12, OpenGL; Metal on macOS).

License: MIT OR Apache-2.0.

`EffectLayer` uploads reusable world-space triangle batches for depth-tested surface overlays
and soft alpha billboards. Effects draw after the scene and before UI; they do not write depth.
The caller owns lifetimes, budgets and particle ordering. Surface overlays have reverse-Z
polygon bias; both analytic masks are procedural and require no asset textures.
