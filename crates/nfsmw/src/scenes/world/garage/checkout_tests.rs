use super::*;
use crate::career::{Career, CareerData, CareerHandle, ProfileDir};
use crate::viewer::Scene;
use nfsmw_career::settings::{Flags, NewCareer};
use nfsmw_career::stable::{CarRecord, CareerRecord, CustomizationRecord};

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires the installed game"]
fn installed_career_garage_previews_purchases_reloads_and_keeps_f6_out_of_saves() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("select an install")).unwrap();
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "garage-career-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let profiles = ProfileDir::new(&scratch.0);
    let mut scene = WorldScene::open(
        &dir,
        super::super::Options {
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
    let baseline = Selection { car: "COBALTSS".into(), ..Default::default() };
    scene.apply_garage_selection(&mut renderer, baseline.clone()).unwrap();
    let data = CareerData::cached(&dir).unwrap();
    let mut career = Career::new("GARAGETEST", data.clone(), &NewCareer::default()).unwrap();
    career.settings.cash = 1_000_000;
    career.settings.car_handle = 42;
    career.mark_career_completed();
    career.settings.flags.remove(Flags::CAREER_COMPLETE);
    career.stable.cars[0] = CarRecord {
        handle: 42,
        frontend_key: blackbox_attrib::vlt_hash("cobaltss"),
        vehicle_key: blackbox_attrib::vlt_hash("cobaltss"),
        flags: CarRecord::FLAG_OWNED | CarRecord::CLASS_MASK,
        customization_index: 0,
        career_index: 0,
        tail: [0; 2],
    };
    career.stable.careers[0] = CareerRecord::new(0);
    career.stable.careers[0].heat = 5.0;
    career.stable.set_customization_for_handle(42, &CustomizationRecord::default());
    profiles.write(&career.alias.clone(), &career.profile_bytes()).unwrap();
    let handle = CareerHandle::default();
    handle.set(career);
    scene.set_career(handle.clone(), Some(profiles.clone()));
    let original = handle.with(|career| career.stable.clone()).unwrap();
    let file_before = profiles.read("GARAGETEST").unwrap();
    let mut desired = baseline.clone();
    desired.parts.parts[nfsmw_data::car::customization::Category::Paint as usize] = Some(
        scene.garage.selected().unwrap().parts.as_ref().unwrap().choices
            [nfsmw_data::car::customization::Category::Paint as usize][0]
            .hash,
    );
    desired.color = Some([12, 120, 240]);
    desired.performance.levels[4] = 1;
    desired.tuning.profiles[1][0] = -3;
    desired.tuning.active = 1;
    desired.parts.parts[nfsmw_data::car::customization::Category::Hood as usize] = Some(
        scene.garage.selected().unwrap().parts.as_ref().unwrap().choices
            [nfsmw_data::car::customization::Category::Hood as usize][0]
            .hash,
    );
    // The developer overlay changes only the running rig.
    scene.apply_garage_selection(&mut renderer, Selection { car: "BMWM3GTR".into(), ..Default::default() }).unwrap();
    scene.apply_garage_selection(&mut renderer, desired.clone()).unwrap();
    assert_eq!(scene.players.lead().unwrap().test_model().paint.as_ref().unwrap().rgb, [12, 120, 240]);
    assert_eq!(scene.garage.applied, desired, "switching back to the owned car must keep the F6 preview");
    assert_eq!(handle.with(|career| career.stable.clone()).unwrap(), original);
    assert_eq!(profiles.read("GARAGETEST").unwrap(), file_before);
    scene.begin_garage_session(&mut renderer).unwrap();
    assert_eq!(scene.garage.applied, baseline, "normal entry discards developer experiments");
    let position = scene.players.lead().unwrap().position();
    let valid_preview = scene.garage.draft.clone();
    scene.garage.draft.car = "MISSING_GARAGE_PREVIEW".into();
    assert!(scene.garage_view(&mut renderer, crate::garage::View::Customization, "customize_customizemain").is_err());
    assert!(scene.garage_backdrop.is_none(), "a failed initial preview cannot publish an incomplete showroom");
    scene.garage.draft = valid_preview;
    scene.garage_view(&mut renderer, crate::garage::View::Customization, "customize_customizemain").unwrap();
    scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0);
    assert_eq!(scene.players.lead().unwrap().position(), position, "the shop keeps its resident world frozen");
    let (_, instances) = scene.frame(16.0 / 9.0);
    // Front-end shadow modes salt the upper two group bits; the room retains its lower identity bits.
    assert!(
        instances.iter().any(|part| part.key.parts().0 & 0x3fff_ffff == (u32::MAX - 2) & 0x3fff_ffff),
        "the shop platform replaces the road"
    );
    scene.preview_garage_session(&mut renderer, desired.clone()).unwrap();
    assert_eq!(handle.with(|career| career.stable.clone()).unwrap(), original);
    scene.cancel_garage(&mut renderer).unwrap();
    assert!(scene.garage_backdrop.is_none(), "leaving the shop restores the resident world");
    assert_eq!(scene.garage.applied, baseline);
    scene.begin_garage_session(&mut renderer).unwrap();
    scene.preview_garage_session(&mut renderer, desired.clone()).unwrap();
    let total = scene.garage.shop_total.expect("installed prices must produce a quote");
    assert!(total > 0);
    handle.with(|career| career.settings.cash = 0);
    scene.refresh_garage_quote(&desired);
    assert_eq!(scene.garage.shop_total, Some(total), "an unaffordable cart retains its displayed total");
    assert!(!scene.garage.shop_items.is_empty(), "item prices remain available");
    assert!(!crate::garage::can_confirm(&scene.garage));
    assert!(scene.confirm_garage_session(&mut renderer, desired.clone()).is_err());
    assert_eq!(handle.with(|career| career.cash()), Some(0));
    handle.with(|career| career.settings.cash = 1_000_000);
    scene.refresh_garage_quote(&desired);
    scene.confirm_garage_session(&mut renderer, desired.clone()).unwrap();
    assert_eq!(handle.with(|career| career.cash()).unwrap(), 1_000_000 - total);
    assert!(handle.with(|career| career.stable.careers[0].heat).unwrap() < 5.0);
    let loaded = Career::from_profile_bytes("GARAGETEST", &profiles.read("GARAGETEST").unwrap(), data).unwrap();
    assert_eq!(loaded.cash(), 1_000_000 - total);
    handle.set(loaded);
    scene.apply_garage_selection(&mut renderer, baseline.clone()).unwrap();
    scene.restore_career_garage(&mut renderer).unwrap();
    assert_eq!(scene.garage.applied, desired, "native stages/parts/tuning and extended RGB must reload");
    quote_ones_digit_change(&mut scene, &mut renderer, &desired);
    scene.begin_garage_session(&mut renderer).unwrap();
    let cash_before_tuning = handle.with(|career| career.cash()).unwrap();
    desired.tuning.profiles[2][0] = 4;
    desired.tuning.active = 2;
    scene.confirm_tuning(&mut renderer, desired.clone()).unwrap();
    assert_eq!(handle.with(|career| career.cash()).unwrap(), cash_before_tuning, "pause tuning never buys equipment");
    assert_eq!(scene.garage.applied.tuning, desired.tuning);
    let stable = handle.with(|career| career.stable.clone()).unwrap();
    let cash = handle.with(|career| career.cash()).unwrap();
    scene.begin_garage_session(&mut renderer).unwrap();
    let mut failure = desired.clone();
    failure.color = Some([240, 12, 120]);
    scene.preview_garage_session(&mut renderer, failure.clone()).unwrap();
    let file = profiles.path("GARAGETEST");
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert!(scene.confirm_garage_session(&mut renderer, failure).is_err());
    assert_eq!(handle.with(|career| career.stable.clone()).unwrap(), stable);
    assert_eq!(handle.with(|career| career.cash()).unwrap(), cash);
    scene.cancel_garage_session(&mut renderer).unwrap();
    assert_eq!(scene.garage.applied, desired);
    handle.clear();
    scene.garage.custom_slot = None;
    scene.begin_tuning(&mut renderer).unwrap();
    let mut quick_tuning = scene.garage.applied.clone();
    quick_tuning.tuning.profiles[0][0] = -2;
    quick_tuning.tuning.active = 0;
    scene.confirm_tuning(&mut renderer, quick_tuning.clone()).unwrap();
    assert_eq!(scene.garage.applied, quick_tuning);
    assert!(scene.garage.custom_slot.is_none(), "quick-race tuning cannot create a My Cars slot");
    assert!(scene.career.garage.is_none());
    scene.close(&mut renderer);
}

