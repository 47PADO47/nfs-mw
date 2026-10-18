use nfsmw_career::rapsheet::{Page, RapTotals};

use super::*;

fn totals() -> RapTotals {
    RapTotals {
        bounty: 1_234_567,
        cost_to_state: 2_000,
        cars_impounded: 1,
        cars_monitored: 3,
        fines_due: 1_500,
        fines_paid: 0,
        times_evaded: 4,
        times_busted: 5,
        unserved: 6,
        served: 7,
        police_damaged: 8,
        police_destroyed: 9,
    }
}

fn text(figures: &[(u32, String)], hash: u32) -> &str {
    figures.iter().find(|(h, _)| *h == hash).map(|(_, t)| t.as_str()).unwrap_or("")
}

#[test]
fn the_button_hashes_follow_the_numbers() {
    assert_eq!(button_hash(1), Some(0xCDA0_A66B));
    assert_eq!(button_hash(6), Some(0xCDA0_A670));
    assert_eq!(button_hash(0), None);
    assert_eq!(button_hash(7), None);
}

#[test]
fn the_serial_stops_at_the_first_zero() {
    let mut serial = [0u8; 16];
    serial[..8].copy_from_slice(b"MW-12345");
    assert_eq!(serial_text(&serial), "MW-12345");
    assert_eq!(serial_text(&[b'A'; 16]), "AAAAAAAAAAAAAAAA");
}

#[test]
fn the_header_and_figures_carry_the_career_numbers() {
    let figures = figures(&totals(), "MW-1", "Rick");
    assert_eq!(text(&figures, CASE_FILE), "CASE FILE: MW-1");
    assert_eq!(text(&figures, PLAYER_NAME), "NAME: Rick");
    assert_eq!(text(&figures, TOTAL_BOUNTY), "BOUNTY: 1,234,567");
    assert_eq!(text(&figures, TOTAL_COST_STATE), "COST TO STATE: 2,000");
    assert_eq!(text(&figures, TOTAL_FINES_DUE), "FINES DUE: 1,500");
    assert_eq!(text(&figures, TOTAL_BUSTED), "TIMES BUSTED: 5");
    assert_eq!(text(&figures, CARS_MONITORED), "CARS MONITORED: 3");
    assert_eq!(figures.len(), 9);
}

#[test]
fn a_pressed_button_opens_its_page_and_is_remembered() {
    assert_eq!(destination(Some(3)), (Page::Infractions, 3));
    assert_eq!(destination(Some(6)), (Page::Rankings, 6));
}

#[test]
fn back_or_a_leave_without_a_press_goes_to_the_safe_house_and_remembers_one() {
    assert_eq!(destination(None), (Page::SafeHouseMenu, 1));
}
