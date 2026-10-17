# Original PC sky shading

Status: embedded PC shader arithmetic verified statically and with an isolated
D3D9 pixel harness. Runtime draw ordering, texture assignment and cloud weights
must be established separately; these fixtures do not prove scene-level parity.

Source: the retail PC executable's embedded SKYBOX effect (proprietary; behavior
reference only). The preferred modded executable contains the same effect bytes.
No original shader, shader bytecode, texture or private harness is included here.

## Inputs and vertex processing

The three selected techniques accept position, UV0 and UV1 from
the programmable PC `SKYDOME_XENON` model, whose stock 44-byte declaration
contains both UV sets. Its authored diffuse is `SKY_NEXGEN_CLOUDS_A`; legacy
ordinary-dome environment texture substitutions do not apply. They transform
position by WorldViewProj. UV1 is independent from UV0; it must not be silently
replaced with the first UV channel. CloudScroll is a scalar uniform.

The base layer produces these texture coordinates:

- Diffuse sample 0: `(UV0.x - 0.4 * CloudScroll, UV0.y)`.
- Miscellaneous sample 1: `UV1`.
- Diffuse sample 2: `(UV0.x - 0.2 * CloudScroll - 0.3, UV0.y)`.
- Interpolated atmospheric background: the vertex result described below.

The second and third layer vertex programs produce
`(UV0.x - CloudScroll, UV0.y)` and `UV1` without those base-layer scale factors.
The second layer samples the first coordinate; the third samples UV1. Texture
sampler addressing/filtering are runtime state, not established by shader code.

## Atmospheric background

At each vertex, define `c` as the dot product of the normalized local light
vector and normalized `(localEye - position)`. Unlike the selected ordinary
world technique, the sky vertex program normalizes the light vector itself.

Using the named atmospheric parameters:

```text
phase = Const1 * (1 + c*c)
      + Const2 * Const3.x / pow(Const3.y + Const3.z*c, 1.5)
extinction = exp(-15000 * (Br + Bm))
background = clamp(phase * OneOverBrBm * (1 - extinction) * Const3.w, 0, 1)
```

All operations on RGB vectors are componentwise. The shader uses its own fixed
sky distance; the ordinary scenery FogValue is not an input to this technique.
The clamped background is interpolated from vertices before pixel composition.
This is an atmospheric background, not a texture multiplied by vertex color.

## Pixel composition

For the base layer, let `T0` and `T2` be the two diffuse samples, `T1` the
miscellaneous sample and `B` the interpolated atmospheric background:

```text
a = 2 * T0.a * T2.a * CloudIntensity.x
cloud = (T1.rgb + T0.rgb) / 2
rgb = cloud * a + B * (1 - a)
alpha = 1
```

The shader does not explicitly saturate `a`. Preserve the order of operations
and distinguish shader-register behavior from the final framebuffer conversion.
The selected original PS 1.1 path on the measured NVIDIA D3D9 device limits
positive pixel constants to one: passing intensity two gave the same result as
one. Keep this compatibility behavior explicit rather than inferring it from
the floating-point equation alone.

The second layer samples SKY_TEXTURE_MISC_2 at sampler zero, outputs its unmodified RGB and
`misc2.a * CloudIntensity.y`. The third layer outputs the unmodified
miscellaneous texture RGB and `misc.a * CloudIntensity.z`. Their eventual visual
contribution depends on the runtime blend states and layer ordering.

## Independent pixel fixtures

The private harness uses original vertex and pixel shaders, synthetic geometry,
controlled constants, point-sampled one-pixel textures and an A8R8G8B8 target
with sRGB conversion, fog render state, blending and alpha test disabled. It
does not run a recreated shader to generate these expected values.

With diffuse RGBA8 `(64,128,192,128)`, miscellaneous RGBA8 `(192,64,32,64)`, and
the recorded synthetic atmospheric parameters, the observed base RGB8 values
for intensity `0`, `0.5`, and `1` are respectively `(60,31,19)`, `(77,47,42)`,
and `(94,64,66)`; alpha is always 255. Intensity two reproduces intensity one.
Layer two alpha is `0`, `64`, `128`; layer three alpha is `0`, `32`, `64`.
The standalone layer RGB values remain equal to their selected source texture.

A second fixture uses parameters directly expressible by the renderer interface:
Rayleigh `(0.2,0.3,0.4)`, Mie `(0.1,0.2,0.3)`, anisotropy `0.1`, light towards
positive Z, eye at the origin, and quad vertices `(±1,±1,1)`. The original shader
then produces base RGB8 `(18,18,17)`, `(46,37,41)`, `(73,57,65)` for intensity
`0`, `0.5`, `1`. The in-scattering scale is one. Cloud layer two, with synthetic
RGBA8 `(64,128,192,128)`, blends over black to `(16,32,48)` at half intensity;
layer three, RGBA8 `(192,64,32,64)`, blends over black to `(24,8,4)`.

These tests establish arithmetic and channel selection. Animated UV sampling,
installed texture selection, camera-relative placement, blending, cloud-weight
transitions and matched gameplay captures remain separate validation requirements.
