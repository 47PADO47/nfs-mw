//! Original-menu footer evidence supplied by the user (options, career and safe house).

use super::*;
use crate::frontend::icon_menu::{IconMenu, Kind};
use crate::frontend::logic::{Footer, ScreenLogic, Sub};
use crate::input::{Bindings, InputDevice};

#[test]
fn original_reference_carousels_publish_back_then_accept() {
    for kind in [Kind::Career, Kind::SafeHouse, Kind::Categories { pause: false, mods: false }] {
        let menu = IconMenu::new(kind, Args::default());
        assert_eq!(menu.footer(), Footer::BackAccept);
        let hints = crate::frontend::prompts::hint_set(
            screen::MAIN_MENU_SUB,
            menu.footer(),
            &Bindings::default(),
            InputDevice::Xbox,
        );
        assert_eq!(hints.into_iter().map(|(_, label, _)| label).collect::<Vec<_>>(), ["Back", "Accept"]);
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn original_reference_footer_draws_back_to_the_left_of_accept() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir =
        GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").expect("installed menus"))).unwrap();
    let data = Arc::new(crate::career::CareerData::load(&dir).unwrap());
    let career = crate::career::Career::new("footer", data, &nfsmw_career::settings::NewCareer::default()).unwrap();
    let handle = crate::career::CareerHandle::shared();
    handle.set(career);
    for args in [
        Args { options: true, ..Args::default() },
        Args { sub: Sub::Career, ..Args::default() },
        Args { safehouse: true, ..Args::default() },
    ] {
        let mut h = Harness::open(screen::MAIN_MENU_SUB, args).expect("installed menus");
        h.wait(1.0);
        for device in [InputDevice::Xbox, InputDevice::Keyboard] {
            let tree = h.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
            let position = |label| {
                tree.nodes
                    .iter()
                    .find(|n| n.visible && n.text.as_deref() == Some(label))
                    .unwrap_or_else(|| panic!("missing {label} on {args:?}"))
                    .world
                    .transform_point3(glam::Vec3::ZERO)
                    .x
            };
            assert!(position("Back") < position("Accept"), "original references put Back before Accept");
        }
    }
    handle.clear();
}
