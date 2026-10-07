# blackbox-render

Backend-neutral renderer for EA Black Box game reimplementations: meshes in the games' common 36-byte vertex
format, DXT or RGBA textures, instanced draws, opaque/alpha-test/blend/additive passes, lit or pre-lit
shading, distance fog, off-screen capture. Runs on wgpu (Vulkan, Direct3D 12, OpenGL; Metal on macOS).
`Backend::Dx11` is reserved for a separate Direct3D 11 implementation (wgpu has none).

License: MIT OR Apache-2.0.
