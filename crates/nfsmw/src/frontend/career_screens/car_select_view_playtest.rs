//! Name-hash bindings must replace the native header's sample values in every car state.

use super::*;
use crate::frontend::career_screens::rep_sheets::playtest::Harness;
use crate::frontend::{ids::screen, logic::ScreenLogic};

struct HeaderLogic(CarSelectInfo);

impl ScreenLogic for HeaderLogic {
    fn message(&mut self, _cx: &mut Cx, _message: u32) {}
    fn start(&mut self, cx: &mut Cx) {
        cx.rt.post_to_package(cx.package, LABEL_TITLE);
        draw_car(cx, &self.0, CarSelectEntry::Plain, 50_000, 2, 3);
    }
}

#[test]
#[ignore = "requires installed career car-select header and labels; set NFSMW_GAME_DIR"]
fn career_batch13_owned_header_replaces_native_bounty_fines_release_cash_and_marker_samples() {
    let mut h = Harness::installed();
    for (status, label_a, value_a, label_b, value_b) in [
        (CarStatus::Free, LABEL_BOUNTY, "12,345", LABEL_FINES, "6,789"),
        (CarStatus::Impounded, LABEL_RELEASE_COST, "Pending", LABEL_CASH, "50,000"),
        (CarStatus::Releasable, LABEL_RELEASE_COST, "7,500", LABEL_CASH, "50,000"),
    ] {
        let info = CarSelectInfo {
            status,
            strikes: 1,
            strike_limit: 3,
            heat: 2.0,
            bounty: 12_345,
            fines: 6_789,
            release_cost: 7_500,
        };
        h.open(screen::CAR_SELECT, Box::new(HeaderLogic(info)));
        for (name, expected) in [
            (LABEL_A, crate::i18n::game_or_error(label_a)),
            (VALUE_A, value_a.to_owned()),
            (LABEL_B, crate::i18n::game_or_error(label_b)),
            (VALUE_B, value_b.to_owned()),
        ] {
            assert_eq!(h.node(name).text.as_deref(), Some(expected.as_str()), "{status:?}: name {name:#x}");
        }
        let (names, count) = match status {
            CarStatus::Free => (STRIKE_MARKER_COUNT, "2"),
            _ => (RELEASE_MARKER_COUNT, "3"),
        };
        for name in names {
            assert_eq!(h.node(name).text.as_deref().map(str::trim), Some(count), "{status:?}: marker count");
        }
    }
}
