use super::{Harness, MenuEvent, Page, Section, model};
use crate::audio::menu;
use crate::frontend::logic::Env;
use crate::settings::{Partial, Settings, test_caps};
use blackbox_feng::runtime::pad;

#[test]
#[ignore = "requires installed garage packages; set NFSMW_GAME_DIR"]
fn batch12_garage_carousel_exit_rejects_supplemental_actions_and_native_feedback() {
    for page in [Page::Categories { section: Section::Performance, backroom: false }, Page::Main] {
        let mut initial = model();
        if page == Page::Main {
            initial.shop = true;
            initial.shop_marker_counts.fill(1);
        }
        let mut h = Harness::open(page, initial);
        h.run(pad::ACCEPT, 1);
        h.run(0, 1);
        assert_eq!(h.top(), page.package(), "the native category exit is still pending");
        let before = h.model.draft.clone();
        h.events.clear();
        menu::take();
        for bits in [pad::BUTTON0 << 4, pad::BUTTON0, pad::LEFT, pad::RIGHT] {
            h.run(bits, 1);
            h.run(0, 1);
            assert_eq!(h.top(), page.package(), "a supplemental action must not push another page during exit");
        }
        assert!(!h.model.backroom, "pending Main exit must not enter the backroom");
        assert_eq!(h.model.draft, before);
        assert!(h.events.is_empty() && h.saved.is_empty());
        assert!(
            !menu::take().iter().any(|id| [0, 2, 4, 5, 7].contains(id)),
            "rejected actions cannot request navigation feedback"
        );
        h.run(0, 90);
        let destination = match page {
            Page::Main => "CustomizeCategory.fng",
            _ => "CustomizePerformance.fng",
        };
        assert_eq!(h.top(), destination, "the originally accepted action completes once");
    }
}

#[test]
#[ignore = "requires installed tuning/discard packages; set NFSMW_GAME_DIR"]
fn batch12_pending_tuning_discard_yes_rejects_pointer_no_until_scheduled_completion() {
    let mut initial = model();
    initial.draft.performance.levels.fill(1);
    let mut h = Harness::open(Page::Tuning, initial);
    h.press(pad::RIGHT);
    h.press(pad::BACK);
    h.press(pad::LEFT);
    assert_eq!(h.top(), "InGameDialog.fng");
    let tree = h.screens.trees().pop().unwrap();
    let no = tree.nodes.iter().find(|node| node.name_hash == crate::frontend::confirm::NO_BUTTON).expect("No answer");
    let point = no.world.transform_point3(glam::Vec3::ZERO).truncate();
    h.events.clear();
    h.run(pad::ACCEPT, 1);
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
    assert!(h.screens.click(point, &mut env).is_none(), "a pending Yes must not accept pointer No");
    assert!(!h.screens.focus_button_at(point));
    assert!(!h.screens.release_at(point, crate::frontend::ids::MOUSE_LEFT_RELEASED));
    h.flush();
    h.run(0, 1);
    assert_eq!(h.top(), "InGameDialog.fng", "the accepted activation remains scheduled");
    assert!(h.events.is_empty());
    h.run(0, 16);
    assert_eq!(h.top(), "Pause_Performance_Tuning.fng", "Yes closes the dialog on its original timer");
    assert!(h.events.is_empty(), "the retained tuning page must still complete its exit");
    h.run(0, 90);
    assert_eq!(h.events.iter().filter(|event| matches!(event, MenuEvent::Cancel)).count(), 1);
    assert!(h.saved.is_empty());
}
