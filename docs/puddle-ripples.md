# Road puddles and rain ripples

## Existing native path

Two interpolated world-XY noise fields supply the pool mask. Specular red,
diffuse alpha and vertex alpha describe reflection eligibility. None encodes
kerb distance. Dampness integrates elapsed time; above 45% saturation a second
reservoir approaches a rain-intensity-dependent target. After rain the reservoir
drains while asphalt remains damp. The receiver already darkens porous colour,
flattens tangent normals and blends reflection roughness from 0.32 to 0.07.
RT, screen-space and environment reflection directions share this normal.
The baseline RT hit returns one sharp sample even on rough film.
The original ripple approximation combines two travelling sine waves.

## Enhanced option

Enable Enhanced wet surfaces, then F8 → Materials → Advanced wet surfaces →
Puddle contours and rain rings (default off), or use `set puddle_ripples on`.
Converging interpolated geometric normals lower the pooling threshold in dips.
Convex crowns shed water first. Flat triangles retain the noise distribution.
The bounded curvature bias approximates local shape without road topology or
drainage data. Two jittered world grids generate expanding ring normals. Their
amplitude follows current rainfall and fades as the rings become subpixel.
RT uses the rough probe for unpooled film and progressively admits sharp
geometry hits inside pools. Screen-space hits keep their pool-dependent filter.

Estimated incremental work includes normal derivatives and two analytic drop
evaluations on nearby rainy pool pixels. No additional rays, textures or passes.
Visual design reference: Sébastien Lagarde,
[Water drop 2b](https://seblagarde.wordpress.com/2013/01/03/water-drop-2b-dynamic-rain-and-its-effects/).
The shader is independently written.

## In-game comparison

RTX 3070, native Vulkan release, 3840×2160, fixed highway camera
(`goto 2423 3415 3 45 -24`), no traffic. Both cases used enhanced wet surfaces,
RT road reflections, high sun shadows with RT soft shadows, overcast skies,
100% rain and full accumulated wetness. At native resolution, individual rings
are visible near the camera in the pooled reflections. Distant film stays smooth.

| Median duration (ms) | Off, two runs | On, two runs |
| --- | --- | --- |
| GPU base render, six-second timestamp trace | 15.44, 15.50 | 15.04, 15.14 |
| Frame time, twelve-second benchmark | 15.65, 15.70 | 15.31, 15.41 |

Run order was off/on/on/off. The whole enhancement averaged 0.38 ms less GPU
time in this view. Coverage changes and skipping mirror rays on rough film
affect this comparison. The timings do not isolate the cost of evaluating rings.
