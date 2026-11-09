//! The second level of detail of a mod car's solids (docs/modding/car-customization.md, "The files of a car").
//!
//! The game asks for a car's solids by name at the levels `A`, `B`, `C` and `D`. A converted car whose second level is the
//! same mesh as its first (the 2012 cars: both are the finest level that fits one solid) need not write it twice: a solid
//! named `<stem>_A` that has no `<stem>_B` beside it serves as `<stem>_B` too. [`alias_second_level`] adds those copies
//! when a mod car's file is read, so everything that looks a solid up by name (the parts, the hinged parts, the posed
//! driver) finds it. A solid the file does have at level `B` is never replaced, and the cars of the game are not touched.

use std::collections::HashSet;

use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;

/// The suffix of the finest level.
const FINEST: &str = "_A";
/// The suffix of the second level.
const SECOND: &str = "_B";

/// Add a copy named `<stem>_B` of every solid named `<stem>_A` in `solids` that has no `<stem>_B` in `solids`.
pub fn alias_second_level(solids: &mut Vec<Solid>) {
    let known: HashSet<u32> = solids.iter().map(|solid| solid.name_hash).collect();
    let copies: Vec<Solid> = solids.iter().filter_map(|solid| second_level_of(solid, &known)).collect();
    solids.extend(copies);
}

/// The copy of `solid` that stands for its second level, when `solid` is a finest level whose second is not in `known`.
fn second_level_of(solid: &Solid, known: &HashSet<u32>) -> Option<Solid> {
    let stem = solid.name.strip_suffix(FINEST)?;
    let name = format!("{stem}{SECOND}");
    let name_hash = bstring_hash(&name);
    if known.contains(&name_hash) {
        return None;
    }
    Some(Solid { name, name_hash, ..solid.clone() })
}

#[cfg(test)]
mod tests {
    use blackbox_solid::{GroupSource, SolidSource, Vertex, read_solids, write_geometry_pack};

    use super::*;

    fn solid(name: &str, height: f32) -> Solid {
        let vertex =
            |position: [f32; 3]| Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [0.0; 2] };
        let corners = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, height]].map(vertex).to_vec();
        let source = SolidSource::new(name, vec![GroupSource::new(corners, vec![0, 1, 2])]);
        read_solids(&write_geometry_pack(&[source]).unwrap()).unwrap().pop().unwrap()
    }

    fn names(solids: &[Solid]) -> Vec<&str> {
        solids.iter().map(|solid| solid.name.as_str()).collect()
    }

    #[test]
    fn a_finest_level_with_no_second_level_gets_one() {
        let mut solids = vec![solid("CAR_KIT00_BODY_A", 1.0), solid("CAR_KIT00_BODY_C", 2.0)];
        alias_second_level(&mut solids);
        assert_eq!(names(&solids), ["CAR_KIT00_BODY_A", "CAR_KIT00_BODY_C", "CAR_KIT00_BODY_B"]);
        let copy = &solids[2];
        assert_eq!(copy.name_hash, bstring_hash("CAR_KIT00_BODY_B"));
        assert_eq!(copy.vertices, solids[0].vertices, "the copy is the finest level");
        assert_ne!(copy.vertices, solids[1].vertices);
    }

    #[test]
    fn a_second_level_the_file_has_is_kept() {
        let mut solids = vec![solid("CAR_KIT00_BODY_A", 1.0), solid("CAR_KIT00_BODY_B", 5.0)];
        alias_second_level(&mut solids);
        assert_eq!(names(&solids), ["CAR_KIT00_BODY_A", "CAR_KIT00_BODY_B"]);
        assert_ne!(solids[1].vertices, solids[0].vertices, "the file's own level B is not replaced");
    }

    #[test]
    fn only_finest_levels_are_copied() {
        let mut solids = vec![solid("CAR_KIT00_BODY_C", 1.0), solid("CAR_KIT00_BRAKELIGHT_ON", 1.0)];
        alias_second_level(&mut solids);
        assert_eq!(solids.len(), 2);
    }

    #[test]
    fn every_stem_gets_its_own_second_level() {
        let mut solids = vec![solid("CAR_KIT00_DRIVER_S0_A", 1.0), solid("CAR_KIT00_DRIVER_S1_A", 2.0)];
        alias_second_level(&mut solids);
        assert_eq!(solids.len(), 4);
        let copy_of = |a: &str, b: &str| {
            let a = solids.iter().find(|solid| solid.name_hash == bstring_hash(a)).unwrap();
            let b = solids.iter().find(|solid| solid.name_hash == bstring_hash(b)).unwrap();
            a.vertices == b.vertices
        };
        assert!(copy_of("CAR_KIT00_DRIVER_S0_A", "CAR_KIT00_DRIVER_S0_B"));
        assert!(copy_of("CAR_KIT00_DRIVER_S1_A", "CAR_KIT00_DRIVER_S1_B"));
        assert!(!copy_of("CAR_KIT00_DRIVER_S0_B", "CAR_KIT00_DRIVER_S1_B"));
    }
}
