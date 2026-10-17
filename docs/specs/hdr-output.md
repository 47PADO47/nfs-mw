# HDR display extension

This is a remake enhancement, not original PC rendering parity. The original
material arithmetic and artistic lighting are display encoded. A floating point
scene buffer alone does not make that pipeline linear or provide HDR output.

## Output contract

- SDR retains the existing UNORM output and original material clamps.
- HDR requires an explicitly supported RGBA16F / extended linear sRGB surface.
  scRGB uses linear BT.709 primaries, D65, with 1.0 representing 80 cd/m2.
- Auto enables HDR only when the OS positively reports HDR on the current display.
  On can opt in when metadata is unknown; an explicit OS SDR report still wins.
- Peak luminance uses a user calibration or an advisory display value. Invalid or
  missing metadata falls back visibly to 1000 nits. Calibration is finite and bounded.
- The window's display information is refreshed periodically and on resize. A
  display switch or HDR toggle reconfigures output and dependent targets.

## Colour and ordering

The authored scene remains display encoded through existing lighting and artistic
effects. In HDR only, final world/normal/glass/car RGB output can exceed 1.0;
intermediate material clamps that define original arithmetic remain. Reflection
captures retain their original SDR contracts. This extends existing highlights;
it does not recover clipped source textures or physically authored radiance.

After scene effects, FXAA and spatial upscaling, decode extended sRGB,
apply the pinned PsychoV mapper in linear BT.709 with a BT.2020 target gamut,
then scale reference white to scRGB nits. SMAA runs on the resulting display image
before UI, as on the SDR path. Preserve signed BT.709 components and
values above 1.0 through the final copy. Do not apply the legacy ACES pass too.
See [PsychoV provenance](../provenance/psychov.md).

The UI is composited last in linear light. Unpremultiply display-encoded UI RGB,
decode sRGB, multiply by alpha again, and scale by independent UI white / 80.
Transparent pixels must stay finite. UI brightness is independent of scene
grading, exposure and peak mapping.
The shared UI contract also encodes additive colour as nonzero RGB with zero
alpha (including authored minimap tiles). Decode that RGB
directly and keep alpha zero, so it adds light without covering the scene.
Only zero RGB and zero alpha represent empty coverage in this encoding.

The existing FSR1 implementation requires bounded positive input. In HDR only,
each stage uses reversible `c / (1 + c)` preconditioning on its samples and
inverts that mapping after filtering. This is a local HDR adapter; the normal
SDR algorithm and its numerical reference remain unchanged. Constant highlights
must survive both EASU and RCAS without being clamped to 1.0.

Ordinary PNG screenshots are SDR previews, with independent temporary output
mapping. They must not read float textures as RGBA8 or change live display state.
Numerical HDR checks read float output. An SDR screenshot is not proof of physical
HDR display presentation or correct monitor calibration.

## Coverage limits

The initial presentation implementation is native scRGB; other backends retain
SDR and report why HDR is unavailable. OS luminance values are advisory, not a
measurement. SDR can use a BT.709 / 1.0-peak PsychoV preview, but the original SDR
path remains the default fallback. No original game files or mod assets are needed.

API reference: [wgpu surface colour spaces and display information](https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-types/src/surface.rs).
