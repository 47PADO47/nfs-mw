use super::vanilla;
use super::*;
use blackbox_pursuit::Infraction;

#[test]
fn defaults_are_the_vanilla_values() {
    let d = BartenderSettings::default();
    assert_eq!(d.max_cops, 8);
    assert_eq!(d.evade_heat_adjust, 0.95);
    assert_eq!(d.time_per_level(10), 2000.0);
    assert_eq!(d.time_per_level(1), 600.0);
    assert_eq!(d.bust_speed_mps(1), 9.0 / 3.6);
    assert_eq!(d.cop_cap(5), None, "without a CopCap the install's row value is used");
    assert_eq!(d.fines.of(Infraction::Speeding), 150);
    assert_eq!(d.fines.of(Infraction::OffRoad), 75);
    assert_eq!(d.busted, BustedRules { timeout: 5.0, reach: 15.0, height_gate: 1.5 });
}

#[test]
fn an_empty_file_is_the_vanilla_tuning() {
    assert_eq!(parse(""), BartenderSettings::default());
    assert_eq!(parse("[Unknown]\nKey = 3\n"), BartenderSettings::default());
}

#[test]
fn a_missing_file_is_the_vanilla_tuning() {
    let path = Path::new("definitely/not/here/NFSMWBartenderSettings.ini");
    assert_eq!(read(path), BartenderSettings::default());
}

#[test]
fn a_file_of_the_vanilla_values_reads_back_the_defaults() {
    let mut text = String::from("[Cops]\nMaxCops = 8\n[Heat]\nEvadeAdjust = 0.95\n[Fines]\n");
    let fine_keys = [
        "Speeding",
        "Racing",
        "RecklessDriving",
        "Assault",
        "HitAndRun",
        "DamageToProperty",
        "ResistingArrest",
        "OffRoad",
    ];
    for (key, value) in fine_keys.iter().zip(vanilla::FINES) {
        text.push_str(&format!("{key} = {value}\n"));
    }
    text.push_str("[Busted]\nTimeout = 5\nReach = 15\nHeightGate = 1.5\n");
    for level in 1..=LEVELS {
        text.push_str(&format!(
            "[Heat{level}]\nTimePerLevel = {}\nBustSpeed = {}\n",
            vanilla::TIME_PER_LEVEL[level - 1],
            vanilla::BUST_SPEED_KPH[level - 1]
        ));
    }
    assert_eq!(parse(&text), BartenderSettings::default());
}

#[test]
fn each_heat_level_reads_its_own_section() {
    let text = "[Heat3]\nCopCap = 12\nTimePerLevel = 300\nBustSpeed = 30\n[Heat10]\nCopCap = 40\n";
    let got = parse(text);
    assert_eq!(got.cop_cap(3), Some(12));
    assert_eq!(got.time_per_level(3), 300.0);
    assert_eq!(got.bust_speed_mps(3), 30.0 / 3.6);
    assert_eq!(got.cop_cap(10), Some(40));
    assert_eq!(got.time_per_level(10), 2000.0, "a level without the key keeps its vanilla value");
    assert_eq!(got.cop_cap(4), None, "a level without a CopCap keeps the install's row");
}

#[test]
fn levels_outside_one_to_ten_use_the_nearest_level() {
    let got = BartenderSettings::default();
    assert_eq!(got.time_per_level(0), 600.0);
    assert_eq!(got.time_per_level(99), 2000.0);
}

#[test]
fn values_outside_their_range_are_clamped() {
    let text = "[Cops]\nMaxCops = 500\n[Heat1]\nTimePerLevel = 0\nCopCap = 0\n[Heat]\nEvadeAdjust = -1\n\
                [Busted]\nTimeout = 0\nReach = 9999\nHeightGate = 99\n";
    let got = parse(text);
    assert_eq!(got.max_cops, 64);
    assert_eq!(got.time_per_level(1), 1.0);
    assert_eq!(got.cop_cap(1), Some(1), "a cop cap of 0 clamps to 1");
    assert_eq!(got.evade_heat_adjust, 0.0);
    assert_eq!(got.busted.timeout, 0.1);
    assert_eq!(got.busted.reach, 500.0);
    assert_eq!(got.busted.height_gate, 10.0);
}

#[test]
fn malformed_values_fall_back_to_vanilla() {
    let text = "[Heat2]\nCopCap = lots\nBustSpeed = fast\n[Fines]\nSpeeding = nope\n[Busted]\nReach = far\n";
    let got = parse(text);
    assert_eq!(got.cop_cap(2), None, "a malformed cap is the install's row");
    assert_eq!(got.bust_speed_mps(2), 12.0 / 3.6, "a malformed speed is the vanilla speed");
    assert_eq!(got.fines.of(Infraction::Speeding), 150);
    assert_eq!(got.busted.reach, 15.0);
}

#[test]
fn each_fine_reads_its_own_key() {
    let got = parse("[Fines]\nOffRoad = 10\nAssault = 2500\n");
    assert_eq!(got.fines.of(Infraction::OffRoad), 10);
    assert_eq!(got.fines.of(Infraction::Assault), 2500);
    assert_eq!(got.fines.of(Infraction::Speeding), 150, "unset fines stay vanilla");
}

#[test]
fn the_busted_constants_read_their_keys() {
    let got = parse("[Busted]\nTimeout = 3\nReach = 20\nHeightGate = 2\n");
    assert_eq!(got.busted, BustedRules { timeout: 3.0, reach: 20.0, height_gate: 2.0 });
}

#[test]
fn the_max_cops_key_sets_the_chase_cap() {
    assert_eq!(parse("[Cops]\nMaxCops = 12\n").max_cops, 12);
}
