# Menu floor reflections

**[observed]** The supplied original-game safe-house screenshot shows the current
car reflected below the floor. The baked mirrored room alone cannot show a
changing car. Installed geometry inspection identifies the floor material on
`BACKDROP_CRIB`: its indexed triangles are on world Z zero within one micrometre.
Its diffuse alpha is floor coverage, not reflection strength. The renderer must
preserve the authored triangles, UVs, vertex colours and blended drawing order.

The game opts in only when every group using that floor texture is planar. A
replacement room that does not satisfy this rule retains its normal floor.
Other stages remain unchanged until their floor contract is established.

## Capture and composition

Mirror the current camera across Z zero, including the eye position. Capture
only actual car parts, with their normal transforms and materials. Clip source
fragments below the floor plane. Reverse winding once for the mirrored view;
the existing projected road capture has a separate winding rule.

The capture clears to transparent black and stores premultiplied colour and
coverage. Opaque and alpha-tested fragments cover the pixel fully; glass keeps
its source-over coverage. It has no UI, temporal history or post processing.

For floor colour `F`, floor alpha `A`, baked room `B`, and premultiplied car
capture `(C, Ac)`, the desired pixel is:

```
F*A + (C + B*(1-Ac))*(1-A)
```

The existing straight-alpha floor pass can produce this with
`alpha = A + Ac*(1-A)` and `rgb = (F*A + C*(1-A))/alpha`, guarding zero alpha.
An empty capture preserves the previous floor. Opaque floor texels hide the car.
The floor geometry bounds the reflection; no estimated rectangular mask is used.
Sample through the capture's unjittered projection, with a clamp sampler.

Refresh after camera/car updates and release bindings on close or restore.
The reflected car uses the authored material light rig; direct shadow-map
visibility in this reduced capture is not a claim of original-game parity.
Local environment cubes remain independent and exclude the car.

## Evidence and limits

The screenshot and geometry/alpha inspection are private local evidence.
This is a new implementation from the observed contract, not ported original
renderer code. GPU mask, coverage, clipping and winding checks and captured
safe-house frames are required before declaring the implementation verified.