fn quote_ones_digit_change(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, baseline: &Selection) {
    scene.begin_garage_session(renderer).unwrap();
    let catalog = scene.garage.selected().unwrap().parts.as_ref().unwrap();
    // Decal surfaces also describe stock bodies, which are not purchasable choices.
    let body = catalog.choices[nfsmw_data::car::customization::Category::Body as usize]
        .iter()
        .find_map(|part| {
            (catalog.decals[2].body_number_positions.get(&part.hash) == Some(&[true; 2])
                && catalog.decals[3].body_number_positions.get(&part.hash) == Some(&[true; 2]))
            .then_some(part.hash)
        })
        .expect("installed Cobalt body has paired number surfaces");
    let mut selection = baseline.clone();
    selection.parts.parts[nfsmw_data::car::customization::Category::Body as usize] = Some(body);
    let tens = catalog.number_choices[0][0].hash;
    let old_ones = catalog.number_choices[1][0].hash;
    let new_ones = catalog.number_choices[1][1].hash;
    for zone in [2, 3] {
        selection.parts.decals[zone][6..8].copy_from_slice(&[Some(tens), Some(old_ones)]);
    }
    scene.career.garage.as_mut().unwrap().baseline = selection.clone();
    for zone in [2, 3] {
        selection.parts.decals[zone][7] = Some(new_ones);
    }
    scene.refresh_garage_quote(&selection);
    assert!(scene.garage.shop_error.is_none(), "{:?}", scene.garage.shop_error);
    assert_eq!(
        scene.garage.shop_items.iter().filter(|item| item.entity == 43).count(),
        1,
        "an unchanged tens digit still bills the atomic number pair exactly once"
    );
    scene.career.garage.as_mut().unwrap().baseline = baseline.clone();
    scene.cancel_garage_session(renderer).unwrap();
}
