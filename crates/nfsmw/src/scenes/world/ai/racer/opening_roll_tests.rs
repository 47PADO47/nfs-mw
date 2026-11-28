//! The installed opening race releases the authored rolling grid into the real race driver.

use super::installed_tests::loaded_world;
use crate::input::ActionState;
use crate::scenes::world::drive::STEP;
use crate::scenes::world::race::RaceOptions;
use crate::viewer::Scene;

#[test]
#[ignore = "requires the installed opening race, cars and NIS data"]
fn installed_opening_roll_seeds_razors_controller_before_go() {
    let (mut scene, mut renderer) = loaded_world();
    let rules = scene.session_rules();
    scene.set_session_rules(crate::scenes::world::session_rules::SessionRules { race_screens: false, ..rules });
    scene.queue_race("16.1.0".into(), RaceOptions { career: true, traffic: Some(0), ..Default::default() });
    let mut checked = 0;
    for _ in 0..20_000 {
        scene.update(&mut renderer, &ActionState::default(), STEP);
        let Some(engine) = scene.race.as_ref().and_then(|race| race.engine.as_ref()) else { continue };
        if engine.phase() != blackbox_race::Phase::Racing || engine.elapsed() <= 0.0 {
            continue;
        }
        let traffic = scene.traffic.as_ref().expect("race traffic world");
        let (_, _, speed) = traffic.racers().find(|(id, ..)| *id == 1).expect("Razor's car");
        let (_, driver) = traffic.racers_driving().find(|(id, _)| *id == 1).expect("Razor's controller");
        assert!(speed > 30.0, "Razor's carried speed at {}: {speed}", engine.elapsed());
        assert!(driver.target_speed > 30.0, "Razor's target at {}: {driver:?}", engine.elapsed());
        assert!(driver.brake < 0.8, "Razor brakes during the rolling handoff: {driver:?}");
        checked += 1;
        if checked == 8 {
            break;
        }
    }
    assert_eq!(checked, 8, "the real controller must run for eight ticks after GO");
}
