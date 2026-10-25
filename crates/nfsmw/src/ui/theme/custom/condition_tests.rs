//! Tests for the vehicle condition chip: hidden when sound, compact on light damage, expanded only on a real warning,
//! and the collapse delay after the warning clears.

use super::{COLLAPSE_SECS, ChipState, ConditionMemory, LIGHT_DAMAGE, warning_label};
use crate::hud::{ConditionWarning, TireCondition, VehicleCondition};

fn condition(cosmetic: Option<f32>, tires: [TireCondition; 4], disabled: bool) -> VehicleCondition {
    VehicleCondition { cosmetic, tires, disabled, destroyed: false }
}

const SOUND: [TireCondition; 4] = [TireCondition::Sound; 4];

#[test]
fn a_sound_car_or_no_condition_hides_the_chip() {
    let mut memory = ConditionMemory::default();
    assert_eq!(memory.update(None, 0.0), ChipState::Hidden, "no simulated car, nothing shown");
    assert_eq!(memory.update(Some(&condition(Some(0.0), SOUND, false)), 0.0), ChipState::Hidden);
    assert_eq!(memory.update(Some(&condition(None, SOUND, false)), 0.0), ChipState::Hidden, "no damage model");
}

#[test]
fn light_damage_is_compact_and_never_expands() {
    let mut memory = ConditionMemory::default();
    let light = condition(Some(LIGHT_DAMAGE), SOUND, false);
    assert_eq!(memory.update(Some(&light), 0.0), ChipState::Compact);
    let scratched = condition(Some(LIGHT_DAMAGE / 2.0), SOUND, false);
    assert_eq!(memory.update(Some(&scratched), 1.0), ChipState::Hidden, "a scratch is not worth a chip");
}

#[test]
fn the_chip_expands_only_on_a_warning_with_the_wheel_of_a_tyre() {
    let mut memory = ConditionMemory::default();
    let mut tires = SOUND;
    tires[3] = TireCondition::Punctured;
    let punctured = condition(Some(0.0), tires, false);
    assert_eq!(
        memory.update(Some(&punctured), 0.0),
        ChipState::Expanded { warning: ConditionWarning::TirePunctured, wheel: Some(3) }
    );
    let heavy = condition(Some(0.9), SOUND, false);
    let expected = ChipState::Expanded { warning: ConditionWarning::HeavyDamage, wheel: None };
    assert_eq!(ConditionMemory::default().update(Some(&heavy), 0.0), expected);
    let disabled = condition(Some(0.0), SOUND, true);
    let expected = ChipState::Expanded { warning: ConditionWarning::Critical, wheel: None };
    assert_eq!(ConditionMemory::default().update(Some(&disabled), 0.0), expected);
}

#[test]
fn the_chip_collapses_a_while_after_the_warning_clears() {
    let mut memory = ConditionMemory::default();
    let heavy = condition(Some(0.9), SOUND, false);
    let repaired = condition(Some(LIGHT_DAMAGE), SOUND, false);
    memory.update(Some(&heavy), 10.0);
    let held = memory.update(Some(&repaired), 10.0 + COLLAPSE_SECS - 0.5);
    assert_eq!(
        held,
        ChipState::Expanded { warning: ConditionWarning::HeavyDamage, wheel: None },
        "held after it cleared"
    );
    assert_eq!(memory.update(Some(&repaired), 10.0 + COLLAPSE_SECS), ChipState::Compact, "collapsed");
}

#[test]
fn a_tyre_warning_names_its_wheel() {
    assert_eq!(warning_label(ConditionWarning::TireBlown, Some(0)), "Tyre blown \u{b7} Front left");
    assert_eq!(warning_label(ConditionWarning::Critical, None), "Vehicle disabled");
}
