//! The lamps of a converted car, with the real 2012 install: the body carries the head and brake lamp markers made from the
//! car's own locators, its lens strips use the game's lamp placeholders, and its textures hold the lit and unlit states of
//! both. Set `MW2012_GAME_DIR` (the 2012 install) and run with `--ignored`. With `LAMPS_KEEP_DIR` set the converted mod
//! folder is written there and kept (for a look at the car with `nfsmw view-car`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blackbox_hash::bstring_hash;
use nfsmw_2012_content::job::{self, Finished, Start};

const CAR: &str = "POR911CARR12";

fn convert(source: &Path) -> PathBuf {
    let dir = match std::env::var_os("LAMPS_KEEP_DIR") {
        Some(keep) => PathBuf::from(keep),
        None => std::env::temp_dir().join(format!("nfsmw-2012-content-lamps-{}", std::process::id())),
    };
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = ['{CAR}']\n")).unwrap();
    let Start::Started { job: running, .. } = job::start(&dir).unwrap() else { panic!("the first start converts") };
    let started = Instant::now();
    let finished = loop {
        if let Some(finished) = running.poll() {
            break finished;
        }
        assert!(started.elapsed() < Duration::from_secs(300), "the conversion did not finish");
        std::thread::sleep(Duration::from_millis(50));
    };
    let Finished::Done(report) = finished else { panic!("the conversion failed") };
    assert_eq!(report.imported.len(), 1, "skipped: {:?}", report.skipped);
    assert!(job::promote(&dir).unwrap());
    dir
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn a_converted_car_has_its_own_lamp_markers_and_lamp_surfaces() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let dir = convert(Path::new(&source));
    let folder = dir.join("files/CARS").join(CAR);

    let geometry = std::fs::read(folder.join("GEOMETRY.BIN")).unwrap();
    let (solids, errors) = blackbox_solid::read_solids_lossy(&geometry).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let body = solids.iter().find(|solid| solid.name == format!("{CAR}_KIT00_BODY_A")).unwrap();
    let find = |name: &str| body.markers.iter().filter(|m| m.name_hash == bstring_hash(name)).collect::<Vec<_>>();
    for name in ["LEFT_HEADLIGHT", "RIGHT_HEADLIGHT", "LEFT_BRAKELIGHT", "RIGHT_BRAKELIGHT", "BRAKELIGHT_CENTRE"] {
        let found = find(name);
        assert!(!found.is_empty(), "no {name} marker");
        let [x, y, z] = found[0].translation();
        println!("{name}: {x:.3} {y:.3} {z:.3}, shines {:?}", &found[0].matrix[8..11]);
        // On the car's own body: inside its length and width, above the ground, on the right side of the centre.
        assert!(x.abs() < 2.8 && y.abs() < 1.2 && (0.2..1.5).contains(&z), "{name} is off the body");
        let front = name.contains("HEAD");
        assert_eq!(x > 0.0, front, "{name} is at the wrong end");
        assert_eq!(found[0].matrix[8] > 0.0, front, "{name} shines the wrong way");
        assert!(
            name.starts_with("LEFT") && y > 0.0 || name.starts_with("RIGHT") && y < 0.0 || y.abs() < 0.06,
            "{name}"
        );
    }
    assert!(find("LEFT_REVERSE").is_empty(), "the 2012 files have no reverse lamp to take a place from");
    assert!(!find("LEFT_EXHAUST").is_empty(), "the tail pipes are still there");

    // The lens strips use the game's lamp placeholders, so the game lights them with the car's state textures.
    let placeholders =
        ["HEADLIGHT_LEFT", "HEADLIGHT_RIGHT", "BRAKELIGHT_LEFT", "BRAKELIGHT_RIGHT", "BRAKELIGHT_CENTRE"];
    let used: Vec<&str> =
        placeholders.into_iter().filter(|name| body.texture_hashes.contains(&bstring_hash(name))).collect();
    println!("lamp placeholders in the body: {used:?}");
    assert!(used.contains(&"HEADLIGHT_LEFT") && used.contains(&"BRAKELIGHT_LEFT"), "{used:?}");

    let textures = std::fs::read(folder.join("TEXTURES.BIN")).unwrap();
    let packs = blackbox_tpk::read_texture_packs(&textures).unwrap();
    // The pack stores a name in 24 bytes, so the game finds a texture by the hash of its full name.
    let names: Vec<u32> = packs.iter().flat_map(|pack| pack.textures.iter().map(|t| t.name_hash)).collect();
    for state in ["HEADLIGHT_ON", "HEADLIGHT_OFF", "BRAKELIGHT_ON", "BRAKELIGHT_OFF"] {
        let wanted = format!("{CAR}_KIT00_{state}");
        assert!(names.contains(&bstring_hash(&wanted)), "no texture {wanted}");
    }
    if std::env::var_os("LAMPS_KEEP_DIR").is_none() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
