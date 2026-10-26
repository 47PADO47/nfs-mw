//! Liveries as variants of one car.
//!
//! The 2012 game lists a livery as a vehicle of its own: the same vehicle id with another `LiveryID`, and a bundle of its
//! own. Such a bundle is the car again with one texture replaced, the picture its paint surface draws over the paint
//! colour (`docs/formats/chameleon-customization.md`). So the conversion makes one car of a vehicle and offers the
//! pictures of its bundles as liveries in the `[customization]` table.
//!
//! This module holds the pure part: grouping bundles by vehicle, checking that a bundle is the same car as its base, and
//! naming the choices.

use std::collections::HashSet;

use chameleon_vehicle::{SurfaceKind, Vehicle};

use crate::vehicle_list::ListEntry;

/// The bundles of one vehicle, by the number in their file names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The vehicle's own bundle: the entry without a livery, or the first entry when the list has none.
    pub base: u32,
    /// The bundles of the other entries, in list order.
    pub liveries: Vec<u32>,
}

/// Group the bundles `files` by the vehicle list `entries`: one group per vehicle id, in list order. A bundle the list does
/// not mention is a group of its own, after the others, and so is a bundle the list names twice (it is placed once).
pub fn groups(entries: &[ListEntry], files: &[u32]) -> Vec<Group> {
    let mut vehicles: Vec<u32> = Vec::new();
    for entry in entries {
        if !vehicles.contains(&entry.vehicle) {
            vehicles.push(entry.vehicle);
        }
    }
    let mut placed: HashSet<u32> = HashSet::new();
    let mut out = Vec::new();
    for vehicle in vehicles {
        let mine: Vec<&ListEntry> = entries
            .iter()
            .filter(|entry| {
                entry.vehicle == vehicle && files.contains(&entry.file()) && !placed.contains(&entry.file())
            })
            .collect();
        let Some(first) = mine.iter().find(|entry| entry.is_base()).or(mine.first()) else { continue };
        let base = first.file();
        placed.insert(base);
        let mut liveries = Vec::new();
        for entry in mine {
            if placed.insert(entry.file()) {
                liveries.push(entry.file());
            }
        }
        out.push(Group { base, liveries });
    }
    for &file in files {
        if placed.insert(file) {
            out.push(Group { base: file, liveries: Vec::new() });
        }
    }
    out
}

/// What a bundle's car is, reduced to what decides whether two bundles are the same car.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// Per level of detail of the body, the vertex and triangle counts of each mesh.
    meshes: Vec<Vec<(usize, usize)>>,
    /// The texture every paint surface of the finest body level draws as its diffuse map, when they all draw the same.
    pub picture: Option<u64>,
}

/// The signature of `vehicle`.
pub fn signature(vehicle: &Vehicle) -> Signature {
    let meshes = vehicle
        .body
        .iter()
        .map(|lod| lod.iter().map(|mesh| (mesh.vertices.len(), mesh.triangles.len())).collect())
        .collect();
    Signature { meshes, picture: picture_of(vehicle) }
}

/// The one picture the paint surfaces of the finest level share, or `None` when there is no paint surface, one has no
/// picture, or they differ (a picture laid out for one surface would be wrong on another).
fn picture_of(vehicle: &Vehicle) -> Option<u64> {
    let finest = vehicle.body.first()?;
    let mut paint = finest.iter().filter(|mesh| mesh.appearance.kind == SurfaceKind::Paint);
    let first = paint.next()?.appearance.diffuse_texture?;
    paint.all(|mesh| mesh.appearance.diffuse_texture == Some(first)).then_some(first)
}

/// Whether `livery` is the car of `base` in another picture: both have a picture, and the bodies have the same meshes.
pub fn same_car(base: &Signature, livery: &Signature) -> bool {
    base.picture.is_some() && livery.picture.is_some() && base.meshes == livery.meshes
}

/// A livery as the manifest lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveryChoice {
    pub name: String,
    /// The picture's name in the car's `TEXTURES.BIN`.
    pub texture: String,
}

/// The choice for the livery at `position` (0 is the car's own bundle) of the car in `folder`. The 2012 game names its
/// liveries with pictures, not text, so they are numbered in list order.
pub fn choice(folder: &str, position: usize) -> LiveryChoice {
    LiveryChoice { name: format!("Livery {}", position + 1), texture: format!("{folder}_LIV{}", position + 1) }
}

#[cfg(test)]
mod tests {
    use chameleon_gfx::Vertex;
    use chameleon_vehicle::{Appearance, PartMesh};

    use super::*;

    fn entry(vehicle: u32, livery: u32) -> ListEntry {
        ListEntry { vehicle, livery }
    }

