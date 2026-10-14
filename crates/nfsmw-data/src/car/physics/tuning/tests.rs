use super::*;

#[test]
fn all_original_positions_and_profiles_use_the_same_normalized_units() {
    let mut selection = TuningSelection::default();
    for active in 0..3 {
        selection.active = active;
        for step in -5..=5 {
            selection.profiles[active as usize] = [step; 7];
            let actual = selection.normalized().unwrap();
            let expected = f32::from(step) / 5.0;
            assert_eq!(actual.steering, expected);
            assert_eq!(actual.handling, expected);
            assert_eq!(actual.brakes, expected);
            assert_eq!(actual.ride_height, expected, "ride height uses inches, not five inches");
            assert_eq!(actual.aerodynamics, expected);
            assert_eq!(actual.nos, expected);
            assert_eq!(actual.induction, expected);
        }
    }
    selection.active = 3;
    assert!(selection.normalized().is_err());
    selection.active = 0;
    selection.profiles[2][4] = 6;
    assert!(selection.normalized().is_err(), "inactive profiles must also be valid");
}

#[test]
fn tuning_unlocks_from_ordinary_packages_and_retains_locked_profile_values() {
    let mut package = PerformancePackage { junkman: 0x7f, ..Default::default() };
    for category in TuningCategory::ALL {
        assert!(!category.available(package, false), "Junkman does not unlock {category:?}");
    }
    package.levels = [1; 7];
    for category in TuningCategory::ALL {
        assert_eq!(category.available(package, false), category != TuningCategory::Aerodynamics);
    }
    assert!(TuningCategory::Aerodynamics.available(PerformancePackage::default(), true));
    let selection = TuningSelection { profiles: [[5; 7], [-5; 7], [0; 7]], active: 1 };
    assert_eq!(selection.normalized().unwrap().steering, -1.0);
    assert!(!TuningCategory::Steering.available(PerformancePackage::default(), false));
    assert_eq!(selection.profiles[0], [5; 7]);
}
