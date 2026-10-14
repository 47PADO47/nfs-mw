# Tyre and ordinary transmission audio provenance

Implemented from the [behavioral specification](../specs/car-audio-tyres-transmission.md).
Reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0;
read as behavioral evidence, with no copied implementation or assets.

- CARSFX_Skids.cpp: constructor76, mixer outputs91–104, class parameters113–155.
- SFXCTL_Wheel.cpp: sensitivities/load ramp30–35, signed wheel normalization89–100,
  side terrain retention and blown-tire replacement150–174.
- Physics/Behaviors/SoundCar.cpp: slip deadzone162–178 and negative lateral skid.
- Physics/Behaviors/Chassis.cpp: understeer50–59 and oversteer95–103, signed vehicle
  speed/rigid-body speed gates, signed wheel-skid averages and actual front steer.
- CARSFX_Engine.cpp: transmission scaler15 at17, eligibility347–349,
  per-update parameter producers386–400. ENGINES_AEMS2.h87–97 and MAIN_AEMS.h125–146
  establish interface order. NFSUG_CarsSFXLoadData.cpp52–55 establishes bounded
  skid-bank audio-level selection; no matching PC upgrade producer established.
- Matching PC CAR_TRANNY setter bounds and decoded pitch are supplied by the
  independent [environment audit](car-audio-environment.md): magnitude0..10000,
  volume0..32787, azimuth0..65536, pitch-offset−16383..16383, lowpass0..25000,
  wet/dry0..32767. The signed producer projection is retained before those bounds.
- Installed ABKC structural probe (2026-10-14): four FX_SKID banks, 22 words,
  seven players; CAR_TRANNY nine words/three players; same-bank kind-zero sounds,
  supported interpreter forms. Original executable SHA256
  80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c.
- Original passive driving capture (2026-10-14): conditional skid/transmission
  player observations after quarantining reused pointers; no audible-equivalence
  claim. Private evidence remains outside the repository.
- Matched PC car packets independently establish fractional factor scale:
  3,561 joined packets, understeer maximum0.636829 and oversteer maximum0.532310;
  direct truncation gives0 throughout. These packets are unaffected by the audio
  player-pointer cache defect. They do not prove the complete source calculation
  against PC wheel-angle/state history.

Synthetic coverage: signed slip/deadzone/load/airborne vectors, truncation and
clamping, explicit understeer/oversteer and surface inputs, transmission reverse
sign and shifting state. Installed tests require NFSMW_GAME_DIR and run actual
modules into headless PCM; no missing-install pass. Original timing, RNG, bank
upgrade selection, environmental DSP and absolute calibration remain unresolved.
Render-rate tests compare fixed telemetry at5/60/144/360FPS and44.1/48kHz,
steady-state PCM energy, voice starts/capacity and post-drop silence. Tests are
implemented but not claimed passed before recorded local validation.
