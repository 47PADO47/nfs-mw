use super::*;
use crate::career::{Career, CareerData, CareerHandle};
use crate::frontend::flow_movie_tests::install_rig;
use crate::frontend::ids::screen;
use nfsmw_career::stable::{CarRecord, CareerRecord};
use std::sync::Arc;

#[test]
#[ignore = "requires installed safe-house/car-select packages; set NFSMW_GAME_DIR"]
fn career_batch13_safe_house_my_cars_keeps_the_selected_backdrop_live() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let mut rig = install_rig().expect("installed menu fixture");
    let data = CareerData::load(&rig.fe.dir).unwrap();
    let key = data.cars.iter().find(|car| car.name.eq_ignore_ascii_case("tt")).unwrap().clone();
    let mut career = Career::new("UI13", Arc::new(data), &Default::default()).unwrap();
    career.stable.cars[0] = CarRecord {
        handle: 7,
        frontend_key: key.frontend_key,
        vehicle_key: key.vehicle_key,
        flags: CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED,
        career_index: 0,
        ..CarRecord::free()
    };
    career.stable.careers[0] = CareerRecord::new(0);
    career.settings.car_handle = 7;
    rig.fe.career = CareerHandle::shared();
    rig.fe.career.set(career);
    rig.fe.career.begin_play();
    rig.fe.save = false;
    rig.with(|fe, host, env| {
        fe.open_safe_house(host, env);
        assert_eq!(fe.screens.top(), Some(screen::MAIN_MENU_SUB));
        fe.apply(vec![Command::CareerCar(CarSelectAction::Open(CarSelectEntry::Plain))], host, env);
        assert_eq!(fe.screens.top(), Some(CAR_SELECT));
        assert_eq!(fe.stage, Stage::Menus, "the owned-car preview must continue updating");
        assert!(fe.menu_in_window);
        assert!(fe.career.is_prelude(), "Resume must build the selected car, not resume a menu scene");
        let Some(Look::SafeHouse(car)) = fe.screens.look() else { panic!("selected owned-car look") };
        assert_eq!(car.record.unwrap().handle, 7);
        assert_eq!(fe.look, fe.screens.look().unwrap());
        assert_eq!(fe.screens.pose(), Some("career_car_select"));
    });
    CareerHandle::shared().clear();
}
