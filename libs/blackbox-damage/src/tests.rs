use super::*;

#[test]
fn undamaged_by_default() {
    let damage = Damage::new(10);
    assert_eq!(damage.zone_count(), 10);
    assert!(!damage.is_damaged());
    assert_eq!(damage.zone(3), Zone::default());
}

#[test]
fn zero_severity_and_unknown_zones_change_nothing() {
    let mut damage = Damage::new(2);
    assert!(!damage.impact(0, 0, true));
    assert!(!damage.impact(9, 6, true));
    assert!(!damage.is_damaged());
}

#[test]
fn light_contact_only_scrapes_paint() {
    let mut damage = Damage::new(4);
    assert!(damage.impact(1, 1, true));
    assert_eq!(damage.zone(1), Zone { panel: 0, paint: 1, glass: 0 });
}

#[test]
fn paint_deepens_with_severity_and_stops_at_max() {
    let mut damage = Damage::new(1);
    damage.impact(0, 1, false);
    damage.impact(0, 3, false);
    assert_eq!(damage.zone(0).paint, 2);
    damage.impact(0, 6, false);
    assert_eq!(damage.zone(0).paint, MAX_LEVEL);
}

#[test]
fn moderate_impact_dents_the_panel_and_cracks_glass_step_by_step() {
    let mut damage = Damage::new(1);
    damage.impact(0, 2, true);
    assert_eq!(damage.zone(0).panel, 1);
    assert_eq!(damage.zone(0).glass, 1);
    damage.impact(0, 3, true);
    assert_eq!(damage.zone(0).glass, 2);
    assert_eq!(damage.zone(0).panel, 2);
}

#[test]
fn unglazed_zone_never_cracks_glass() {
    let mut damage = Damage::new(1);
    for _ in 0..10 {
        damage.impact(0, MAX_SEVERITY, false);
    }
    assert_eq!(damage.zone(0).glass, 0);
    assert_eq!(damage.zone(0).panel, MAX_LEVEL);
}

#[test]
fn heavy_impact_shatters_glazed_zone_at_once() {
    let mut damage = Damage::new(1);
    damage.impact(0, SHATTER_SEVERITY, true);
    assert_eq!(damage.zone(0).glass, MAX_LEVEL);
}

#[test]
fn levels_are_monotonic_and_report_no_change_when_capped() {
    let mut damage = Damage::new(1);
    for _ in 0..5 {
        damage.impact(0, MAX_SEVERITY, true);
    }
    let capped = damage.clone();
    assert!(!damage.impact(0, MAX_SEVERITY, true));
    assert_eq!(damage, capped);
}

#[test]
fn puncture_reports_first_time_only() {
    let mut damage = Damage::new(1);
    assert!(damage.puncture(2));
    assert!(!damage.puncture(2));
    assert!(damage.is_punctured(2));
    assert!(!damage.is_punctured(0));
    assert!(!damage.puncture(4));
    assert!(damage.is_damaged());
}

#[test]
fn repair_clears_everything() {
    let mut damage = Damage::new(3);
    damage.impact(2, 6, true);
    damage.puncture(0);
    damage.repair();
    assert_eq!(damage, Damage::new(3));
}
