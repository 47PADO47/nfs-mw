# Visual Treatment

Sources read: original PC 1.3 shader bytecode and runtime inspection (private),
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0, symbols only), and
[noclip.website](https://github.com/magcius/noclip.website/tree/master/src/NeedForSpeedMostWanted)
(MIT, reverse-engineered post-processing reference). No extracted assets or code are shipped.

## Inputs and order

The source is the scene before the HUD. Work in the original display-range colour
space, with scene values clamped to [0, 1]. Visual Treatment precedes optional modern bloom, tone mapping and anti-aliasing.

## Local luminance

Downsample scene luminance with four equal taps. The separable blur uses offsets
-3, -1.5, 1.5 and 3 pixels with weights 1/6, 1/3, 1/3 and 1/6; repeat horizontal
and vertical twice. Quarter resolution is an implementation choice; exact PC
target sizing remains unverified.

## Visual Treatment

[verified] Load `visuallook/default` from AttribSys. Read `ColourBloomTint`,
`Desaturation`, `BlackBloomIntensity`, `ColourBloomIntensity`, `BlackBloomCurve`
and `ColourBloomCurve`. Curves are four cubic Bezier ordinates from the y component
of each matrix row. Do not embed the authored curve or tint data in source.

Blur scene luminance, using weights (0.6125, 0.5154, 0.0721), without normalization.
Evaluate both curves at clamped blurred luminance. Desaturate the scene towards
its luminance. Multiply it by a blend between one and the black curve, then add
scene colour times tint, colour curve and colour intensity. Apply the installed
gain map's blue channel; the green channel is an optional additive vignette.
The ordinary look has no additive effect vignette; only blue gain is applied here.
Use the standard map with hash 0x968775E8, then 0x0CC087FE; without it use unit gain.
Sample the map with flipped vertical UVs. Detail-map additions are omitted.

The High setting applies the full profile. Low blends halfway with the source;
this is a port convenience, not a claim of PC Low-setting parity. Default Off
preserves the existing render. Pursuit/cinematic profiles and time-of-day blending
are not included. No temporal history or frame-rate-dependent state is introduced.

## Failure and lifetime

Missing or malformed look data disables grading with a log message. Missing gain map uses unit gain. Scene replacement clears the profile.
Allocate reusable quarter-size targets only for active effects; recreate on resize.
Backend capability checks hide unsupported controls and preserve user preferences.
