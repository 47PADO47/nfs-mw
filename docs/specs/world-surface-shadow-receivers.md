# World surface shadow receivers

The selected original modern normal, reflective-road and window programs apply
visibility to direct illumination and highlights. Ambient, reflected images and
atmospheric in-scattering remain visible in shadow. See the corresponding material
specifications for the independently measured unshadowed arithmetic.

Normal and reflective materials use:
`2 * (texture * ambient + (texture * diffuse * Lambert + highlight) * q
      + reflection) * extinction + scatter`.

Windows use:
`2 * (texture * ambient + (texture * diffuse + 2 * alpha * highlight) * q)
 + alpha * reflection + scatter`.

The receiver must use world position and the rasterized geometric plane for bias,
independent of the normal map or authored smooth normal. This prevents sloping
roads with flat lighting normals from incorrectly shadowing themselves.

The existing custom cascade/contact options supply q for these material families
as well as common world materials. This is an explicit custom producer; connecting
it does not reconstruct the original projected shadow camera or filtering. Disabled
shadows return q=1 exactly. Reflection captures omit camera-dependent visibility.
Original SDR saturation, opacity and cutout rules remain applied after shading.

Validation must cover all three receivers, off-camera and moving casters,
resolution changes, disabled round trips and ambient/reflection preservation on
both native graphics APIs. Original producer parity requires separate evidence.
