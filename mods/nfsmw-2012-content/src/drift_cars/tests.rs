use std::time::{Duration, Instant};

use super::{KnownCars, RECHECK, applies_to, ids_in};

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw2012-drift-cars-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cars")).unwrap();
    dir
}

#[test]
fn a_car_of_the_mod_applies_in_any_case() {
    let known = ["por911carr12".to_owned(), "aud_r8".to_owned()];
    assert!(applies_to("POR911CARR12", &known));
    assert!(applies_to("aud_r8", &known));
}

#[test]
fn a_stock_car_or_another_mods_car_does_not_apply() {
    let known = ["por911carr12"];
    assert!(!applies_to("CARRERAGT", &known));
    assert!(!applies_to("POR911CARR1", &known), "only a whole id matches");
    assert!(!applies_to("", &known));
    assert!(!applies_to("POR911CARR12", &[] as &[&str]));
}

#[test]
fn the_ids_are_the_manifests_of_the_cars_folder() {
    let dir = folder("ids");
    std::fs::write(dir.join("cars/POR911CARR12.toml"), "").unwrap();
    std::fs::write(dir.join("cars/aud_r8.toml"), "").unwrap();
    std::fs::write(dir.join("cars/readme.txt"), "").unwrap();
    assert_eq!(ids_in(&dir.join("cars")), ["aud_r8", "por911carr12"]);
    assert!(ids_in(&dir.join("nothing")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_car_converted_while_the_game_runs_is_found_after_the_recheck_time() {
    let dir = folder("late");
    let mut known = KnownCars::new(&dir);
    assert!(!known.contains("AUD_R8"));
    std::fs::write(dir.join("cars/aud_r8.toml"), "").unwrap();
    assert!(!known.contains("AUD_R8"), "the folder was read just now");
    known.read_at = Instant::now().checked_sub(RECHECK + Duration::from_secs(1));
    assert!(known.contains("AUD_R8"));
    assert!(known.contains("aud_r8"), "a known car needs no new read");
    assert!(!known.contains(""));
    std::fs::remove_dir_all(&dir).unwrap();
}
