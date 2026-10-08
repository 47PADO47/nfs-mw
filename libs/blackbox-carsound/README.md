# blackbox-carsound

The car sound controllers of EA Black Box games: per-frame car telemetry in, the engine mix out.

[`EngineMixer`] holds everything the engine sound needs of one car: the audio RPM (with the clutch model of the
local player's car), the gear-shift and throttle-stab behaviour, the mix of the accelerate and decelerate Ginsu
loops, and the tachometer value that goes back to the physics. The tuning values come in as plain structs
(`CarSoundTuning`) that the game fills from its data; the output is plain numbers (the Ginsu frequency, linear
loop volumes, one-shot events such as the gear clunk) that the game maps to its synthesiser (`blackbox-ginsu`)
and its banks.

- **Pure and deterministic.** No audio output, no file access, no game knowledge. The original's controllers
  run once per frame with constants that only fit a fixed rate; here [`EngineMixer::update`] takes real frame
  times and runs them on a fixed 60 Hz tick (`TICK_HZ`), at most `MAX_TICKS_PER_UPDATE` per call.
- **Written from the specs** `docs/specs/engine-sound.md` and `docs/specs/engine-sound-effects.md`, not from the
  decompilation (`docs/provenance/engine-sound.md`).
- **Not done yet:** the skid, road and wind noise, nitrous, turbo and collision controllers (specified in
  `engine-sound-effects.md`; their tuning structs exist, their controllers do not), and the sample (AEMS) layer.

```rust
use blackbox_carsound::{CarInput, CarSoundTuning, EngineMixer};

let mut mixer = EngineMixer::new(&CarSoundTuning::default());
let out = mixer.update(1.0 / 60.0, &CarInput { rpm_pct: 0.6, throttle: 1.0, gear: 3, ..CarInput::default() });
// out.ginsu_frequency, out.accel_volume, out.decel_volume, out.events.gear_clunk ...
```
