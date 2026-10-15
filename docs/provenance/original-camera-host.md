# Original camera host timing provenance

- Original PC 1.3 executable, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Main scheduler dispatch inspected locally: scheduler elapsed time is forwarded
  unchanged to camera actions and movers. World simulation dilation is separate.
- Native follow and clearance contracts are recorded in the companion original
  camera specifications/provenance; no executable bytes are included here.
- The fixed 60 Hz camera clock, timestamped physics inputs and render-offset
  interpolation are original host adaptations. They preserve the checked native
  60 FPS update cadence independently of presentation FPS; they are not claimed
  to reproduce the original program's update-count artifacts at higher FPS.
- Synthetic replay checks cover 5/30/60/144/360 FPS, slowed simulation, long
  frames, pause, body attachment, view cuts and look-back. These checks are not
  original-game visual captures or evidence for deferred cinematic tracks.
- Implementation follows [the host spec](../specs/original-camera-host.md).

- Custom camera clearance shares the geometry and retained 60 Hz update cadence,
  while its final stateless safety pass and vertical-FOV adapter are host policies.
  Orbit controls, custom follow dynamics and custom optics are not native parity.
