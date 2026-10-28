//! The low-detail bundles (`VEHICLES/VEH_<n>_LO.BNDL`) as the `C` level of a car's body.
//!
//! Every car has a `_HI` bundle (the finest body, then two coarse levels of 800 to 1,400 and 400 to 800 triangles) and an
//! `_LO` bundle of the same car for the distance: its body at six levels, of which the third (5,000 to 6,000 triangles)
//! is a real mid-detail model between the finest (30,000 triangles) and the coarse ones, which the `_HI` bundle lacks (the first
//! two levels of the `_LO` bundle are a finer model of 11,000 to 18,000 triangles, which is no gain over `_HI`). The other
//! levels of the `_LO` bundle repeat the `_HI` bundle's own coarse levels (87 of the 92 cars that have a second one), so they add
//! nothing. The game draws `A` and `B` near, `C` for a car about 20 to 25 pixels tall and `D` below (`scenes/world/drive/car_detail.rs`); without the mid
//! level `C` is the 800-triangle model, 40 times coarser than `B`.
//!
//! The converter writes `A` and `B` from the first level of a body, `C` from the second and `D` from the third
//! (`convert.rs`), so [`add_mid`] puts the mid level of the `_LO` bundle second and the `_HI` bundle's own first coarse level
//! third. The finest level stays `A` and `B`: the hinged parts of the car are cut from it. A car with liveries keeps its own
//! levels (a livery changes the texture coordinates of its paint meshes, which the `_LO` bundle's do not follow).
//! Spec: `docs/formats/chameleon-traffic.md` ("Low-detail bundles").

use std::collections::HashMap;
use std::path::PathBuf;

use chameleon_bundle::{Bundle, Pool};
use chameleon_gfx::Texture;
use chameleon_vehicle::{Lod, find_spec, load, load_texture};

use crate::source::{CarFile, Source, SourceCar};

/// The mid level is worth a level of its own when it has no more than this share of the finest level's triangles...
const MAX_SHARE: f32 = 0.6;
/// ...and at least this many triangles: a smaller one is no better than the car's own coarse level.
const MIN_TRIANGLES: usize = 2000;

/// The mid-detail level of a car and the textures it draws that the car's own meshes do not.
pub struct Mid {
    pub level: Lod,
    pub textures: HashMap<u64, Texture>,
}

/// Triangles of a level.
fn triangles(level: &Lod) -> usize {
    level.iter().map(|mesh| mesh.triangles.len()).sum()
}

/// Whether a level of `mid` triangles is a mid-detail level for a body whose finest level has `finest`.
pub fn is_mid(mid: usize, finest: usize) -> bool {
    mid >= MIN_TRIANGLES && (mid as f32) <= MAX_SHARE * finest as f32
}

/// Which level of an `_LO` body is the mid-detail one: the one with the fewest triangles that [`is_mid`] accepts, the first of
/// them when two are alike. It is the third level in most cars, the second in one and the first in one.
pub fn pick_mid(levels: &[Lod], finest: usize) -> Option<usize> {
    let fits = |(_, level): &(usize, &Lod)| is_mid(triangles(level), finest);
    levels.iter().enumerate().filter(fits).min_by_key(|(_, level)| triangles(level)).map(|(at, _)| at)
}

/// The `_LO` bundle of the car in `file`, next to it: the same number, found without regard to case.
fn low_path(file: &CarFile) -> Option<PathBuf> {
    let wanted = format!("VEH_{}_LO.BNDL", file.number);
    let dir = file.path.parent()?;
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().eq_ignore_ascii_case(&wanted))
        .map(|entry| entry.path())
}

/// Read the mid-detail level of the car in `file` out of its `_LO` bundle. `have` are the textures the car already holds, which
/// are not read again; `finest` is the triangle count of the car's finest level. `None` when the car has no `_LO` bundle, it
/// does not read, or none of its levels is a mid level ([`pick_mid`]).
pub fn read(source: &Source, file: &CarFile, have: &HashMap<u64, Texture>, finest: usize) -> Option<Mid> {
    let data = std::fs::read(low_path(file)?).ok()?;
    let bundle = Bundle::parse(&data).ok()?;
    let spec = find_spec(&bundle)?;
    let (textures, shaders) = source.shared_bundles();
    let mut pool = Pool::new();
    pool.add(bundle.clone());
    pool.add(Bundle::parse(textures).ok()?);
    pool.add(Bundle::parse(shaders).ok()?);
    let mut levels = load(&pool, spec).ok()?.body;
    let level = levels.swap_remove(pick_mid(&levels, finest)?);
    let mut found = HashMap::new();
    for id in level.iter().filter_map(|mesh| mesh.appearance.diffuse_texture) {
        if have.contains_key(&id) || found.contains_key(&id) || !pool.contains(id) {
            continue;
        }
        if let Ok(texture) = load_texture(&pool, id) {
            found.insert(id, texture);
        }
    }
    Some(Mid { level, textures: found })
}

/// Put the mid level second in the body of `car`, between the finest level and the car's own first coarse level (the one that
/// was second), and drop the car's own third level. A body of one level gets the mid level after it.
pub fn arrange(car: &mut SourceCar, mid: Mid) {
    let mut own = std::mem::take(&mut car.vehicle.body).into_iter();
    let finest = own.next();
    let coarse = own.next();
    car.vehicle.body = finest.into_iter().chain([mid.level]).chain(coarse).collect();
    car.textures.extend(mid.textures);
}

