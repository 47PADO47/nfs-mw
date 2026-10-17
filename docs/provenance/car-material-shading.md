# Car material shading

- **Spec:** [Original PC car material shading](../specs/car-material-shading.md).
- **Date:** 2026-10-16; specification written before implementation.
- **Sources:** retail PC executable shader descriptor, embedded CAR effect,
  material constant updater, model/camera/lighting uploads and technique selection
  (proprietary; behavior inspection only, no code or shader bytecode copied).
  Earlier [car assembly](car-assembly.md) research provided the asset material layout.
- **Identity:** stock executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  CAR effect SHA-256
  `4beb55069eff9394a1fe932b75073a0d0bf7775245654c9fafcb288b78dd6671`.
  The prior stock/modded comparison found all 31 embedded effects identical;
  that does not establish identical active hooks or runtime constants.
- **Static checks:** descriptor ID 4 and 36-byte declaration; compiled effect's
  eight technique annotations and explicit technique/pass shader records;
  modern shadow levels 1-5 and unshadowed level 0; raw local camera upload;
  independent vertex COLOR clamping; quarter diffuse-alpha range; blended
  CarSpecScale and first-channel environment ramp.
- **Behavioral checks:** private D3D9 harness executes the original programs using
  synthetic geometry/textures and authored-style material constants. Its 396
  fixtures distinguish both highlight families, environment gains, varying vertex
  normals, vertex alpha independence and rotated/scaled model inputs. These fixtures validate shader arithmetic,
  not installed gameplay composition or original projected shadow reconstruction.
- **Remaining:** runtime blend/cull/sampler state coverage, projected shadows,
  posed-part shared light-context selection and complete original cube capture
  orientation/content.

The original framebuffer alpha blend state remains unverified. Original opaque
and cutout shader alpha and source-alpha RGB composition are tested separately.

Independent cube-sampler checks execute the original CAR programs with one-texel
and eight-texel synthetic faces, both point and linear filtering, all six axes and
edge/tie directions. All 240 measured samples agree: filtering remains within the
selected face, with exact major-axis ties choosing Z before Y before X. This
justifies the native original-family array-view lookup while retaining the
custom family's hardware cube lookup. Original runtime mip/LOD and capture policy
are not established by these synthetic fixtures.
