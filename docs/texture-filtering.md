# Texture filtering

Texture filtering is a separate scene-surface quality preference: Trilinear,
Anisotropic 8x (the default), or Anisotropic 16x. Select it in Video options or
F8 Materials. F8 Save profile preserves it; startup restore, saved profile load
and A/B comparison also include it. Visual styles and quality presets preserve
this independent choice. Config uses `texture_filtering = "anisotropic-16x"`;
`NFSMW_TEXTURE_FILTERING` and `--texture-filtering` accept the same names, or
`8x` / `16x`.

The previous common native texture sampler and Bevy image sampler were fixed at
8x, while the native normal/reflective-road and sky material samplers used linear
trilinear sampling without anisotropy. The new setting makes these surface paths
consistent. Installed mip chains, texture colour space, shader LOD bias and
reflection capture policy remain unchanged. Filtering cannot restore missing
mips or detail absent from the installed texture.

Native changes apply immediately to retained texture and material bind groups.
Mesh, material and texture handles stay stable; texture pixels are not reuploaded.
The native common/car diffuse, normal/reflective-road, architectural glass and
sky surface samplers follow the preference, including newly streamed resources.
Only an actual change rebuilds their bind groups; steady-state frames do no
sampler or resource rebuilding. UI, post-processing lookup/gain maps, shadow
comparison/filtering and environment cube samplers retain their separate policies.

Bevy applies the choice before the first surface upload. Later changes are saved
but require a renderer restart; the active value is reported in the apply result
and newly streamed images keep that active value. This retains render-only image
ownership rather than keeping a second copy of every texture in CPU memory just
to change its sampler. Video labels this requirement and F8 explains it.

Both factories inspect the actual adapter's anisotropic-filtering downlevel flag.
If it is absent, the effective choice is Trilinear with an explicit capability
downgrade. Requested preferences remain saved. 8x and 16x use linear magnification,
minification and mip filtering with the corresponding anisotropy clamp.

This is a renderer quality control, not a claim that all original PC material
sampler states or driver overrides have been reproduced. The implementation
follows this checkout's wgpu sampler descriptors and anisotropic-filtering
capability contract. Relevant sources are `blackbox-gfx/settings/filtering.rs`,
`blackbox-render/gpu/filtering.rs` and `blackbox-bevy-render/texture.rs`.
