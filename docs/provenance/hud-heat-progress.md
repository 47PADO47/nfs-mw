# Original minimap heat progress

- Spec: [hud-pursuit-state.md](../specs/hud-pursuit-state.md), heat-progress section.
- Source read: CC0 `dbalatoni13/nfsmw`, `Frontend/HUD/FeHeatMeter.cpp`; read for
  understanding, no source text copied. Its unsolved second-mask branch differs
  below half progress, so the PC instructions resolve that branch.
- **[binary]** Original PC constructor, update and multi-image rotation setter
  confirm the two registered mask IDs, separate first/second-half mapping,
  multiplier/icon scripts and degree-valued mask rotation. The setter marks the
  authored multi-image state dirty; the existing runtime setter supplies that
  state directly. Detailed executable identity and addresses remain private.
- **[verified]** Installed HUD assets supply the arc group, masks and their pivots.
  The installed regression checks visibility, mask states, scripts and the
  multiplier over both halves and an integer crossing.
- Implemented in `nfsmw::hud::police::heat`; no gameplay growth values changed.
  Police-to-wave progression is checked independently of the HUD. Full matched
  gameplay, frame screenshots and heat-meter visual parity remain unverified.
