# Original PC car material shading

Status: selected programmable car shader arithmetic verified from the shipped
CAR effect, runtime material setters and independent original-shader fixtures.
Gameplay captures and original projected-shadow/cube capture parity remain open.
Written before implementation; see [provenance](../provenance/car-material-shading.md).

## Families and inputs

Solid effect ID 4 selects CAR with position, normal, BGRA color and UV0 in its
36-byte declaration. Paint, glass, chrome, rims and ordinary car surfaces share
this family; their material ramps and diffuse textures distinguish them.

The unshadowed programmable technique uses a half-vector highlight and full
environment contribution. The modern projected-shadow techniques use a reflected
light highlight and half environment contribution. They are distinct arithmetic
families, not one equation with an optional visibility multiplier. Main-view
technique selection depends on shadow settings and object distance; non-main car
views select the unshadowed technique. Keep both explicit and preserve the custom
renderer as a separate default. Original projection/shadow filtering is outside
this batch; applying custom visibility to the modern family is an approximation.

## Material delivery

For each RGB ramp, `min = minScale * minColor` and
`range = maxScale * maxColor - min`. Diffuse alpha is separate:
`minAlpha = authoredMinAlpha`,
`rangeAlpha = (authoredMaxAlpha - authoredMinAlpha) * 0.25`.
Specular RGB min and range are multiplied by the blended time-of-day CarSpecScale.
The programmable path uses the authored environment ramp without the fixed-function
reduction. Both selected vertex programs read only the environment ramp's first
channel; uploaded green/blue channels do not affect these programs.
Global specular/environment power multipliers default to one.

## Vertex stage

Let N be the stored normal, P the position, V the normalized local eye-minus-P,
and L the first local light direction. Do not normalize N again. LocalEyePos is
the actual camera transformed by the runtime's rigid inverse: transpose the
model's linear 3x3 and apply it to camera-minus-model-origin. This helper does
not divide out model scale. A separate environment-context eye clamp is not
the CAR vertex shader input. Clone light directions are transformed by the
same transpose without normalization. Keep the original normal in local space;
normalizing a scaled world normal or replacing the transpose with a true affine
inverse changes these inputs. Transform the reflected local vector through the
model linear matrix for the native world-aligned cube map. Installed part/context
selection remains distinct from these verified matrix operations.

`f = max(dot(N,V), 0.01)` and `s = clamp(2 * vertex.r, 0, 1)`.
Sum three directional lights as
`light = sum(clamp(dot(N,Lk)+0.1,0,1) * Ck)`; no ambient term is added.
Compute `diff = DiffuseMin + DiffuseRange*f`,
`spec = SpecularMin + SpecularRange*pow(f,SpecularPower)`, and
`env = EnvmapMin + EnvmapRange*pow(f,EnvmapPower)`.
The original LIT instruction bounds its exponent to the D3D vertex instruction
range; ordinary authored powers are within it.

Vertex COLOR outputs are independently saturated before perspective interpolation:
`diffuseColor = clamp([s*light*diff.rgb, diff.a],0,1)` and
`specularColor = clamp([s*spec.rgb, s*env],0,1)`.
Vertex alpha is unused. The reflected view vector is `2*f*N-V`, transformed by
WorldView for the original cube map. The remake's world-aligned cube requires the
equivalent world-space vector; capture orientation remains separately validated.

The modern family passes N, L and V to the pixel stage. The unshadowed family
passes N and the vertex-normalized half-vector `normalize(V+L)`. The pixel shaders
do not renormalize any of these interpolated vectors.

## Pixel stage

The original D3D9 cube lookup filters within the selected face and clamps at its
edge; it does not blend neighboring faces. Choose the largest absolute direction
component, resolving exact ties in Z, Y, X order. The face order and orientation
are +X, -X, +Y, -Y, +Z, -Z with standard cube coordinates. The native original
families expose the same environment image as six array layers and sample its
selected face linearly with clamped UVs at level zero. No duplicate image or asset
is required. Custom glossy retains hardware seamless cube sampling. Current
runtime environment images have one mip; original mip/LOD policy and capture
contents remain separate gaps.

Let T be diffuse RGBA and C sampled cube RGB. The modern highlight is
`g = 2 * clamp(dot(2*dot(N,L)*N-L,V),0,1)^16`.
There is no additional front-facing-light step gate. Its shadow attenuation q
applies twice to the specular contribution:

`rgb = (diffuseColor.rgb*T.rgb + g*diffuseColor.a*specularColor.rgb*q)*q
       + C*specularColor.a*diffuseColor.a*0.5`.

The unshadowed highlight is `g = 2 * clamp(dot(N,H),0,1)^8` and
`rgb = diffuseColor.rgb*T.rgb + g*diffuseColor.a*specularColor.rgb
       + C*specularColor.a*diffuseColor.a`.
Both output `alpha = T.a*diffuseColor.a`. The native alpha test and source-alpha RGB composition use
that composed alpha. Exact original framebuffer alpha blend state remains
unverified; the synthetic D3D9 blend setup alone does not establish it. The legacy custom family retains its existing
texture-only alpha test. Original cutouts skip the texture-only depth prepass;
the color pass writes their correctly tested depth. Saturate RGB/alpha at the original
SDR boundary before source-alpha blending, including on floating intermediates.
The native HDR scene intermediate preserves the original composed alpha. Presented
SDR targets and exported captures deliberately remain opaque: their write mask and
capture contract keep alpha at one. Shader-output alpha is therefore validated by
reading the floating scene intermediate before presentation, separately from the
published SDR RGB and blending checks. Custom opaque/cutout intermediates retain
their existing alpha of one.
The selected CAR shaders have no atmospheric calculation and their passes disable
fog. Do not add the world fog or custom clearcoat/AO/skylight to these original
families. Custom rendering retains its existing behavior.

## Validation and remaining scope

Independent fixtures execute original vertex/pixel programs with synthetic paint,
glass and general ramps, controlled light matrices, face-colored cube textures,
vertex color/alpha, nonunit normals, varying normals, view rotation and shadow
textures. Numeric expectations must come from that independent execution.
Keep renderer uniform packing, source-alpha composition and native high-FPS
frame delivery separately tested. The light rig and material scale update by elapsed
time; no per-frame intensity accumulation is introduced.

Original projected maps/filtering, exact runtime sampler/culling and complete cube
capture orientation/content remain gaps. This implementation does not add unrelated
vinyl, damage, headlight or texture-compositing features. Original effect ID 2's skinned deformation is not reconstructed; groups routed
through their light material currently retain a rigid-mesh fallback.
