//! One 2012 traffic bundle as a traffic car of the game: its geometry at four levels of detail, its textures, its collision
//! bounds fitted to the bundle's collision soup, its manifest, and nothing else (no sound of its own, no driver, no cameras).

use std::path::Path;

use anyhow::{Context, Result};
use chameleon_bundle::Bundle;
use chameleon_vehicle::{Lod, TrafficName, traffic_name};

use crate::config::Config;
use crate::fit::Fit;
use crate::paints::PaintChoice;
use crate::source::{CarFile, Source};
use crate::{convert, wheels};

use super::class::{self, COLOURS};
use super::manifest::{self, Entry};
use super::{ids, stats};

/// A traffic bundle with the names the mod gives its car.
pub struct Named {
    pub file: CarFile,
    pub name: TrafficName,
    pub id: String,
    pub display: String,
    /// The position of the car among the traffic cars, which chooses its colour.
    pub index: usize,
}

/// What one converted traffic car came to.
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    pub id: String,
    pub display: String,
    pub class: &'static str,
    /// Triangles of the finest body level.
    pub triangles: usize,
    /// Bytes of the car's files.
    pub bytes: usize,
}

/// Name the traffic bundles of the install: read each one's name, and give each car an id. A bundle with no name is skipped,
/// with a warning.
pub fn name_all(files: Vec<CarFile>, source: &Source) -> Vec<Named> {
    let mut taken: Vec<String> = Vec::new();
    let mut named = Vec::with_capacity(files.len());
    for file in files {
        let found = source.bundle_bytes(&file).ok().and_then(|bytes| traffic_name(&Bundle::parse(&bytes).ok()?));
        let Some(name) = found else {
            log::warn!("{}: no traffic car is named in this bundle: left out", file.path.display());
            continue;
        };
        let id = ids::id_of(&name, &taken);
        taken.push(id.clone());
        let display = ids::display_of(&name, class::facts(&name, 0.0).display);
        let index = named.len();
        named.push(Named { file, name, id, display, index });
    }
    named
}

/// Convert one traffic car and write its files under the mod folder `root`.
pub fn import(source: &Source, named: &Named, config: &Config, root: &Path) -> Result<Done> {
    let mut car = source.load(&named.file)?;
    let body = car.vehicle.body.first().context("the car has no body")?;
    let (rear_end, front_end) = body
        .iter()
        .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[2]))
        .fold((f32::MAX, f32::MIN), |(low, high), z| (low.min(z), high.max(z)));
    let axles = hub_axles(&car.vehicle).context("the car has no four hubs")?;
    let placed = wheels::place(&car.vehicle, (rear_end, front_end), axles)
        .context("the wheel layout of the car could not be measured")?;
    placed.seat_hubs(&mut car.vehicle);
    let fit = Fit::own(placed.layout);
    let facts = class::facts(&named.name, placed.layout.wheelbase);
    car.vehicle.body = arrange_levels(std::mem::take(&mut car.vehicle.body));
    let options = convert::Options {
        max_texture_size: config.max_texture_size,
        liveries: Vec::new(),
        driver: None,
        driver_poses: Vec::new(),
    };
    let built = convert::build(&car, &fit, &named.id, &options)?;
    let imported = stats::of(&placed, &facts);
    let (colour, metallic) = COLOURS[named.index % COLOURS.len()];
    let own = PaintChoice { color: colour, metallic };
    let mut paints = vec![own];
    paints.extend(
        COLOURS.iter().map(|&(color, metallic)| PaintChoice { color, metallic }).filter(|choice| *choice != own),
    );
    let plate = match built.plate {
        true => crate::plate::manifest_table(Some(PLATE_TEXT), &crate::plate::picture_name(&named.id)),
        false => String::new(),
    };
    let entry = Entry {
        class: facts.class,
        weight: class::weight(facts.class),
        max_instances: 0,
        patterns: class::patterns(facts.class).iter().map(|name| (*name).to_owned()).collect(),
    };
    let wheels = placed.manifest_table();
    let text = manifest::text(&manifest::Car {
        name: &named.display,
        manufacturer: &make_of(&named.name),
        stats: &imported,
        paints: &paints,
        plate: &plate,
        wheels: &wheels,
        traffic: &entry,
    });
    let bytes = built.geometry.len() + built.textures.len() + built.bounds.len();
    write(root, &named.id, &built, &text)?;
    Ok(Done {
        id: named.id.clone(),
        display: named.display.clone(),
        class: facts.class.name(),
        triangles: built.triangles,
        bytes,
    })
}

/// The text on every traffic plate: the 2012 base game's default registration (`plate.rs`).
const PLATE_TEXT: &str = "NEED4SPD";

/// The brand: the make of the lamp object's name, spaced into words (`GMchevrolet` is Chevrolet's own tie-in name).
fn make_of(name: &TrafficName) -> String {
    match name.make.as_str() {
        "GMchevrolet" => "Chevrolet".to_owned(),
        make => make.to_owned(),
    }
}

