# PsychoV display mapper

Ported under MIT from RenoDX, Copyright (c) 2025 Carlos Lopez Jr.; PsychoV
test30 Copyright (c) 2026 Carlos Lopez Jr. Full MIT notices are retained in
both concatenated WGSL fragments. No original-game shader code is involved.

- Repository: <https://github.com/clshortfuse/renodx>
- Pinned commit: `83cf7aa5680c7d4ff078ae7bd845d62a2cc8bdea`
- File: `src/shaders/tonemap/psychov/test30.hlsl`
- SHA-256: `12711440fe96b41211d2b0a2bbfd2ae88419d26fc3bb43ccd2f9bfd59da59c11`
- Entry: `renodx::tonemap::psychov::psychotm_test30`
- License: <https://github.com/clshortfuse/renodx/blob/83cf7aa5680c7d4ff078ae7bd845d62a2cc8bdea/LICENSE>

## Specialization and conversion

The port specializes upstream defaults: exposure, highlights, shadows, contrast,
purity scale, cone response exponent, gamut compression and manual compression
1; input/output gray anchors 0.18; source boundary BT.709. The positional
compatibility parameters retain their defaults, including white-curve mode 0
and clip point 100. Peak is display peak / scene reference white.
Gamut mode 1 targets BT.2020 for HDR, mode 0 BT.709 for SDR previews. The
algorithm's returned coordinates remain linear BT.709, including signed values.

A small source wrapper loads a float scene texel, decodes extended sRGB, calls
the function with those defaults, then either scales by reference white / 80
for scRGB or encodes sRGB for an SDR preview. Host settings reject non-finite
calibration. Upstream `isinf(x)` is expressed as `abs(x) > f32::MAX`, equivalent
for IEEE f32, because the WGSL writer has no `isinf` builtin.

Conversion used official DXC v1.9.2609 (2026-09-29): `-spirv -T ps_6_0 -E
fs_main -O3 -HV 2021 -fspv-target-env=vulkan1.1`. Naga 30.0.1 parses SPIR-V with
strict capabilities, no coordinate-space adjustment, validates the module, and
writes explicit-type WGSL. The two fragments are concatenated with a fullscreen
vertex entry before shader validation/creation. Generated complementary else
branches were rewritten as complementary if guards over immutable SSA values.
The diagnostic entry guard is local: it only decodes/scales diagnostic colours.
The final source sample uses normalized, clamped bilinear coordinates so a plain
render-scale resize can precede display mapping without out-of-bounds texel loads.
At matching sizes the numerical reference samples exactly at texel centres.

`display_vectors.rs` contains numerical outputs from that pinned HLSL wrapper
compiled with DXC, run as SPIR-V on a real Vulkan adapter. The WGSL port is tested
against those values for black, neutral ramps, saturated primaries, mixed hues,
negative input handling, large highlights and alpha, across three HDR calibration
pairs and a BT.709 SDR preview. The reference inputs are converted to FP16 in the
rendering test, so comparisons allow small quantization and backend differences.
These tests establish conversion agreement, not perceptual quality on every panel.

See [HDR output contract](../specs/hdr-output.md) for ordering, original arithmetic
limits, UI handling and display calibration caveats.
