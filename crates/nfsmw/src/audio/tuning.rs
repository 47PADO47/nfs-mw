//! From a car's sound data (`nfsmw-data`) to the tuning the engine mixer (`blackbox-carsound`) reads. Every
//! field is a copy: the two crates name them alike so the mapping stays obvious.

use blackbox_carsound as cs;
use nfsmw_data::sound::{self as data, CarSound};

/// The engine mixer's tuning for `sound`, whose accelerate loop starts at `accel_min_frequency` (the `.gin`
/// file's lowest frequency; below it the loops play at it and the playback rate carries the ratio). The local
/// player's car gets the RPM remap and the clutch model.
pub fn tuning(sound: &CarSound, accel_min_frequency: f32) -> cs::CarSoundTuning {
    cs::CarSoundTuning {
        engine: cs::EngineTuning { accel_loop_min_frequency: accel_min_frequency, ..engine(&sound.engine) },
        shift: shift(&sound.shift),
        accel_transition: accel(&sound.accel_transition),
        ..cs::CarSoundTuning::default()
    }
}

fn levels(l: data::MixLevels) -> cs::MixLevels {
    cs::MixLevels { steady: l.steady, large: l.large }
}

fn engine(e: &data::EngineSound) -> cs::EngineTuning {
    let mix = &e.mix;
    cs::EngineTuning {
        mode: if e.decel_loop.is_empty() { cs::EngineMode::Single } else { cs::EngineMode::Dual },
        min_rpm: e.min_rpm,
        max_rpm: e.max_rpm,
        rpm_map: e.rpm_map,
        mix: cs::EngineMix {
            aems: levels(mix.aems),
            ginsu: levels(mix.ginsu),
            decel_aems: levels(mix.decel_aems),
            decel_ginsu: levels(mix.decel_ginsu),
            ginsu_neg: levels(mix.ginsu_neg),
            accel_delta_threshold: mix.accel_delta_threshold,
            decel_delta_threshold: mix.decel_delta_threshold,
            aems_volume: mix.aems_volume,
            decel_aems_volume: mix.decel_aems_volume,
            accel_loop_volume: mix.accel_loop_volume,
            decel_loop_volume: mix.decel_loop_volume,
        },
        decel_window: cs::DecelWindow {
            min_rpm: e.decel_window.min_rpm,
            max_rpm: e.decel_window.max_rpm,
            fade_in_fraction: e.decel_window.fade_in_fraction,
            fade_out_fraction: e.decel_window.fade_out_fraction,
        },
        low_pass_cutoff: e.low_pass_cutoff,
        shift_sweet_volume: e.shift_sweet_volume,
        sputter_volume: e.sputter_volume,
        accel_loop_min_frequency: 0.0,
        redline_enabled: true,
    }
}

fn stage(s: &data::ShiftStage) -> cs::ShiftStage {
    cs::ShiftStage { rpm: s.rpm, time_ms: s.time_ms, curve: s.curve }
}

fn wobble(w: data::Wobble) -> cs::Wobble {
    cs::Wobble { amplitude: w.amplitude, period_ms: w.period_ms, decay_ms: w.decay_ms }
}

fn shift(s: &data::ShiftSound) -> cs::ShiftTuning {
    cs::ShiftTuning {
        up_sound_delay: s.up_sound_delay,
        down_sound_delay: s.down_sound_delay,
        up_volume: s.up_volume,
        down_volume: s.down_volume,
        up_engage_attack_volume: s.up_engage_attack_volume,
        up_engage_attack_ms: s.up_engage_attack_ms,
        up_disengage_fall: s.up_disengage_fall.iter().map(stage).collect(),
        up_engage: s.up_engage.as_ref().map(stage),
        down_disengage_fall: s.down_disengage_fall,
        down_engage_rise: s.down_engage_rise,
        down_engage_fall: s.down_engage_fall,
        down_reattach_scale: s.down_reattach_scale,
        lfo_rpm: wobble(s.lfo_rpm),
        lfo_volume: wobble(s.lfo_volume),
        upgrade_level: 0,
    }
}

fn accel(a: &data::AccelTransition) -> cs::AccelTransition {
    cs::AccelTransition {
        peak_ms: a.peak_ms,
        peak_rpm: a.peak_rpm,
        peak_volume: a.peak_volume,
        resume_ms: a.resume_ms,
        interrupt_ms: a.interrupt_ms,
    }
}
