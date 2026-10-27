# Player driving parity

- Date: 2026-10-25.
- Spec: [player driving corrections](../specs/player-driving-parity.md).
- Sources: original PC v1.3 executable, static Ghidra inspection and isolated
  execution of unchanged machine code with synthetic interfaces; the prior
  [vehicle audit](vehicle-decomp-audit.md). Original proprietary code is a
  behavioral reference only. No executable bytes or decompiled code are included.
- Reference identity: selected memory-patched executable SHA-256
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`;
  existing research executable
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  A complete file comparison finds one differing PE characteristics byte:
  the large-address-aware flag. All executable sections are identical.
- Steering evidence: the maximum-steering routine at `0068e9e0` loads descriptor
  `008f8f58`, whose six points begin at `008f8ae4`. The rate routine at
  `0068ec70` separately loads descriptor `008f8f94`, pointing to `008f8b64`.
  Instruction/data inspection establishes both consumers, beyond table presence.
  The selected executable supplies 31 isolated original-code vectors, including
  partial/full/negative history, pad, both wheel modes, tuning and persistence.
- Implementation: correct the human steering range and PC history-remap tables;
  preserve the rate tables/algorithm and AI early return. Align calibration construction with
  the gameplay tire/jump/wall-steering profile.
- Audit corrections: the original loaded-tire routine applies low-speed
  dynamic-friction division only in the sliding branch. The existing wheel
  integration already limits applied lateral force to quarter mass times lateral
  speed divided by timestep. Neither old suspicion justifies changing physics.
  Ordinary impacts no longer puncture tires; an existing regression covers this.
- Limits: isolated vectors do not execute the loaded mod stack or establish
  whole driving trajectories. Configured WidescreenFix simulation rate and
  XtendedInput deadzones must be distinguished from observed runtime behavior.
  Expanded sliding-tire triage compares original execution to a Python
  transcription, not direct Rust execution; it is not a Rust parity pass.

## PC partial-stick curve and runtime timing

- The PC human-steering routine directly selects descriptor `008f8ee0`, whose
  21 points begin at `008f8b80`. This is the consumer, not an inferred default
  from the presence of several curves. Forty-three isolated selected-executable
  samples and a separate original rate-helper vector establish numeric outputs.
- A passive user-driven session confirms the same loaded points and unchanged
  guarded human/outer-steering, tuning-getter and physics-frame code. Observed
  raw-to-remembered values match the selected curve. The former generic medium
  curve was a genuine mismatch; only the PC human history curve changes.
- The selected modded session advances active physics at 144 updates per real
  second. Full Speedbreaker reduces simulation delta from 1/144 to 1/576 while
  retaining that real cadence. Histories retain 33 input and 9 rate samples.
  This does not justify changing the global remake/AI clock to monitor refresh.
- Input records include actual controls, body velocity/orientation, gear,
  source identity and all seven returned tuning values. The session spans two
  source identities with a loading/time-reset gap; segment timing by identity and
  monotonic simulation clock, rather than dividing all events by total wall time.
- Force samples at human steering are not post-step forces: tire begin-frame
  clears them before this point. Current-step force comparison requires a later
  wheel-force callback. No tire-force or full-route parity pass is claimed.
- A second passive capture observes the completed wheel-force callback, with
  owner-validated body snapshots and four current tire outputs. Summed tire
  world forces agree with the body force-accumulator change within 0.023 N per
  component across 8,487 callbacks. This checks original capture consistency,
  not Rust tire-force parity. The setup remained consistent with neutral tuning.
  Revalidated collection-parent links identify `911turbo`; the user confirms
  maximum ordinary upgrades plus Junkman. An installed-data regression checks
  captured mass 1,590 kg, steering multiplier 1.05 and both tire radii 0.3171 m
  for that package. A generated instance key alone does not identify the base car.
