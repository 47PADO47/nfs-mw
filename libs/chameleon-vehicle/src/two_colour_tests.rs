//! The two-colour paint packs against a real install (set `CHAMELEON_GAME_DIR`, or `MW2012_GAME_DIR`, to the install folder
//! and run with `--ignored`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chameleon_bundle::{Bundle, Pool};

use crate::{
    Mask, PaintBlend, Sheet, SurfaceKind, colour_groups, find_spec, load, load_texture, paint_packs, paint_surfaces,
    paints,
};

fn install() -> Option<PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR")).map(PathBuf::from)
}

/// Run `check` on every high-detail car bundle of the install, with the bundle's file name.
fn each_car(dir: &Path, mut check: impl FnMut(&str, &Bundle)) {
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        check(&file, &Bundle::parse(&data).unwrap());
    }
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn two_colour_packs_name_a_pair_of_their_cars_paints() {
    let Some(dir) = install() else { return };
    let gameplay = std::fs::read(dir.join("GAMELOGIC/GAMEPLAY.BNDL")).unwrap();
    let groups = paint_packs(&Bundle::parse(&gameplay).unwrap()).unwrap();
    let named = groups.iter().flat_map(|group| &group.packs).filter(|pack| pack.two_paint != 0).count();
    let (mut resolved, mut cars) = (0, 0);
    each_car(&dir, |file, bundle| {
        let pairs = colour_groups(bundle).unwrap();
        let records = paints(bundle);
        let here: usize =
            groups.iter().map(|group| group.two_colour_packs(&pairs, &records).iter().flatten().count()).sum();
        println!("{file}: {} colour groups, {here} two-colour packs", pairs.len());
        resolved += here;
        cars += usize::from(here > 0);
    });
    // 67 packs name a colour group; 62 belong to the 12 vehicles the install has a bundle for (the other 5 are a vehicle
    // of the list without a bundle).
    assert_eq!(named, 67);
    assert_eq!((resolved, cars), (62, 12));
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn the_veyron_super_sport_pairs_silver_with_dark_grey() {
    let Some(dir) = install() else { return };
    let data = std::fs::read(dir.join("VEHICLES/VEH_122692_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&data).unwrap();
    let pairs = colour_groups(&bundle).unwrap();
    let silver = pairs.iter().find(|pair| pair.id == 0x10_E35F).unwrap();
    assert_eq!((silver.first, silver.second), (0x12_462A, 0x16_3181));
    // The sixth group is in no pack.
    assert_eq!(pairs.len(), 6);
    let records = paints(&bundle);
    let colour = |id: u32| records.iter().find(|paint| paint.id == id).unwrap().srgb();
    assert_eq!(colour(silver.first), [0x64, 0x61, 0x66]);
    assert_eq!(colour(silver.second), [0x32, 0x32, 0x32]);
}

/// The pool of a car bundle with the shared textures and shaders.
fn pool_of<'a>(bundle: &Bundle<'a>, textures: &'a [u8], shaders: &'a [u8]) -> Pool<'a> {
    let mut pool = Pool::new();
    pool.add(bundle.clone());
    pool.add(Bundle::parse(textures).unwrap());
    pool.add(Bundle::parse(shaders).unwrap());
    pool
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn the_paint_surfaces_line_up_with_the_loaded_paint_meshes() {
    let Some(dir) = install() else { return };
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let mut checked = 0;
    each_car(&dir, |file, bundle| {
        let pool = pool_of(bundle, &textures, &shaders);
        let spec = find_spec(bundle).unwrap();
        let vehicle = load(&pool, spec).unwrap();
        let surfaces = paint_surfaces(&pool, spec).unwrap();
        assert_eq!(surfaces.len(), vehicle.body.len(), "{file}: levels");
        for (level, found) in vehicle.body.iter().zip(&surfaces) {
            let meshes: Vec<(&str, usize)> = level
                .iter()
                .filter(|mesh| mesh.appearance.kind == SurfaceKind::Paint)
                .map(|mesh| (mesh.appearance.technique.as_str(), mesh.triangles.len()))
                .collect();
            let seen: Vec<(&str, usize)> = found.iter().map(|s| (s.technique.as_str(), s.triangles)).collect();
            assert_eq!(meshes, seen, "{file}");
        }
        checked += 1;
    });
    assert!(checked > 50, "only {checked} cars found");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn the_cars_with_two_colour_packs_have_surfaces_that_show_them() {
    let Some(dir) = install() else { return };
    let gameplay = std::fs::read(dir.join("GAMELOGIC/GAMEPLAY.BNDL")).unwrap();
    let groups = paint_packs(&Bundle::parse(&gameplay).unwrap()).unwrap();
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let mut masked = 0;
    each_car(&dir, |file, bundle| {
        let pairs = colour_groups(bundle).unwrap();
        let records = paints(bundle);
        let packs: usize =
            groups.iter().map(|group| group.two_colour_packs(&pairs, &records).iter().flatten().count()).sum();
        if packs == 0 {
            return;
        }
        let pool = pool_of(bundle, &textures, &shaders);
        let spec = find_spec(bundle).unwrap();
        let surfaces = paint_surfaces(&pool, spec).unwrap();
        let finest = &surfaces[0];
        assert!(finest.iter().any(|s| matches!(s.blend, PaintBlend::Second | PaintBlend::Mask(_))), "{file}");
        assert!(finest.iter().all(|s| s.blend != PaintBlend::Picture), "{file}: a livery picture");
        let ids: BTreeSet<u64> = surfaces
            .iter()
            .flatten()
            .filter_map(|s| match s.blend {
                PaintBlend::Mask(id) => Some(id),
                _ => None,
            })
            .collect();
        assert!(ids.len() <= 1, "{file}: {} masks", ids.len());
        let Some(&id) = ids.first() else { return };
        let mask = Mask::read(&load_texture(&pool, id).unwrap()).unwrap();
        let picture = Sheet::with_mask(mask).bake([0.4, 0.1, 0.1], [0.9, 0.9, 0.9]);
        assert_eq!(Some(picture.data.len()), picture.slice_size(), "{file}");
        masked += 1;
    });
    assert_eq!(masked, 7, "the cars with a mask: Cobra, Challenger, Ford GT, Boss 302, Caterham, GT500, Trans Am");
}
