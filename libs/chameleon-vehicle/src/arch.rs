//! Whether a tyre sits in the wheel arch of a body: the check that decides between the two places a 2012 car gives for an axle.
//!
//! A car's graphics spec gives the hubs the wheels are drawn at, and its vehicle info gives the axles the physics uses. On
//! 91 of the 93 cars of the install they agree with each other and with the cavity the body has for the tyre. They do not
//! where a bundle mixes parts of two cars (the BMW 1M's hubs and info are its own, its body is another car's and its arches
//! are at the info's axles) or where the data is wrong (the Venom GT Spyder's front hubs lie behind its doors). A tyre
//! drawn where the body has its arch has none of the body's vertices inside it; drawn elsewhere it has some.

use crate::Lod;

/// How much of the tyre's radius and width the test uses: the tyre is slightly shrunk, so the arch's lip, the inner
/// liner and a brake duct that touch the real tyre do not count.
const RADIUS_SHARE: f32 = 0.85;
const WIDTH_SHARE: f32 = 0.8;
/// At most this many vertices inside the tyre is an axle the tyre clearly fits (the cars that fit have none to 8).
const CLEAR: usize = 8;
/// More than this many is an axle the tyre clearly does not fit.
const BLOCKED: usize = 16;
/// The two places must be this far apart (metres) for the choice to matter.
const MIN_SHIFT: f32 = 0.03;

/// The tyre of one wheel as the test sees it, in the engine's space (x left, y up, z forward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TyreVolume {
    /// Distance of the tyre's middle from the centre line.
    pub lateral: f32,
    /// Hub height.
    pub height: f32,
    pub radius: f32,
    pub half_width: f32,
}

/// How many vertices of `body` (either side of the car) lie inside the tyre when its hub is at `z` along the car.
pub fn intrusion(body: &Lod, tyre: &TyreVolume, z: f32) -> usize {
    let reach = (RADIUS_SHARE * tyre.radius).powi(2);
    let inside = |p: [f32; 3]| {
        let (dy, dz) = (p[1] - tyre.height, p[2] - z);
        dy * dy + dz * dz < reach && (p[0].abs() - tyre.lateral).abs() < WIDTH_SHARE * tyre.half_width
    };
    body.iter().flat_map(|mesh| mesh.vertices.iter()).filter(|v| inside(v.position)).count()
}

/// Whether the tyre belongs at `other` rather than at `hub` (both `z` along the car): it does when it is a clear way off
/// the hub, the body has its vertices in the tyre at the hub and none, or almost none, at `other`.
pub fn prefer_other(body: &Lod, tyre: &TyreVolume, hub: f32, other: f32) -> bool {
    if (other - hub).abs() < MIN_SHIFT {
        return false;
    }
    if intrusion(body, tyre, hub) <= BLOCKED {
        return false;
    }
    intrusion(body, tyre, other) <= CLEAR
}

#[cfg(test)]
mod tests {
    use chameleon_gfx::Vertex;

    use super::*;
    use crate::{Appearance, PartMesh, SurfaceKind};

    fn body(points: &[[f32; 3]]) -> Lod {
        let vertices = points.iter().map(|&position| Vertex { position, color: [255; 4], uv: [0.0; 2] }).collect();
        let appearance = Appearance {
            kind: SurfaceKind::Body,
            technique: String::new(),
            diffuse_texture: None,
            diffuse_colour: None,
            alpha_test: false,
        };
        vec![PartMesh { vertices, triangles: Vec::new(), appearance }]
    }

    const TYRE: TyreVolume = TyreVolume { lateral: 0.76, height: 0.33, radius: 0.33, half_width: 0.12 };

    /// A panel of `count` vertices lying across the tyre at `z`.
    fn panel(z: f32, count: usize) -> Vec<[f32; 3]> {
        (0..count).map(|n| [0.76, 0.33 + 0.001 * n as f32, z]).collect()
    }

