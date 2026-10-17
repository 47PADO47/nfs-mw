use super::state::WetRoad;
use crate::settings::RoadReflections;

fn run(road: &mut WetRoad, fps: u32, seconds: u32, raining: bool) {
    for _ in 0..fps * seconds {
        road.step(1.0 / fps as f32, raining);
    }
}

#[test]
fn wet_and_dry_history_matches_elapsed_time_at_high_fps() {
    let mut reference = WetRoad::default();
    run(&mut reference, 60, 10, true);
    let expected_wet = reference.amount();
    run(&mut reference, 60, 4, false);
    let expected_damp = reference.amount();
    for fps in [30, 60, 144, 360] {
        let mut road = WetRoad::default();
        run(&mut road, fps, 10, true);
        assert!((road.amount() - expected_wet).abs() < 0.0001, "{fps} FPS onset");
        run(&mut road, fps, 4, false);
        assert!((road.amount() - expected_damp).abs() < 0.0001, "{fps} FPS drying");
        assert!(road.eligible(), "dry weather can retain dampness");
        run(&mut road, fps, 12, false);
        assert_eq!(road.amount(), 0.0);
        assert!(!road.eligible());
    }
}

#[test]
fn wetness_override_and_quality_never_replace_rain_history() {
    let mut road = WetRoad::default();
    run(&mut road, 60, 2, true);
    let history = road.wetness;
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        road.step(dt, false);
    }
    assert_eq!(road.wetness, history);
    road.override_amount = Some(1.0);
    road.quality = RoadReflections::Off;
    assert!(!road.eligible());
    assert_eq!(road.contribution(), 0.0);
    road.quality = RoadReflections::Vehicles;
    assert!(road.eligible());
    assert_eq!(road.contribution(), 1.0);
    assert_eq!(road.wetness, history);
    road.override_amount = None;
    assert_eq!(road.amount(), history);
}