/// The z (forward) of the front and the rear axle of the car as its hubs have them: the mean of the two hubs in front of the
/// middle, and of the two behind it.
fn hub_axles(vehicle: &chameleon_vehicle::Vehicle) -> Option<[f32; 2]> {
    if vehicle.wheels.len() != 4 {
        return None;
    }
    let middle = vehicle.wheels.iter().map(|wheel| wheel.position[2]).sum::<f32>() / 4.0;
    let mean = |front: bool| {
        let hubs: Vec<f32> =
            vehicle.wheels.iter().map(|wheel| wheel.position[2]).filter(|z| (*z > middle) == front).collect();
        hubs.iter().sum::<f32>() / hubs.len().max(1) as f32
    };
    Some([mean(true), mean(false)])
}

/// The body levels the four letters take. The converter writes `A` and `B` from the first level, `C` from the second and `D`
/// from the third (`convert.rs`), so a traffic car's five levels (9,700, 5,100, 2,900, 1,600 and 100 triangles) are cut to
/// the first, the second and the fourth: `A` and `B` draw 9,700, `C` 5,100 and `D` 1,600. The 100-triangle level is a blob
/// for a car a few pixels wide, which the game does not draw (it looks for `A` to `D` only).
pub fn arrange_levels(levels: Vec<Lod>) -> Vec<Lod> {
    let wanted: &[usize] = match levels.len() {
        0 => &[],
        1 => &[0],
        2 => &[0, 1],
        3 => &[0, 1, 2],
        _ => &[0, 1, 3],
    };
    levels.into_iter().enumerate().filter(|(at, _)| wanted.contains(at)).map(|(_, level)| level).collect()
}

/// Write the car's files and manifest under the mod folder `root`.
fn write(root: &Path, id: &str, built: &convert::Built, manifest: &str) -> Result<()> {
    let folder = root.join("files").join("CARS").join(id);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    for (name, bytes) in
        [("GEOMETRY.BIN", &built.geometry), ("TEXTURES.BIN", &built.textures), ("BOUNDS.BIN", &built.bounds)]
    {
        std::fs::write(folder.join(name), bytes).with_context(|| format!("writing {name} of {id}"))?;
    }
    let cars = root.join("cars");
    std::fs::create_dir_all(&cars).with_context(|| format!("creating {}", cars.display()))?;
    let path = cars.join(format!("{}.toml", id.to_ascii_lowercase()));
    std::fs::write(&path, manifest).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chameleon_vehicle::{Appearance, PartMesh, SurfaceKind, Vehicle, Wheel};

    /// A level tagged by the number of its meshes.
    fn level(tag: usize) -> Lod {
        let mesh = || PartMesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
            appearance: Appearance {
                kind: SurfaceKind::Body,
                technique: String::new(),
                diffuse_texture: None,
                diffuse_colour: None,
                alpha_test: false,
            },
        };
        (0..tag).map(|_| mesh()).collect()
    }

    fn tags(levels: &[Lod]) -> Vec<usize> {
        levels.iter().map(Vec::len).collect()
    }

    #[test]
    fn five_levels_become_the_first_the_second_and_the_fourth() {
        let levels: Vec<Lod> = (1..=5).map(level).collect();
        assert_eq!(tags(&arrange_levels(levels)), [1, 2, 4]);
    }

    #[test]
    fn fewer_levels_are_kept_and_none_is_none() {
        assert_eq!(tags(&arrange_levels((1..=4).map(level).collect())), [1, 2, 4]);
        assert_eq!(tags(&arrange_levels((1..=3).map(level).collect())), [1, 2, 3]);
        assert_eq!(tags(&arrange_levels(vec![level(1), level(2)])), [1, 2]);
        assert_eq!(tags(&arrange_levels(vec![level(1)])), [1]);
        assert!(arrange_levels(Vec::new()).is_empty());
    }

    #[test]
    fn the_axles_are_the_means_of_the_hubs_in_front_and_behind() {
        let wheel = |x: f32, z: f32| Wheel { name: String::new(), position: [x, 0.3, z], lods: Vec::new() };
        let vehicle = Vehicle {
            body: Vec::new(),
            wheels: vec![wheel(0.7, 1.6), wheel(-0.7, 1.7), wheel(0.7, -0.9), wheel(-0.7, -1.0)],
        };
        let [front, rear] = hub_axles(&vehicle).unwrap();
        assert!((front - 1.65).abs() < 1e-5 && (rear + 0.95).abs() < 1e-5, "{front} {rear}");
        assert!(hub_axles(&Vehicle { body: Vec::new(), wheels: Vec::new() }).is_none());
    }

    #[test]
    fn the_brand_of_the_chevrolet_tie_in_is_chevrolet() {
        let name = |make: &str| TrafficName { make: make.to_owned(), model: "X".to_owned() };
        assert_eq!(make_of(&name("GMchevrolet")), "Chevrolet");
        assert_eq!(make_of(&name("Audi")), "Audi");
    }
}
