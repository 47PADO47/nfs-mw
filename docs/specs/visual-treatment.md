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
Evaluate both curves at clamped blurred luminance. The historically named
`Desaturation` attribute is the source-colour coefficient: for source `c`, luminance
`l = dot(c, weights)` and attribute `d`, the base is `d*c + (1-d)*l`. Thus zero
is grayscale and one retains source colour. Both original shader families agree;
do not interpret `d` as the amount of grayscale. Multiply the base by a blend
between one and the black curve, then add
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

## Supplied mod reference preset

A separate Reference mode applies the user's Extra Options override at full
strength. Off, Subtle (half stock profile) and Original (full installed profile)
retain their profile and blend choices. All modes use the verified source-colour
coefficient above. The reference values are explicit user settings:
RGB tint (0.68, 0.62, 0.03), colour intensity 0.75, source-colour coefficient 0.80
and black intensity 0.40. The mod writes its "Saturation" input directly to the
original `Desaturation` attribute. Preserve that raw value: 0.80 retains 80%
source colour and adds 20% luminance before the bloom and gain terms.
EnableVisualTreatmentOverride selects replacement parameters, not a new blend
amount. Installed luminance weights, curves and gain map remain the source for
all other parameters.

The renderer accepts an optional generic parameter override independent of the
profile blend strength. Changing the preset or strength rebuilds only the post
prefix and preserves the current profile and adapter-selected luminance format.
A missing/invalid installed profile still bypasses treatment. Save/restore and
renderer capability resolution preserve the requested Reference choice; a
backend without scene treatment reports it inactive. Reference reproduces these
mod parameter overrides, not all modded-game lighting or final-frame effects.

## Asymmetric original-program regression

Original PC black-only and combined treatment programs were executed with
synthetic source RGB8 (153,102,51), equal luminance weights and neutral bloom/gain.
Raw coefficients 0, 0.2, 0.8 and 1 produce RGB8 (102,102,102), (112,102,92),
(143,102,61) and (153,102,51) in both families. A second set exercises the
supplied mod parameters with constant synthetic curve samples 0.4 and 0.2.
Its 0.8 result is (124,87,47). These fixtures establish coefficient orientation;
they do not establish dynamic look selection, exact blur target sizing or the
complete modded-game frame.
