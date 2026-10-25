//! Every car of a real install loads (set `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`).

use std::path::Path;

use chameleon_bundle::{Bundle, Pool};

use crate::{find_spec, load, vehicle_name};

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_high_detail_car_loads() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let dir = Path::new(&dir);
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let mut loaded = 0;
    let mut names = 0;
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let path = entry.path();
        let file = path.file_name().unwrap().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let spec = find_spec(&bundle).unwrap_or_else(|| panic!("{file}: no spec"));
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&textures).unwrap());
        pool.add(Bundle::parse(&shaders).unwrap());
        let vehicle = load(&pool, spec).unwrap_or_else(|error| panic!("{file}: {error}"));
        assert_eq!(vehicle.wheels.len(), 4, "{file}");
        assert!(vehicle.body.iter().all(|lod| !lod.is_empty()) && !vehicle.body.is_empty(), "{file}");
        assert!(vehicle.wheels.iter().all(|wheel| wheel.lods.first().is_some_and(|lod| !lod.is_empty())), "{file}");
        names += usize::from(vehicle_name(&bundle).is_some());
        loaded += 1;
    }
    assert!(loaded > 50, "only {loaded} cars found");
    println!("{loaded} cars loaded, {names} with a name");
}