/// Give `car` (the car of `file`) the mid level of its `_LO` bundle as its `C` level, when it has one worth using. Returns
/// whether it did.
pub fn add_mid(source: &Source, file: &CarFile, car: &mut SourceCar) -> bool {
    let Some(finest) = car.vehicle.body.first().map(triangles) else { return false };
    let Some(mid) = read(source, file, &car.textures, finest) else { return false };
    log::debug!("{}: a mid level of {} triangles from the low-detail bundle", file.number, triangles(&mid.level));
    arrange(car, mid);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chameleon_vehicle::{Appearance, PartMesh, SurfaceKind, Vehicle};

    /// A level of `count` one-triangle meshes, to count by.
    fn level(count: usize) -> Lod {
        (0..count)
            .map(|_| PartMesh {
                vertices: Vec::new(),
                triangles: vec![[0, 0, 0]],
                appearance: Appearance {
                    kind: SurfaceKind::Body,
                    technique: String::new(),
                    diffuse_texture: None,
                    diffuse_colour: None,
                    alpha_test: false,
                },
            })
            .collect()
    }

    fn car(levels: &[usize]) -> SourceCar {
        SourceCar {
            vehicle: Vehicle { body: levels.iter().map(|&n| level(n)).collect(), wheels: Vec::new() },
            textures: HashMap::new(),
            paints: Vec::new(),
            collision: None,
            hinged: None,
            lamps: Default::default(),
        }
    }

    fn counts(car: &SourceCar) -> Vec<usize> {
        car.vehicle.body.iter().map(triangles).collect()
    }

    #[test]
    fn a_level_is_a_mid_level_when_it_is_big_enough_and_well_under_the_finest() {
        assert!(is_mid(5617, 30908), "the 911: 5,617 of 30,908");
        assert!(!is_mid(827, 30908), "a coarse level is no mid level");
        assert!(!is_mid(11366, 12000), "nearly as fine as the finest: nothing gained");
        assert!(is_mid(2000, 20000) && !is_mid(1999, 20000));
    }

    #[test]
    fn the_mid_level_is_the_smallest_that_is_big_enough_and_well_under_the_finest() {
        let levels: Vec<Lod> = [16870, 16870, 5617, 827, 95, 436].map(level).to_vec();
        assert_eq!(pick_mid(&levels, 30908), Some(2), "the 911: the 5,617-triangle level");
        let odd: Vec<Lod> = [17151, 6015, 993, 100, 798].map(level).to_vec();
        assert_eq!(pick_mid(&odd, 35238), Some(1), "a body with one level of 17,000 and one of 6,000");
        let none: Vec<Lod> = [5672, 5672, 100, 468].map(level).to_vec();
        assert_eq!(pick_mid(&none, 5672), None, "the small body of the 1M has no mid level");
        assert_eq!(pick_mid(&[], 100), None);
    }

    #[test]
    fn the_mid_level_goes_second_and_the_third_own_level_goes() {
        let mut car = car(&[300, 20, 10]);
        arrange(&mut car, Mid { level: level(100), textures: HashMap::new() });
        assert_eq!(counts(&car), [300, 100, 20], "finest, mid, the car's first coarse level");
    }

    #[test]
    fn a_body_of_one_level_gets_the_mid_level_after_it() {
        let mut car = car(&[300]);
        arrange(&mut car, Mid { level: level(100), textures: HashMap::new() });
        assert_eq!(counts(&car), [300, 100]);
    }

    #[test]
    fn the_textures_of_the_mid_level_join_the_cars() {
        let mut car = car(&[300, 20]);
        let texture = |id| {
            let pixels = Texture {
                width: 4,
                height: 4,
                depth: 1,
                array_size: 1,
                mip_levels: 1,
                dxgi_format: 0,
                format: chameleon_gfx::PixelFormat::Rgba8,
                data: Vec::new(),
            };
            (id, pixels)
        };
        car.textures.extend([texture(1)]);
        arrange(&mut car, Mid { level: level(100), textures: HashMap::from([texture(2)]) });
        let mut ids: Vec<u64> = car.textures.keys().copied().collect();
        ids.sort_unstable();
        assert_eq!(ids, [1, 2]);
    }

    /// The `_LO` bundles of the install: which cars get a mid level, and that it is the 5,000 to 6,000 triangle one.
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_low_detail_bundles_give_the_cars_a_mid_level() {
        let Some(install) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let source = Source::open(std::path::Path::new(&install)).unwrap();
        let (mut with, mut without) = (0, 0);
        let (mut smallest, mut biggest) = (usize::MAX, 0);
        for file in source.cars() {
            let mut car = source.load(file).unwrap();
            let before = car.vehicle.body.iter().map(triangles).collect::<Vec<_>>();
            match add_mid(&source, file, &mut car) {
                true => {
                    with += 1;
                    let after = counts(&car);
                    assert_eq!((after[0], after[2]), (before[0], before[1]), "{}", file.number);
                    smallest = smallest.min(after[1]);
                    biggest = biggest.max(after[1]);
                }
                false => without += 1,
            }
        }
        println!("{with} cars with a mid level of {smallest} to {biggest} triangles, {without} without");
        assert!(with > 80, "{with} cars got a mid level");
    }
}
