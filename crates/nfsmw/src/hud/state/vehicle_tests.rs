use super::*;

#[test]
fn a_new_car_reads_no_body_damage() {
    assert_eq!(cosmetic_level(&Damage::new(10)), 0.0);
    assert_eq!(cosmetic_level(&Damage::new(0)), 0.0, "a car without zones reads 0, not NaN");
}

#[test]
fn the_body_damage_is_the_share_of_the_panel_and_paint_levels() {
    let mut damage = Damage::new(2);
    // Severity 6: paint 3 and one panel level on zone 0, out of 2 zones * 2 aspects * 3 levels.
    damage.impact(0, 6, false);
    assert!((cosmetic_level(&damage) - 4.0 / 12.0).abs() < 1e-6);
    for _ in 0..3 {
        damage.impact(0, 6, true);
        damage.impact(1, 6, true);
    }
    assert_eq!(cosmetic_level(&damage), 1.0, "every zone at its worst, the glass left out");
}

#[test]
fn a_sound_car_has_no_warning() {
    let condition = VehicleCondition { cosmetic: Some(0.2), ..Default::default() };
    assert_eq!(condition.warning(), None);
    assert_eq!(condition.damaged_tires(), 0);
    assert_eq!(VehicleCondition::default().warning(), None, "no damage model, nothing to warn of");
}

#[test]
fn the_warning_is_the_most_serious_condition() {
    let mut condition = VehicleCondition { cosmetic: Some(HEAVY_DAMAGE), ..Default::default() };
    assert_eq!(condition.warning(), Some(ConditionWarning::HeavyDamage));
    condition.tires[2] = TireCondition::Punctured;
    assert_eq!(condition.warning(), Some(ConditionWarning::TirePunctured));
    assert_eq!(condition.damaged_tires(), 1);
    condition.tires[0] = TireCondition::Blown;
    assert_eq!(condition.warning(), Some(ConditionWarning::TireBlown));
    condition.disabled = true;
    assert_eq!(condition.warning(), Some(ConditionWarning::Critical));
    let wrecked = VehicleCondition { destroyed: true, ..Default::default() };
    assert_eq!(wrecked.warning(), Some(ConditionWarning::Critical));
}

#[test]
fn missing_vehicle_data_stays_absent() {
    let telemetry = HudTelemetry::default();
    assert_eq!(telemetry.shift_up_rpm, None);
    assert!(!telemetry.nos_burning);
    let state = crate::hud::HudState::default();
    assert_eq!(state.condition, None, "no car, no condition");
}
