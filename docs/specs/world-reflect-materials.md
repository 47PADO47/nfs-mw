# Original reflective world surfaces

Status: selected unshadowed modern shader behavior measured with independent
synthetic original-program fixtures. Full gameplay reflection capture remains
separate from material arithmetic. Specification precedes implementation; see
[provenance](../provenance/world-reflect-materials.md).

## Inputs

Effect ID 1 uses the tangent declaration described in
[normal world materials](world-normal-materials.md). Preserve position, raw
normal, tangent and handedness, vertex RGBA and UV0. UV1 is unused by the selected
modern shader. The shader receives diffuse, normal and specular 2D textures plus
a separate reflected 2D scene texture. This reflection is screen projected;
substituting an environment cube does not reproduce the original effect.

All three surface samples use `UV0 + textureOffset.xy`. The reflected sample uses
`(clip.x / clip.w * 0.5 + 0.5, -clip.y / clip.w * 0.5 + 0.5)`.
The reflection coordinate derives from the actual clip position and is
interpolated homogeneously before division. The surface normal does not distort
this coordinate. Normal and specular alpha are unused. Reflection texture alpha
is unused. Runtime sampler filtering, mip bias and texture animation are not
established by these arithmetic fixtures.

## Vertex calculations

Construct `B = cross(N,T.xyz) * T.w` without normalizing the authored basis.
Project the light into the tangent basis and normalize the resulting vector.
Normalize the eye-minus-position vector before projecting it into that basis.
Interpolate both vectors; normalize them again in the pixel stage.

Clamp `A.rgb = ambient.rgb * ambient.a * vertex.rgb` before COLOR interpolation.
Preserve and saturate vertex alpha in `A.a`. Calculate atmospheric extinction E
and in-scattering S per vertex using the bounded equations in
[world routing](world-material-routing.md), then interpolate them separately.
Nonuniform model transform equivalence remains unverified.

## Modern pixel calculations

Let D be the diffuse sample, M the specular sample, F the reflected sample and
`n = normalize(2 * normalSample.rgb - 1)`. Let l and v be the normalized
interpolated tangent-space light and view vectors.

`d = dot(n,l)`

`h = pow(clamp(dot(2*d*n-l,v),0,1), specularPower)`

`base = D.rgb * (A.rgb + diffuseLight.rgb * clamp(d,0,1))`

`highlight = 2 * M.rgb * specularLight.rgb * h`

`reflection = F.rgb * (2*M.r) * D.a * A.a * reflectionScale`

`output.rgb = 2 * (base + highlight + reflection) * E + S`

`output.a = 1`

The reflective material updater supplies highlight RGB as the authored time-of-day
specular color multiplied by `1 - CloudIntensity`. This attenuation applies only
to this family, independently of the reflection scale. The reflection scale is
the original specular-light alpha, supplied by the view's RoadDampness state.
The additional specular-mask doubling distinguishes this family from
the ordinary normal-mapped shader. The reflected red mask, diffuse alpha and
vertex alpha multiply reflection strength; they do not make this output
transparent. Preserve the original SDR saturation before post-processing and
source blending, including when a modern post chain uses a floating-point
intermediate. Other material families retain their existing behavior.

## Wet-road scope

The original weather update raises RoadDampness while raining and lowers it
after rain stops; settled dry state is zero. Its inspected step is per weather
update, so reproducing it once per rendered frame would make it frame-rate
dependent. A wet-road implementation needs the original weather cadence and
history, plus the reflected 2D scene producer and visibility policy.

The native world supplies the separate [projected road capture](wet-road-reflections.md)
and elapsed-time dampness history. An explicit material texture still takes
precedence; absent/inactive captures bind black. Original world materials must
be enabled. Dry highlight RGB still follows time and cloud cover independently.

## Shadowed and legacy families

The inspected modern basic shadow technique applies one visibility factor to
direct diffuse and highlight. Ambient and reflection bypass that factor. All
three contributions still receive atmospheric extinction. Original projected
shadow sampling and higher shadow techniques remain separate implementation gaps.

The legacy no-shadow shader differs materially: no shader atmosphere; normal and
specular use unshifted UV0; the normal is not normalized in the pixel stage; the
half-vector highlight has exponent four; output alpha is diffuse alpha times
vertex alpha. It also forms the tangent-space half-vector differently. The
modern implementation is not a claim of legacy technique parity.

## Validation boundary

Independent fixtures cover reflection contribution and mask channels, diffuse
and vertex alpha, reflection alpha, specular power, tangent handedness, non-unit
bases, eye displacement, fog, UV offsets and homogeneous reflection projection.
These isolate original shader behavior using synthetic inputs. Matching them
does not verify the original scene-capture contents, visibility, refresh cadence,
reflection-camera transforms, runtime sampler state or full-scene appearance.
