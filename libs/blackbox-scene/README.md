# blackbox-scene

Glue between the format readers and `blackbox-gfx`: uploading TPK textures (BC when possible, else RGBA8)
and solids (one mesh per solid, one draw per shading group), the blend rule from a texture's
`AlphaBlendType`, axis-aligned boxes and frustum culling. It only knows the `RenderBackend` trait object, so any renderer can be handed the
uploads.

License: MIT OR Apache-2.0.
