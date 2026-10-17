# Original PC normal-mapped world materials

Status: verified simple unshadowed Shader Model 2 arithmetic and input declaration;
live sampling state and complete gameplay matching remain unverified. Written before
implementation from the retail PC executable and embedded WORLDNORMALMAP effect.
See [provenance](../provenance/world-normal-materials.md).

## Inputs and routing

World effect ID 3 selects this family. Its 60-byte declaration consists of position
FLOAT3 at 0, normal FLOAT3 at 12, BGRA D3DCOLOR at 24, UV0 FLOAT2 at 28, UV1 FLOAT2
at 36 and tangent FLOAT4 at 44. Tangent w supplies handedness; there is no stored
binormal. Decode this extension only for the verified declaration, never every
60-byte format: skinned formats have different attributes.

The selected technique samples diffuse, normal and specular textures. The solid
texture slots are respectively 0, 1 and 3. All samples use UV0 plus a shared texture
offset. UV1 is present but unused by this technique. Texture lookup failure or an
unsupported backend must retain an explicit legacy fallback, rather than inventing
normal or specular inputs. Preserve independent diffuse blend classification.

## Vertex stage

Let p, n and t be the local position, normal and tangent xyz. Construct
`b = cross(n,t) * tangent.w`. Do not normalize this basis again. Transform the
light and eye into local coordinates. For rigid poses, equivalent world-space
dot products preserve the authored magnitudes. Nonuniform scale equivalence has
not been established.

The tangent-space light is `normalize([dot(t,L), dot(b,L), dot(n,L)])`.
Normalize the local eye-minus-position vector, then project it onto the same
basis to form the tangent-space view vector. Clamp the ambient COLOR output
`ambient.rgb * ambient.a * vertex.rgb` into [0,1] before interpolation. Preserve
vertex alpha. Compute extinction E and in-scattering S at vertices using the
bounded atmospheric equations in [world routing](world-material-routing.md),
then interpolate them independently.

## Pixel stage

Decode and normalize `N = 2 * normalTexture.rgb - 1`. Normalize the interpolated
light L and view V. Let `d = dot(N,L)`, `lambert = clamp(d,0,1)`,
`R = 2*d*N - L`, and `highlight = pow(clamp(dot(R,V),0,1), specularPower)`.
The output before SDR saturation is:

`RGB = 2 * (diffuse.rgb * (ambientVertex + diffuseLight.rgb * lambert)
            + specularTexture.rgb * specularLight.rgb * highlight) * E + S`

`alpha = diffuse.a * vertex.a`

Clamp RGBA into [0,1] before source blending. All ambient, diffuse and specular
light receive extinction in this family, unlike the ordinary World shader.

## Runtime light constants

The common world shader updater supplies diffuse RGB and specular RGB from the
time-of-day lighting state. Ambient comes from the same state including its
intensity alpha. The original global specular power default is 11.0. Its setter
does not use per-group car LightMaterial records for effect ID 3; those setters
are restricted to the car and bone shader families. The selected pixel shader
uses specular RGB, not specular alpha.

## Scope and validation

Implement the simple unshadowed technique with the original-material profile
switch. Disabling that switch retains the legacy prelit result. Independent
original shader pixel fixtures must cover tangent handedness, normal direction,
specular texture/power, ambient intensity and atmospheric extinction.

Complete projected shadow filtering, animated texture offsets and
texture-frame redirection, exact runtime
sampler and culling state remain
separate gaps. Use static offset zero until authored animation is understood.
Reflective effect ID 1 and glossy window ID 5 have separate specifications and
native paths; their different equations must not replace this family.
The existing custom shadow/AO system is an enhancement, not original shadow
parity. The normal family participates in depth, caster geometry and dynamic
shadow receiving. Only direct diffuse and highlights receive visibility; ambient
and scatter bypass it. AO modulation remains separate. See
[surface receivers](world-surface-shadow-receivers.md).

The game caps each authored ambient and diffuse RGB channel at 1 before supplying shader
constants, with no lower clamp. This applies before Lambert multiplication;
clamping the resulting vertex COLOR alone is not equivalent for bright inputs.

Some installed normal-map geometry contains nonfinite authored tangents, including
indexed vertices. Upload preserves their exact bits. Finite fixture agreement
does not establish matching original rasterization for those degenerate inputs;
that remains a separate validation gap. No replacement tangent is invented.
