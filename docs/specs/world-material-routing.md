# Original PC world material routing

Status: verified static dispatch and shader arithmetic; runtime visual matching remains required.

Sources: the retail PC 1.3 executable and its embedded D3DX effects (proprietary,
read for behavior only); [noclip.website's independent MW renderer](https://github.com/magcius/noclip.website/tree/master/src/NeedForSpeedMostWanted)
(MIT, read-only corroboration). This specification was written before its implementation.
No shader bytecode, extracted textures or disassembly belongs in source control.

## Effect dispatch

The solid's shading-group effect ID is an index into the executable's shader
descriptor table. It is not an alpha-blending mode. The verified table has
144-byte entries; the constructor selects by index and the resource loader uses
that same entry. Relevant IDs are:

| Effect ID | Runtime shader | Embedded effect | Scope |
|---|---|---|---|
| 0 | WorldShader | WORLD | Ordinary textured scenery |
| 1 | WorldReflectShader | WORLDREFLECT | Reflective, normal-mapped scenery |
| 2 | WorldBoneShader | WORLDBONE | Skinned geometry |
| 3 | WorldNormalMap | WORLDNORMALMAP | Normal-mapped scenery |
| 4 | CarShader | CAR | Vehicle surfaces |
| 5 | GlossyWindow | GLOSSYWINDOW | Glossy window surfaces |
| 6 | billboardshader | TREE | Foliage / tree surfaces |
| 7 | WorldMinShader | WORLDMIN | Minimal world pass |
| 8 | WorldNoFogShader | WORLDNOFOG | Authored unfogged textured surfaces |
| 17 | WorldPrelitShader | WORLDPRELIT | Unfogged, unlit textured color |
| 19 | skyshader | SKYBOX | Multilayer atmospheric sky |

Shader selection must happen per group, preserving mixed-effect solids. The
existing vehicle light-material overrides have precedence and remain unchanged.
Texture alpha usage still chooses cutout/blended/opaque pipeline state separately.
The world-family cutout rejects combined alpha at or below `0.375`, following
the independent noclip.website MW renderer's `ALPHA_TEST` reference. This is
not an exact retail render-state reconstruction. A `0.5` cutoff rejects entire
installed walls authored with vertex alpha `127/255`, even with opaque texels.
Texture holes must remain cut out; increasing vertex alpha instead would also
change authored blending.
Unknown effects retain an explicit fallback and are not reported as matched.

## Color semantics

Let `T` be sampled diffuse RGBA, `V` the interpolated vertex RGBA, `A` the
ambient color multiplied by its effective runtime intensity, `D` the diffuse light color,
`L = clamp(dot(normal, localLightVector), 0, 1)`, `E` the atmospheric extinction
per channel and `S` the in-scattered light. Multiplications below are componentwise.

The original vertex shaders compute their lighting and atmospheric terms at
vertices; perspective interpolation supplies those terms to the pixel shaders.
Distance is `clamp(length(localEye - position) * FogValue, 0, 2500)`.
Extinction is `min(exp(-(Br + Bm) * distance), 1)` per channel; in-scattering is
clamped to `[0, 1]` before interpolation. Lambert illumination is clamped to
`[0, 1]`. The source vertex normals enter the dot product directly, rather than
being normalized again in the shader. Shader color arithmetic and the final
framebuffer clamp are distinct; an HDR enhancement may retain a different range.
The D3D9 vertex COLOR outputs clamp ambient and direct contributions individually
to `[0, 1]` before interpolation. This also applies to pass-through vertex color
of the no-fog/prelit families; normalized color bytes already satisfy it.
The following describes the simple unshadowed world technique. Original shadow
techniques additionally attenuate the direct contribution using the projected
shadow map; replacing that map with the custom shadow system is an enhancement.

| Family | RGB before final framebuffer clamp | Alpha |
|---|---|---|
| World | `2 * T.rgb * (A * V.rgb * E + D * L) + S` | `T.a * V.a` |
| Tree | `2 * T.rgb * (A * V.rgb + D * L * E) + S` | `T.a` |
| WorldNoFog | `2 * T.rgb * V.rgb` | `T.a * V.a` |
| WorldPrelit | `2 * T.rgb * V.rgb` | `2 * T.a * V.a` |

World and Tree differ in which contribution receives extinction. In particular,
Tree ignores vertex alpha. WorldNoFog and WorldPrelit neither compute fog nor
receive ambient/directional lighting. Do not infer the family from a texture or
solid name, or apply the same prelit formula to every world group.

The shader's doubled arithmetic is not an instruction to halve runtime source
vertex colors again. GPU input colors are normalized D3DCOLOR bytes with BGRA
storage. Preserve the effective intensity component (the inspected PC lighting update sets it to four); a boolean lighting-enabled
flag is a separate value.

## Sky and deferred families

Effect 19 uses a second UV pair and multiple texture samples, cloud parameters
and atmospheric background terms. It is not a single unfogged prelit sample.
Normal family 3 uses the tangent and texture inputs described in
[normal-mapped world materials](world-normal-materials.md). Reflective family 1
uses its distinct [normal/specular material](world-reflect-materials.md), including
dry time/cloud lighting. Family 5 uses the authored global reflection and separate
[window shader](world-window-materials.md). Unsupported backends or missing
required inputs retain explicit single-diffuse fallbacks. Wet-road scene capture
remains incomplete.

The preferred modded reference and stock PC executable have byte-identical copies
of all 31 embedded effects. This establishes a common shader source, not identical
runtime constants or active mod hooks. Changes which require mod-specific assets
must not be required to render stock installations.

## Validation

- Mixed-group synthetic solids must route every verified family independently.
- Original versus legacy fallback selection must preserve texture and alpha mode.
- Shader fixtures must isolate vertex alpha, light direction, authored ambient
  intensity and extinction. Tree alpha must remain unchanged when vertex alpha changes.
- No-fog/prelit must be invariant under atmospheric and dynamic-light changes.
- Compare matched scenes at stable camera/location/settings and record reference
  identity. Static arithmetic tests alone do not establish overall visual parity.
- Lighting and atmosphere transitions use elapsed time, never a per-frame step.

## SDR composition boundary

Independent original D3D9 fixtures confirm UNORM RGB/alpha saturation before
source-alpha blending, including doubled prelit alpha. Reconstruct this SDR
boundary explicitly for these original families, even when a modern backend
uses a floating-point intermediate. Modern HDR/post choices remain enhancements.
Vertex COLOR outputs also saturate before perspective interpolation; world
normals are used with their stored magnitude, without an added normalization.

The game caps each authored ambient and diffuse RGB channel at 1 before supplying shader
constants, with no lower clamp. This applies before Lambert multiplication;
clamping the resulting vertex COLOR alone is not equivalent for bright inputs.
