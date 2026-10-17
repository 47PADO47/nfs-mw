# Original world window materials

Status: original GlossyWindow effect ID 5 vertex/pixel arithmetic and named
reflection binding inspected in the matched PC executable. Independent original
shader fixtures validate the implementation separately. Original projected
shadows, sampler state and gameplay composition remain separate checks.
See [provenance](../provenance/world-window-materials.md).

## Inputs and binding

This family uses ordinary position, raw normal, vertex BGRA and UV0. It needs no
tangent or secondary UV channel. Diffuse texture T comes from the solid group.
The second image R is the globally resident authored `WINDOWREFLECTION` texture
in `GLOBAL/INGAMEA.BUN`, selected by name hash. It is a 256 by 128 two-dimensional
image with five mip levels, not a dynamic cube or screen capture.

The shader takes the shared time-of-day diffuse and ambient RGBA, sun direction,
local camera, model/view transforms and scattering constants. Car material
ramps, CarSpecScale, normal/specular textures and the car lighting rig do not
participate. Declared SpecularColour/SpecularPower are unused by the selected
programs. There is no TextureOffset input in this effect.

## Vertex stage

Let N be the stored normal, L the direction toward the sun in the same space,
and V the normalized camera-minus-position. Do not normalize N. Compute
`lambert = clamp(dot(N,L),0,1)` and
`highlight = clamp(dot(2*dot(N,L)*N-L,V),0,1)^10`.
The reflection uses the raw dot product before the Lambert clamp.

Use the original world scattering calculation: distance times FogValue is
clamped to 0 through 2500, E is component-wise extinction capped at one, and S is
in-scattered light clamped to zero through one. The phase cosine uses the
normalized camera-minus-position direction and the sun direction. Evaluate all
these terms at vertices, before perspective interpolation.

The D3D COLOR outputs saturate independently:
`D = clamp(DiffuseColour.rgb*lambert,0,1)` and
`A = clamp(AmbientColour.rgb*AmbientColour.a*vertexRGB*E,0,1)`.
Direct lighting is not multiplied by E. Highlight and S travel as texture
coordinates; neither is recomputed per pixel. Vertex alpha does not affect the
resulting RGB or output alpha.

Diffuse UV is UV0. The authored reflection image is sampled at
`reflectionUV = (0.5*u + 0.5*dot(N,WorldViewRow0.xyz), 0.5*v)`.
Equivalently, for the current world-space renderer, transform N without
normalization and dot it with the camera-right row of the view matrix. This
preserves the view-dependent slide of the authored reflection. Original
nonrigid local camera/light context behavior remains outside these rigid
scenery fixtures.

## Pixel stage and scope

For an unshadowed or fully lit selected programmable pass:
`rgb = 2*T.rgb*(D+A) + 4*T.a*highlight + T.a*R.rgb + S`.
`alpha = T.a`.

The base texture alpha is both gloss mask and output opacity; vertex alpha does
not modulate it. Output RGB/alpha saturates at the original SDR boundary before
blending. Opaque and cutout intermediates retain composed shader alpha while the
renderer retains its separate opaque SDR output/capture contract.

Shadowed programs apply visibility q to direct light and the vertex highlight:
`rgb = 2*((T.rgb*D + 2*T.a*highlight)*q + T.rgb*A) + T.a*R.rgb + S`.
The disabled/fully lit reconstruction uses q=1. Dynamic visibility is supplied
by the selected shadow system; original projected filtering/classification
remains partial. Custom AO, clearcoat,
car sky lighting and car materials do not enter this shader.

The native backend uses an explicit material pair with shared lighting globals.
A backend unable to create the window material returns unsupported so the caller
can retain its documented legacy fallback. Original sampler filtering, address
and mip policy, exact blend/cull selection and installed gameplay composition
remain independently tracked; shader arithmetic alone does not establish them.
