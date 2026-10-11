# Ground contact integration

- Source executable hash and reference revision are recorded in
  [the specification](../specs/ground-contact-response.md), written before implementation.
- Seven synthetic inputs were executed through the original PC single-plane
  response: centred, sliding, off-centre, tilted, elastic, stationary and separating.
- Source regressions compare original synthetic velocities, angular velocities,
  normal impulse and friction state. Ground queries additionally exercise deep
  penetration, per-surface friction and disabled-ground integration.
- Cloud support, car contacts and cargo remain on the shared fixed clock;
  full-state render-rate regressions are run after integration.
- Original cached world-face selection and live ground/wall comparisons remain
  open; collision rays are used to supply ground height and normal.
