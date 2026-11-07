use super::*;
use crate::career::{Career, CareerData, CareerHandle};
use crate::garage::menu_rows::Row;
use crate::scenes::world::Options;
use crate::viewer::Scene;
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::{CarRecord, CustomizationRecord};

#[test]
#[ignore = "needs installed cars and tuning catalogs"]
fn installed_race_play_path_stock_tuning_keeps_the_driven_car_and_career() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: None,
            height: 30.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 1000.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    let baseline = Selection { car: "CARRERAGT".into(), ..Default::default() };
    scene.apply_garage_selection(&mut renderer, baseline.clone()).unwrap();
    let mut career = Career::new("TUNINGTEST", CareerData::cached(&dir).unwrap(), &NewCareer::default()).unwrap();
    career.settings.car_handle = 42;
    career.stable.cars[0] = CarRecord {
        handle: 42,
        frontend_key: blackbox_attrib::vlt_hash("cobaltss"),
        vehicle_key: blackbox_attrib::vlt_hash("cobaltss"),
        flags: CarRecord::FLAG_OWNED | CarRecord::CLASS_MASK,
        customization_index: 0,
        career_index: 0,
        tail: [0; 2],
    };
    career.stable.set_customization_for_handle(42, &CustomizationRecord::default());
    let handle = CareerHandle::default();
    handle.set(career);
    scene.set_career(handle.clone(), None);
    let saved = handle.with(|career| career.profile_bytes()).unwrap();
    assert_eq!(scene.tuning_available(), Some(false));
    scene.begin_tuning(&mut renderer).unwrap();
    assert_eq!(scene.last_car, "CARRERAGT");
    assert_eq!(scene.garage.applied, baseline, "opening tuning cannot restore the career's Cobalt");
    assert!(
        TuningCategory::ALL.into_iter().all(|category| !Row::Tuning(category).enabled(&scene.garage)),
        "stock categories remain locked inside the open tuning page"
    );
    assert!(scene.career.garage.as_ref().unwrap().handle.is_none());
    let mut selection = baseline;
    selection.tuning.active = 1;
    scene.confirm_tuning(&mut renderer, selection.clone()).unwrap();
    assert_eq!(scene.garage.applied, selection);
    assert!(scene.garage.custom_slot.is_none(), "pause tuning cannot create a My Cars slot");
    assert_eq!(handle.with(|career| career.profile_bytes()).unwrap(), saved, "another career car must stay unchanged");
    scene.close(&mut renderer);
}