    #[test]
    fn a_vehicle_with_liveries_is_one_group_led_by_its_own_entry() {
        let entries = [entry(1, 0), entry(2, 2), entry(2, 20), entry(2, 21), entry(3, 0)];
        let found = groups(&entries, &[1, 2, 3, 20, 21]);
        assert_eq!(
            found,
            vec![
                Group { base: 1, liveries: vec![] },
                Group { base: 2, liveries: vec![20, 21] },
                Group { base: 3, liveries: vec![] },
            ]
        );
    }

    #[test]
    fn the_entry_without_a_livery_is_the_base_wherever_it_stands() {
        let found = groups(&[entry(2, 20), entry(2, 0)], &[2, 20]);
        assert_eq!(found, vec![Group { base: 2, liveries: vec![20] }]);
    }

    #[test]
    fn without_an_entry_of_its_own_the_first_livery_leads() {
        let found = groups(&[entry(2, 20), entry(2, 21)], &[20, 21]);
        assert_eq!(found, vec![Group { base: 20, liveries: vec![21] }]);
    }

    #[test]
    fn entries_without_a_bundle_are_left_out_and_bundles_without_an_entry_stand_alone() {
        let entries = [entry(1, 0), entry(2, 0), entry(2, 20)];
        let found = groups(&entries, &[2, 20, 77]);
        assert_eq!(found, vec![Group { base: 2, liveries: vec![20] }, Group { base: 77, liveries: vec![] }]);
    }

    #[test]
    fn a_bundle_the_list_names_twice_is_placed_once() {
        let found = groups(&[entry(1, 0), entry(2, 1)], &[1]);
        assert_eq!(found, vec![Group { base: 1, liveries: vec![] }]);
    }

    #[test]
    fn every_bundle_lands_in_exactly_one_group() {
        let entries = [entry(5, 5), entry(5, 6), entry(7, 0), entry(5, 8)];
        let files = [5, 6, 7, 8, 9];
        let mut all: Vec<u32> = Vec::new();
        for group in groups(&entries, &files) {
            all.push(group.base);
            all.extend(group.liveries);
        }
        all.sort_unstable();
        assert_eq!(all, files);
    }

    fn mesh(kind: SurfaceKind, texture: Option<u64>, triangles: usize) -> PartMesh {
        PartMesh {
            vertices: vec![Vertex { position: [0.0; 3], color: [255; 4], uv: [0.0; 2] }; 3],
            triangles: vec![[0, 1, 2]; triangles],
            appearance: Appearance {
                kind,
                technique: String::new(),
                diffuse_texture: texture,
                diffuse_colour: None,
                alpha_test: false,
            },
        }
    }

    fn car(meshes: Vec<PartMesh>) -> Vehicle {
        Vehicle { body: vec![meshes], wheels: Vec::new() }
    }

    #[test]
    fn the_picture_is_the_one_texture_all_paint_surfaces_share() {
        let shared = car(vec![
            mesh(SurfaceKind::Paint, Some(7), 4),
            mesh(SurfaceKind::Paint, Some(7), 1),
            mesh(SurfaceKind::Body, None, 2),
        ]);
        assert_eq!(signature(&shared).picture, Some(7));
        let differing = car(vec![mesh(SurfaceKind::Paint, Some(7), 4), mesh(SurfaceKind::Paint, Some(8), 1)]);
        assert_eq!(signature(&differing).picture, None);
        let bare = car(vec![mesh(SurfaceKind::Paint, Some(7), 4), mesh(SurfaceKind::Paint, None, 1)]);
        assert_eq!(signature(&bare).picture, None, "a paint surface without a picture");
        assert_eq!(signature(&car(vec![mesh(SurfaceKind::Body, Some(7), 4)])).picture, None, "no paint surface");
    }

    #[test]
    fn a_livery_is_the_same_car_when_only_the_picture_differs() {
        let body = || mesh(SurfaceKind::Body, None, 3);
        let base = signature(&car(vec![mesh(SurfaceKind::Paint, Some(1), 10), body()]));
        let livery = signature(&car(vec![mesh(SurfaceKind::Paint, Some(2), 10), body()]));
        assert!(same_car(&base, &livery));
        let other_shape = signature(&car(vec![mesh(SurfaceKind::Paint, Some(2), 11), body()]));
        assert!(!same_car(&base, &other_shape));
        let no_picture = signature(&car(vec![mesh(SurfaceKind::Paint, None, 10), body()]));
        assert!(!same_car(&base, &no_picture));
        assert!(!same_car(&no_picture, &base));
    }

    #[test]
    fn choices_are_numbered_from_one_and_fit_the_manifest() {
        let first = choice("NISSKYLINER9", 0);
        assert_eq!(first, LiveryChoice { name: "Livery 1".into(), texture: "NISSKYLINER9_LIV1".into() });
        assert_eq!(choice("X", 4).name, "Livery 5");
        assert!(first.texture.len() <= 56);
    }
}
