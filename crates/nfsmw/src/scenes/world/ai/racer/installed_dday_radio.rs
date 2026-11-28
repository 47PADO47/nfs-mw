//! Radio eligibility in career races.
use super::*;

#[test]
#[ignore = "needs installed race data and world collision"]
fn dday_holds_the_radio_and_a_normal_race_does_not() {
    let input = ActionState::default();
    for (event, expected) in [("16.1.0", true), ("15.2.1", false)] {
        let (mut scene, mut renderer) = loaded_world();
        assert!(!scene.holds_radio(), "free roam permits the radio");
        scene.queue_race(event.into(), RaceOptions { career: true, ..Default::default() });
        for _ in 0..600 {
            scene.update(&mut renderer, &input, STEP);
            if scene.race.as_ref().is_some_and(|race| race.event == event) {
                break;
            }
        }
        assert_eq!(scene.race.as_ref().map(|race| race.event.as_str()), Some(event));
        assert_eq!(scene.holds_radio(), expected, "{event}: radio eligibility");
    }
}