    #[test]
    fn vertices_inside_the_tyre_count_on_either_side() {
        let mut points = panel(1.4, 5);
        points.push([-0.76, 0.33, 1.4]);
        points.push([0.2, 0.33, 1.4]);
        points.push([0.76, 0.9, 1.4]);
        assert_eq!(intrusion(&body(&points), &TYRE, 1.4), 6);
        assert_eq!(intrusion(&body(&points), &TYRE, 1.9), 0);
    }

    #[test]
    fn a_tyre_moves_to_where_the_body_has_room_for_it() {
        let lod = body(&panel(1.4, 40));
        assert!(prefer_other(&lod, &TYRE, 1.4, 1.0));
        assert!(!prefer_other(&lod, &TYRE, 1.0, 1.4), "the room is where it already is");
    }

    #[test]
    fn a_small_difference_or_a_second_blocked_place_changes_nothing() {
        let mut points = panel(1.4, 40);
        points.extend(panel(1.0, 40));
        assert!(!prefer_other(&body(&points), &TYRE, 1.4, 1.0), "both are blocked");
        assert!(!prefer_other(&body(&panel(1.4, 40)), &TYRE, 1.4, 1.38), "2 cm is no choice");
        assert!(!prefer_other(&body(&panel(1.4, 10)), &TYRE, 1.4, 1.0), "a few vertices are the arch's lip");
    }

    /// The tyre of the left wheel of `vehicle` at the front or the rear, as the test sees it.
    fn left_tyre(vehicle: &crate::Vehicle, front: bool) -> Option<(TyreVolume, f32)> {
        let wheel = vehicle.wheels.iter().find(|w| w.position[0] > 0.0 && (w.position[2] > 0.0) == front)?;
        let extent = |axis: usize| {
            let values = wheel.lods.first()?.iter().flat_map(|m| m.vertices.iter().map(move |v| v.position[axis]));
            Some(values.fold((f32::MAX, f32::MIN), |(low, high), v| (low.min(v), high.max(v))))
        };
        let (lateral, vertical) = (extent(0)?, extent(1)?);
        let tyre = TyreVolume {
            lateral: wheel.position[0],
            height: wheel.position[1],
            radius: (vertical.1 - vertical.0) / 2.0,
            half_width: (lateral.1 - lateral.0) / 2.0,
        };
        Some((tyre, wheel.position[2]))
    }

    /// Only the axles whose hubs the body has no arch for move to the vehicle info's axle: the BMW 1M (a body that is
    /// another car's), the Venom's front and the E46 GTR's rear (10 cm behind its arch).
    #[test]
    #[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
    fn only_the_cars_with_hubs_outside_their_arches_move_an_axle() {
        use chameleon_bundle::{Bundle, Pool};
        let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
        let dir = std::path::Path::new(&dir);
        let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
        let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
        let mut moved = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
                continue;
            }
            let data = std::fs::read(entry.path()).unwrap();
            let bundle = Bundle::parse(&data).unwrap();
            let name = crate::vehicle_name(&bundle).unwrap_or_default();
            let mut pool = Pool::new();
            pool.add(bundle.clone());
            pool.add(Bundle::parse(&textures).unwrap());
            pool.add(Bundle::parse(&shaders).unwrap());
            let vehicle = crate::load(&pool, crate::find_spec(&bundle).unwrap()).unwrap();
            let Ok(stats) = crate::read_stats(&bundle) else { continue };
            for (end, front) in [(0, true), (1, false)] {
                let (tyre, hub) = left_tyre(&vehicle, front).unwrap();
                if prefer_other(&vehicle.body[0], &tyre, hub, stats.axle_z[end]) {
                    moved.insert(format!("{name} {}", ["front", "rear"][end]));
                }
            }
        }
        let wanted =
            ["BMW_1M_2012 front", "BMW_1M_2012 rear", "BMW_M3E46GTR_2001 rear", "Hennessey_VenomGTSpyder_2012 front"];
        assert_eq!(moved.into_iter().collect::<Vec<_>>(), wanted);
    }
}
