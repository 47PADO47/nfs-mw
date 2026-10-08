//! Shared test fixtures: a generic car tuning (values of the same size as real data, no game names) and
//! helpers to run scripted sequences at the fixed tick.

use crate::{
    AccelTransition, CarInput, CarSoundTuning, DecelWindow, EngineMix, EngineMode, EngineOutput, EngineTuning,
    MixLevels, ShiftStage, ShiftTuning, TICK_SECONDS, Wobble,
};

pub fn levels(steady: f32, large: f32) -> MixLevels {
    MixLevels { steady, large }
}

pub fn engine_tuning() -> EngineTuning {
    EngineTuning {
        mode: EngineMode::Dual,
        min_rpm: 1500.0,
        max_rpm: 7784.0,
        rpm_map: [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0],
        mix: EngineMix {
            aems: levels(0.5, 0.8),
            ginsu: levels(0.7, 1.0),
            decel_aems: levels(0.6, 0.8),
            decel_ginsu: levels(0.8, 1.0),
            ginsu_neg: levels(0.1, 0.3),
            accel_delta_threshold: 100.0,
            decel_delta_threshold: 120.0,
            aems_volume: 25000,
            decel_aems_volume: 20000,
            accel_loop_volume: 30000,
            decel_loop_volume: 28000,
        },
        decel_window: DecelWindow { min_rpm: 2000.0, max_rpm: 7000.0, fade_in_fraction: 0.25, fade_out_fraction: 0.02 },
        low_pass_cutoff: 20000,
        shift_sweet_volume: 20000,
        sputter_volume: 0,
        accel_loop_min_frequency: 0.0,
        redline_enabled: true,
    }
}

pub fn shift_tuning() -> ShiftTuning {
    ShiftTuning {
        up_sound_delay: 0.05,
        down_sound_delay: 0.03,
        up_volume: 25000,
        down_volume: 20000,
        up_engage_attack_volume: 0.3,
        up_engage_attack_ms: 200,
        up_disengage_fall: vec![ShiftStage {
            rpm: 3500,
            time_ms: 200,
            curve: [[0.0, 0.0], [0.3, -0.4], [0.7, -0.9], [1.0, -1.0]],
        }],
        up_engage: Some(ShiftStage {
            rpm: 600,
            time_ms: 300,
            curve: [[0.0, 0.0], [0.2, 1.0], [0.5, -0.3], [1.0, 0.0]],
        }),
        down_disengage_fall: (1500, 150),
        down_engage_rise: (1200, 200),
        down_engage_fall: (800, 250),
        down_reattach_scale: 0.3,
        lfo_rpm: Wobble { amplitude: 200, period_ms: 120, decay_ms: 600 },
        lfo_volume: Wobble { amplitude: 4000, period_ms: 100, decay_ms: 500 },
        upgrade_level: 0,
    }
}

pub fn tuning() -> CarSoundTuning {
    CarSoundTuning {
        engine: engine_tuning(),
        shift: shift_tuning(),
        accel_transition: AccelTransition {
            peak_ms: 300,
            peak_rpm: 3000,
            peak_volume: 0.4,
            resume_ms: 600,
            interrupt_ms: 700,
        },
        turbo: None,
        ..CarSoundTuning::default()
    }
}

pub fn input(rpm_pct: f32, throttle: f32, gear: i32) -> CarInput {
    CarInput { rpm_pct, throttle, gear, ..CarInput::default() }
}

/// Steps `ticks` fixed ticks with `make(tick index)` as the telemetry; returns every tick's output.
pub fn run<F: FnMut(usize) -> CarInput>(
    mixer: &mut crate::EngineMixer,
    ticks: usize,
    mut make: F,
) -> Vec<EngineOutput> {
    (0..ticks).map(|i| mixer.update(TICK_SECONDS, &make(i))).collect()
}

/// Steady input for `seconds`.
pub fn hold(mixer: &mut crate::EngineMixer, seconds: f32, input: CarInput) -> Vec<EngineOutput> {
    let ticks = (seconds * 60.0).round() as usize;
    run(mixer, ticks, |_| input)
}

pub fn all_finite(out: &EngineOutput) -> bool {
    [
        out.ginsu_frequency,
        out.playback_rate,
        out.accel_loop.frequency,
        out.accel_loop.playback_rate,
        out.decel_loop.frequency,
        out.decel_loop.playback_rate,
        out.accel_volume,
        out.decel_volume,
        out.aems_volume,
        out.redline_sample_volume,
        out.low_pass_hz,
        out.visual_rpm_fraction,
        out.aems_torque,
        out.eng_rpm,
        out.eng_torque,
        out.physics_rpm,
        out.throttle,
    ]
    .iter()
    .all(|v| v.is_finite())
}
