# Visual Treatment provenance

Date: 2026-10-11. Specification: [Visual Treatment](../specs/visual-treatment.md).

Original PC shader bytecode and runtime constants were examined privately. The
luminance extraction, four-tap blur and colour composition were described in the
specification before independently implementing Rust/WGSL. Authored curves and
tints are read from the user's installed AttribSys data, never redistributed.

References: dbalatoni13/nfsmw (CC0, symbols); magcius/noclip.website (MIT,
reverse-engineered post-processing). No source copied from either project.

Validation records belong in [the feature guide](../visual-treatment.md). Exact PC
dynamic exposure and time-of-day/pursuit look parity remain unverified. The
quarter-size targets are an implementation choice, not full engine parity.

## Extra Options reference parameters, 2026-10-16

The user's supplied values were checked against the installed Extra Options
configuration and matching plugin before implementation. Plugin SHA-256:
`32bc3ae6c3f36218ae507b666d8fb816c2263fdb7819498286bca11537ec8ff2`.
The installed hook writes RGB into ColourBloomTint, ColorBloomIntensity into
ColourBloomIntensity, its Saturation input directly into Desaturation, and
BlackBloomIntensity into the corresponding original attribute. The generated
original attribute layout corroborates this parameter routing. Attribute routing
alone does not establish the shader coefficient orientation; see the correction
below. The six values are user-provided tuning, not redistributed game data.

[ExOptsTeam/NFSMWExOpts](https://github.com/ExOptsTeam/NFSMWExOpts) (GPL-3.0) was
read only as a behavioral reference and compared with the installed hook.
No source was copied or ported. Parameter routing and persistence have synthetic
checks; these overrides do not establish full modded-game rendering parity.

## Source-colour coefficient correction, 2026-10-16

The initial override implementation inherited a reversed interpretation of
`Desaturation`. The default midpoint concealed the error; synthetic expectations
computed from that interpretation were insufficient evidence. Matched PC runtime
constant preparation supplies the raw attribute as the source-colour coefficient
and its complement as the luminance coefficient. Both the black-only and combined
original shader programs were executed with asymmetric values 0, 0.2, 0.8 and 1
and independently authored constant textures. Both retain more source colour as
the raw value increases. Public tests retain only synthetic inputs and measured
output pixels; no original shader program or extracted asset is redistributed.

Stock program SHA-256:
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
The preferred modded program has an identical treatment effect resource
(SHA-256 `28754287ff2a24c5fe461dd004425f6d0223ae69219749bfdd34c56e0240e3cf`).
Matching embedded programs establishes this arithmetic reference, not equivalence
of every active mod hook or complete runtime frame. The original-program fixture
run used D3D9 HAL, an unorm render target, explicit linear texture sampling and no
sRGB conversion. The corrected native interpolation is independently implemented
from the behavioral equation in the specification.
