# blackbox-scene

Glue between the format readers and `blackbox-render`: uploading TPK textures (BC when possible, else RGBA8)
and solids (one mesh per solid, one draw per shading group), the blend rule from a texture's
`AlphaBlendType`, axis-aligned boxes and frustum culling.

License: MIT OR Apache-2.0.
