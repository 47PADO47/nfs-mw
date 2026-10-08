//! Nonsense input and nonsense tuning must never produce NaN, infinity or a panic.

use super::common::{all_finite, hold, input, run, tuning};
use crate::{
    AccelTransition, CarInput, CarSoundTuning, DecelWindow, EngineMixer, EngineMode, EngineTuning, ShiftStage,
    ShiftTuning, TICK_SECONDS, WheelInput,
};

fn nonsense() -> CarInput {
    let wheel = WheelInput { slip: f32::NAN, skid: f32::INFINITY, load: f32::NEG_INFINITY, ..WheelInput::default() };
    CarInput {
        rpm_pct: f32::NAN,
        throttle: f32::INFINITY,
        brake: f32::NAN,
        gear: i32::MIN,
        speed: f32::NAN,
        up_dot: f32::NAN,
        pitch_multiplier: f32::NAN,
        wheels: [wheel; 4],
        ..CarInput::default()
    }
}

#[test]
fn nonsense_telemetry_gives_finite_output() {
    let mut m = EngineMixer::new(&tuning());
    hold(&mut m, 1.0, input(0.6, 1.0, 3));
    for out in hold(&mut m, 1.0, nonsense()) {
        assert!(all_finite(&out), "{out:?}");
    }
    for out in hold(
        &mut m,
        1.0,
        CarInput { gear: i32::MAX, rpm_pct: 50.0, throttle: -4.0, speed: -1e30, ..input(0.0, 0.0, 1) },
    ) {
        assert!(all_finite(&out), "{out:?}");
    }
    // And it recovers.
    let out = *hold(&mut m, 3.0, input(0.5, 0.5, 4)).last().unwrap();
    assert!((out.physics_rpm - 5500.0).abs() < 1.0);
}

#[test]
fn nonsense_frame_times_are_harmless() {
    let mut m = EngineMixer::new(&tuning());
    for dt in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0, 1e30, 1e-30] {
        let out = m.update(dt, &input(0.5, 1.0, 3));
        assert!(all_finite(&out), "dt {dt}");
    }
}

#[test]
fn rpm_below_idle_reads_as_idle() {
    let mut m = EngineMixer::new(&tuning());
    let out = *hold(&mut m, 2.0, input(-3.0, 0.0, 1)).last().unwrap();
    assert!((out.physics_rpm - 1000.0).abs() < 1e-3);
    assert!((out.eng_rpm - 1000.0).abs() < 1.0);
    assert!((out.ginsu_frequency - 1500.0).abs() < 1.0);
}

#[test]
fn empty_tuning_is_silent_but_finite() {
    let zero = CarSoundTuning {
        engine: EngineTuning { min_rpm: 0.0, max_rpm: 0.0, rpm_map: [0.0; 4], ..EngineTuning::default() },
        shift: ShiftTuning::default(),
        accel_transition: AccelTransition::default(),
        ..CarSoundTuning::default()
    };
    let mut m = EngineMixer::new(&zero);
    let outs = run(&mut m, 600, |i| input((i % 100) as f32 / 100.0, 1.0, 2 + (i / 120) as i32));
    assert!(outs.iter().all(all_finite));
    assert!(outs.iter().all(|o| o.accel_volume == 0.0 && o.aems_volume == 0.0));
}

#[test]
fn inverted_ranges_and_zero_stage_times_are_harmless() {
    let mut t = tuning();
    t.engine.min_rpm = 8000.0;
    t.engine.max_rpm = 1000.0;
    t.engine.decel_window =
        DecelWindow { min_rpm: 7000.0, max_rpm: 2000.0, fade_in_fraction: 2.0, fade_out_fraction: -1.0 };
    t.engine.mix.accel_delta_threshold = 0.0;
    t.engine.mix.decel_delta_threshold = -5.0;
    t.shift.up_disengage_fall = vec![ShiftStage { rpm: 0, time_ms: 0, curve: [[0.0; 2]; 4] }; 2];
    t.shift.up_engage = Some(ShiftStage { rpm: -500, time_ms: -20, curve: [[1.0, 1.0]; 4] });
    t.shift.down_engage_rise = (0, 0);
    t.shift.down_engage_fall = (0, 0);
    t.shift.down_disengage_fall = (0, 0);
    t.shift.lfo_rpm.period_ms = 0;
    t.shift.lfo_volume.period_ms = u32::MAX;
    t.accel_transition.peak_ms = 0;
    t.accel_transition.resume_ms = 0;
    t.accel_transition.interrupt_ms = 0;
    for mode in [EngineMode::Dual, EngineMode::Single] {
        t.engine.mode = mode;
        let mut m = EngineMixer::new(&t);
        let outs = run(&mut m, 900, |i| {
            let gear = 2 + ((i / 60) % 5) as i32;
            input(
                ((i % 200) as f32 / 200.0).min(1.0),
                if (i / 45) % 2 == 0 { 1.0 } else { 0.0 },
                if i % 400 > 300 { gear - 1 } else { gear },
            )
        });
        assert!(outs.iter().all(all_finite), "{mode:?}");
    }
}

#[test]
fn volumes_stay_in_range_whatever_the_tuning() {
    let mut t = tuning();
    t.engine.mix.aems_volume = u32::MAX;
    t.engine.mix.accel_loop_volume = u32::MAX;
    t.shift.up_engage_attack_volume = 50.0;
    t.shift.lfo_volume.amplitude = u32::MAX;
    let mut m = EngineMixer::new(&t);
    let outs = run(&mut m, 600, |i| input(0.8, 1.0, 2 + (i / 100) as i32));
    for out in outs {
        for v in [out.accel_volume, out.decel_volume, out.aems_volume, out.redline_sample_volume] {
            assert!((0.0..=1.0).contains(&v), "{v}");
        }
    }
    let _ = TICK_SECONDS;
}
