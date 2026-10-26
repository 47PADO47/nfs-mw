use super::*;

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("patrolcar", text)
}

fn wave(heat: u32, count: u32, chance: u32) -> CopWave {
    CopWave { heat, count, chance }
}

#[test]
fn the_short_form_gives_every_level_the_default_numbers() {
    let manifest = car("role = \"cop\"\nheat = [3, 2]\n").unwrap();
    assert_eq!(manifest.waves, [wave(2, DEFAULT_COUNT, DEFAULT_CHANCE), wave(3, DEFAULT_COUNT, DEFAULT_CHANCE)]);
    assert_eq!(manifest.cop_class, CopClass::Patrol);
}

#[test]
fn cop_tables_give_the_numbers_per_level() {
    let text =
        "role = \"cop\"\n[[cop]]\nheat = 3\ncount = 2\nchance = 15\n[[cop]]\nheat = [5, 4]\ncount = 6\nchance = 90\n";
    let manifest = car(text).unwrap();
    assert_eq!(manifest.waves, [wave(3, 2, 15), wave(4, 6, 90), wave(5, 6, 90)]);
    assert!(manifest.heat.is_empty(), "the short form was not used");
}

#[test]
fn a_table_leaves_out_the_numbers_it_does_not_change() {
    let manifest = car("role = \"both\"\n[[cop]]\nheat = 7\ncount = 4\n").unwrap();
    assert_eq!(manifest.waves, [wave(7, 4, DEFAULT_CHANCE)]);
    let manifest = car("role = \"cop\"\n[[cop]]\nheat = 7\nchance = 50\n").unwrap();
    assert_eq!(manifest.waves, [wave(7, DEFAULT_COUNT, 50)]);
}

#[test]
fn the_short_form_and_the_tables_mix_when_they_name_other_levels() {
    let manifest = car("role = \"cop\"\nheat = [1, 2]\n[[cop]]\nheat = 6\ncount = 3\nchance = 80\n").unwrap();
    assert_eq!(manifest.waves.iter().map(|w| w.heat).collect::<Vec<_>>(), [1, 2, 6]);
    assert_eq!(manifest.waves[2], wave(6, 3, 80));
}

#[test]
fn a_level_named_twice_is_refused_across_both_forms() {
    assert_eq!(car("role = \"cop\"\nheat = [2]\n[[cop]]\nheat = 2\n"), Err(ContentError::RepeatedHeat(2)));
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = 2\n[[cop]]\nheat = [1, 2]\n"), Err(ContentError::RepeatedHeat(2)));
}

#[test]
fn bad_numbers_are_refused() {
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = 1\ncount = 0\n"), Err(ContentError::InvalidCopCount(0)));
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = 1\ncount = 9\n"), Err(ContentError::InvalidCopCount(9)));
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = 1\nchance = 0\n"), Err(ContentError::InvalidCopChance(0)));
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = 1\nchance = 101\n"), Err(ContentError::InvalidCopChance(101)));
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = 0\n"), Err(ContentError::InvalidHeat(0)));
    assert_eq!(car("role = \"cop\"\n[[cop]]\nheat = []\n"), Err(ContentError::EmptyHeat));
}

#[test]
fn an_unknown_key_in_a_cop_table_is_refused() {
    assert!(matches!(car("role = \"cop\"\n[[cop]]\nheat = 1\nweight = 3\n"), Err(ContentError::Parse(_))));
    assert!(matches!(car("role = \"cop\"\n[[cop]]\ncount = 3\n"), Err(ContentError::Parse(_))), "heat is required");
}

#[test]
fn cop_tables_and_class_need_a_cop_role() {
    assert_eq!(car("[[cop]]\nheat = 1\n"), Err(ContentError::HeatWithoutCop));
    assert_eq!(car("cop_class = \"heavy\"\n"), Err(ContentError::HeatWithoutCop));
}

#[test]
fn the_unit_class_is_read() {
    assert_eq!(car("role = \"cop\"\ncop_class = \"heavy\"\n").unwrap().cop_class, CopClass::Heavy);
    assert_eq!(car("role = \"both\"\ncop_class = \"pursuit\"\n").unwrap().cop_class, CopClass::Pursuit);
    assert!(matches!(car("role = \"cop\"\ncop_class = \"suv\"\n"), Err(ContentError::Parse(_))));
}

#[test]
fn a_cop_with_no_level_has_no_waves_and_means_every_level() {
    let manifest = car("role = \"cop\"\n").unwrap();
    assert!(manifest.waves.is_empty() && manifest.heat.is_empty());
}
