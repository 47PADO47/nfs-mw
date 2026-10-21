use crate::{EmitterSpec, Rng, control::Control};

// Synthetic input/output fixtures verified by executing the original PC controller.
#[test]
fn final_cycle_update_emits_with_the_pc_positive_rollover() {
    let spec = EmitterSpec { on_cycle: 0.1, one_shot: true, ..Default::default() };
    let mut control = Control::default();
    let mut rng = Rng::new(1);
    for expected in [0.04, 0.04, 0.06] {
        let delta = control.emission_delta(&spec, false, &mut rng, 0.04).unwrap();
        assert!((delta - expected).abs() < 1e-6);
    }
    assert!(control.finished());
    assert!(control.emission_delta(&spec, false, &mut rng, 0.04).is_none());
}

#[test]
fn delay_rollover_emits_only_the_unblocked_fraction_of_the_tick() {
    let spec = EmitterSpec { start_delay: 0.03, on_cycle: 0.1, one_shot: true, ..Default::default() };
    let mut control = Control::default();
    let mut rng = Rng::new(1);
    assert!(control.emission_delta(&spec, false, &mut rng, 0.02).is_none());
    let delta = control.emission_delta(&spec, false, &mut rng, 0.02).unwrap();
    assert!((delta - 0.01).abs() < 1e-6);
}

#[test]
fn no_off_cycle_makes_a_non_one_shot_continuous() {
    let spec = EmitterSpec { on_cycle: 0.1, ..Default::default() };
    let mut control = Control::default();
    let mut rng = Rng::new(1);
    for _ in 0..100 {
        assert_eq!(control.emission_delta(&spec, false, &mut rng, 0.02), Some(0.02));
    }
    assert!(!control.finished());
}
