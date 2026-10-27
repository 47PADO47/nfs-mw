use super::*;
use crate::career::{Career, CareerData, CareerHandle};
use crate::scenes::world::session_rules::SessionRules;

struct ClearCareer(CareerHandle);

impl Drop for ClearCareer {
    fn drop(&mut self) {
        self.0.clear();
    }
}

#[test]
#[ignore = "requires installed career data, cars and road collision"]
fn installed_extra_options_testing_survives_career_frames_and_rejects_multiplayer() {
    let _lock = crate::career::SHARED_CAREER_TEST.lock().unwrap();
    let shared = CareerHandle::shared();
    shared.clear();
    let _clear = ClearCareer(shared.clone());
    let (mut scene, mut renderer) = garage_fixture();
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = std::sync::Arc::new(CareerData::load(&dir).unwrap());
    let career = Career::new("f5-career", data, &nfsmw_career::settings::NewCareer::default()).unwrap();
    shared.set(career);
    scene.set_career(shared.clone(), None);
    assert!(shared.is_active(), "the regression must use an actively played career");
    let profile = shared.with(|career| career.profile_bytes()).unwrap();
    let density = scene.traffic_setting;

    for (key, value) in [("unlimited_nos", "on"), ("unlimited_breaker", "on"), ("traffic_off", "on")] {
        command::run(&mut scene, &mut renderer, &["testing", key, value]).unwrap();
    }
    for _ in 0..3 {
        scene.prepare_extra();
        assert!(scene.extra.local_allowed);
        assert!(scene.extra.testing.unlimited_nos && scene.extra.testing.unlimited_breaker);
        assert!(scene.extra.testing.traffic_off, "career preparation must not clear the chosen override");
    }
    scene.extra_action(&mut renderer, Action::Refill).unwrap();
    assert_eq!(scene.traffic_setting, density, "F5 keeps the F8 traffic-density preference");
    assert_eq!(shared.with(|career| career.profile_bytes()).unwrap(), profile);
    scene.extra_action(&mut renderer, Action::Clear).unwrap();
    assert!(!scene.extra.testing.active());

    scene.set_session_rules(SessionRules::session(7));
    scene.extra.testing.unlimited_nos = true;
    scene.prepare_extra();
    assert!(!scene.extra.local_allowed);
    assert!(!scene.extra.testing.active());
    assert!(scene.extra_action(&mut renderer, Action::Refill).is_err());
    assert!(command::run(&mut scene, &mut renderer, &["testing", "unlimited_nos", "on"]).is_err());
    assert!(shared.is_active(), "session gating must not unload the career");
}
