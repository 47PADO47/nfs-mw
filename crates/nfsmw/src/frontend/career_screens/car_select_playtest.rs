//! Safe-house owned-car route regressions, separate from the standalone My Cars manager.

use super::*;
use crate::career::CareerData;
use crate::frontend::career_screens::rep_sheets::playtest::Harness;
use crate::frontend::quick_play::{fill::logo_textures, pool};
use crate::input::{Action, Bindings, InputDevice};
use blackbox_feng::{NodeKind, runtime::pad};
use nfsmw_career::stable::CarRecord;
use std::sync::Arc;

#[test]
#[ignore = "requires installed bonus cars, career tables and selection art"]
fn batch16_career_m3_gtr_has_identity_and_measured_stats() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let pool = pool::load_now(&dir);
    let special = pool
        .presets
        .iter()
        .find(|p| p.car.folder.eq_ignore_ascii_case("BMWM3GTRE46"))
        .or_else(|| pool.presets.iter().find(|p| p.car.folder.eq_ignore_ascii_case("BMWM3GTR")))
        .expect("M3 GTR preset");
    let (car, _) = super::super::car_lot_look::pool_car(&special.car.name)
        .expect("owned bonus car must resolve beyond the for-sale pool");
    assert!(car.stats.is_some(), "bonus identity must retain its own measured performance");
    let mut career = Career::new("UI16", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    let lot = career.data.cars.iter().find(|c| c.name == special.car.name).unwrap();
    career.stable.cars[0] = CarRecord {
        handle: 7,
        frontend_key: lot.frontend_key,
        vehicle_key: lot.vehicle_key,
        flags: CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED,
        career_index: 0,
        customization_index: 0,
        ..CarRecord::free()
    };
    career.settings.car_handle = 7;
    career.stable.careers[0] = CareerRecord::new(0);
    assert_eq!(career.stable.free_car_count(), 1, "the M3 fixture must be driveable");
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    open(&mut h);
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT));
    for name in [0x3e01_ad1d, 0xb05d_d708] {
        assert!(h.node(name).visible, "owned M3 GTR identity must render");
    }
    CareerHandle::shared().clear();
}

fn fixture() -> (Career, Vec<pool::PoolCar>) {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let pool = pool::load_now(&dir);
    let cars: Vec<_> = ["CARRERAGT", "TT", "GTI"]
        .into_iter()
        .map(|folder| {
            pool.cars.iter().find(|car| car.folder.eq_ignore_ascii_case(folder)).expect("installed career car").clone()
        })
        .collect();
    let mut career = Career::new("UI13", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    for (index, car) in cars.iter().enumerate() {
        let lot = career.data.cars.iter().find(|lot| lot.name == car.name).unwrap();
        career.stable.cars[index] = CarRecord {
            handle: 7 + index as u32,
            frontend_key: lot.frontend_key,
            vehicle_key: lot.vehicle_key,
            flags: CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED,
            career_index: index as u8,
            customization_index: index as u8,
            ..CarRecord::free()
        };
        career.stable.careers[index] = CareerRecord::new(index as u8);
        career.stable.careers[index].bounty = 1000 * (index as u32 + 1);
    }
    career.settings.car_handle = 7;
    career.markers.add(MarkerKind::ExtraStrike, 1);
    assert_eq!(career.stable.free_car_count(), 3, "the fixture must contain live, driveable owned cars");
    (career, cars)
}

fn open(h: &mut Harness) {
    h.open(screen::CAR_SELECT, Box::new(CareerCarSelectLogic::new(CarSelectEntry::Plain)));
}

fn assert_car(h: &Harness, expected: &pool::PoolCar, handle: u32) {
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT), "the owned selector must remain open");
    let Some(Look::SafeHouse(look)) = h.screens.look() else { panic!("career selection must publish its preview") };
    assert_eq!(look.car, expected.name);
    assert_eq!(look.record.unwrap().handle, handle);
    assert_eq!(CareerHandle::shared().with(|career| career.settings.car_handle), Some(handle));
    let (maker, model) = logo_textures(h.screens.assets(), expected);
    for (name, texture) in [(0x3e01_ad1d, maker), (0xb05d_d708, model)] {
        let node = h.node(name);
        assert!(node.visible, "selected car identity must be visible");
        assert!(matches!(node.kind, NodeKind::Image { texture: got, .. } if got == texture));
    }
    let tree = h.tree();
    assert!(!tree.nodes.iter().any(|n| n.visible && n.text.as_deref().is_some_and(|s| s.starts_with("Debug:"))));
    assert!(!tree.nodes.iter().any(|n| n.visible && n.text.as_deref() == Some(expected.name.as_str())));
    let bounty = format!("{}", 1000 * (handle - 6));
    let bounty = crate::i18n::fmt::grouped(bounty.parse().unwrap());
    assert_eq!(h.node(0x322b_18f9).text.as_deref(), Some(bounty.as_str()));
}

#[test]
#[ignore = "requires installed career tables, owned-car package and art; set NFSMW_GAME_DIR"]
fn career_batch13_owned_car_selection_binds_preview_logos_names_and_return_state() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let (career, cars) = fixture();
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    open(&mut h);
    assert_car(&h, &cars[0], 7);
    for index in [1, 2] {
        h.press(pad::RIGHT);
        assert_car(&h, &cars[index], 7 + index as u32);
    }
    h.press(pad::BACK);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    assert_eq!(CareerHandle::shared().with(|career| career.settings.car_handle), Some(7));
    open(&mut h);
    assert_car(&h, &cars[0], 7);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    open(&mut h);
    assert_car(&h, &cars[1], 8);
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "requires installed career car select; set NFSMW_GAME_DIR"]
fn career_batch13_owned_car_footer_has_only_supported_bound_actions() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let (career, _) = fixture();
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    open(&mut h);
    let logic = CareerCarSelectLogic { original: 7, ..CareerCarSelectLogic::new(CarSelectEntry::Plain) };
    for device in [InputDevice::Keyboard, InputDevice::Xbox] {
        let hints =
            crate::frontend::prompts::hint_set(screen::CAR_SELECT, logic.footer(), &Bindings::default(), device);
        let actions: Vec<_> = hints.iter().filter_map(|h| h.2).collect();
        assert_eq!(actions, [Action::MenuBack, Action::MenuAccept, Action::MenuButton2]);
        let tree = h.presented(device);
        assert!(
            !tree
                .nodes
                .iter()
                .any(|n| n.visible && n.text.as_deref().is_some_and(|t| t == "Showcase" || t == "Sell Car"))
        );
        assert!(
            tree.nodes
                .iter()
                .any(|n| n.visible && n.text.as_deref() == Some(crate::i18n::game_or_error(LABEL_USE_MARKER).as_str()))
        );
        if device == InputDevice::Xbox {
            assert!(tree.nodes.iter().any(|n| n.visible
                && matches!(n.kind, NodeKind::Image {texture, ..} if texture == crate::ui::input_icons::ATLAS)));
        }
    }
    h.commands.clear();
    h.press(pad::BUTTON0 << 5);
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT), "career select does not sell cars");
    h.press(pad::BUTTON0 << 4);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "the marker opens its confirmation");
    assert!(
        h.tree()
            .nodes
            .iter()
            .any(|node| node.text.as_deref() == Some(crate::i18n::game_or_error(TEXT_ADD_STRIKE).as_str()))
    );
    h.run(0, 100);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    h.run(0, 100);
    assert!(
        h.commands
            .iter()
            .any(|command| matches!(command, Command::CareerCar(CarSelectAction::UseStrikeMarker { handle: 7 })))
    );
    CareerHandle::shared().clear();
}
