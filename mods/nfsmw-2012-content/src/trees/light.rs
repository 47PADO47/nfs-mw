//! The lighting data a 2005 tree vertex carries: a normal and a grey vertex colour. The 2012 tree vertices have neither in a
//! form that is read (`docs/formats/chameleon-vegetation.md`), so they are made the way the 2005 trees' own are
//! (`docs/specs/world-material-routing.md`, "Tree"): the leaf cards take the normal that points away from the middle of the
//! crown, so the whole crown is lit like a ball and a card is lit the same from either side, and the vertex colour is a grey that
//! ambient light is multiplied by, darker at the bottom of the crown.

use std::collections::HashMap;

/// The vertex colour (blue, green, red, alpha) at the bottom of a crown and at its top, as grey levels. The 2005 trees' leaf cards
/// run from about 0x2F up to about 0x46.
const CROWN_DARK: f32 = 47.0;
const CROWN_LIGHT: f32 = 70.0;
/// The vertex colour of bark, the 2005 trunks' own.
pub const BARK: [u8; 4] = [56, 56, 56, 255];

fn normalised(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length < 1e-6 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

/// The middle of the points: their mean. The origin for no points.
pub fn centre(points: &[[f32; 3]]) -> [f32; 3] {
    if points.is_empty() {
        return [0.0; 3];
    }
    let mut sum = [0.0; 3];
    for point in points {
        for (total, value) in sum.iter_mut().zip(point) {
            *total += value;
        }
    }
    sum.map(|total| total / points.len() as f32)
}

/// The normals of leaf cards: each vertex's direction away from `centre`. A vertex at the centre points up.
pub fn crown_normals(positions: &[[f32; 3]], centre: [f32; 3]) -> Vec<[f32; 3]> {
    positions.iter().map(|p| normalised([p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]])).collect()
}

/// The vertex colour of a leaf at height `z` of a crown that reaches from `low` to `high`: grey, darkest at the bottom.
pub fn crown_colour(z: f32, low: f32, high: f32) -> [u8; 4] {
    let t = match high - low > 1e-3 {
        true => ((z - low) / (high - low)).clamp(0.0, 1.0),
        false => 1.0,
    };
    let grey = (CROWN_DARK + (CROWN_LIGHT - CROWN_DARK) * t).round() as u8;
    [grey, grey, grey, 255]
}

/// Smooth normals of a surface: the area-weighted mean of the faces around each position, shared by every vertex at the same
/// place (a seam keeps one normal). A vertex no face touches points up.
pub fn smooth_normals(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let key = |p: [f32; 3]| p.map(|value| (value * 1000.0).round() as i32);
    let mut sums: HashMap<[i32; 3], [f32; 3]> = HashMap::new();
    for triangle in triangles {
        let [a, b, c] = (*triangle).map(|index| positions[index as usize]);
        let (ab, ac) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let face = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
        for corner in [a, b, c] {
            let sum = sums.entry(key(corner)).or_default();
            for (total, value) in sum.iter_mut().zip(face) {
                *total += value;
            }
        }
    }
    positions.iter().map(|&p| normalised(sums.get(&key(p)).copied().unwrap_or_default())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crown_normals_point_away_from_the_middle() {
        let normals = crown_normals(&[[2.0, 0.0, 0.0], [0.0, 0.0, -3.0], [1.0, 1.0, 1.0]], [1.0, 1.0, 1.0]);
        assert_eq!(normals[0], [1.0, -1.0, -1.0].map(|v| v / 3f32.sqrt()));
        let length = 18f32.sqrt();
        assert!((normals[1][2] + 4.0 / length).abs() < 1e-6 && (normals[1][0] + 1.0 / length).abs() < 1e-6);
        assert_eq!(normals[2], [0.0, 0.0, 1.0], "a vertex at the middle points up");
    }

    #[test]
    fn the_crown_grows_lighter_towards_the_top() {
        assert_eq!(crown_colour(0.0, 0.0, 10.0), [47, 47, 47, 255]);
        assert_eq!(crown_colour(10.0, 0.0, 10.0), [70, 70, 70, 255]);
        assert_eq!(crown_colour(-5.0, 0.0, 10.0)[0], 47, "below the crown: the darkest");
        assert_eq!(crown_colour(3.0, 4.0, 4.0)[0], 70, "a flat crown is the lightest");
    }

    #[test]
    fn a_closed_surface_keeps_outward_normals_and_a_seam_shares_one() {
        // Two counter-clockwise triangles on the z = 0 plane seen from +z, with the shared corner doubled (a seam).
        let positions =
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let normals = smooth_normals(&positions, &[[0, 1, 2], [3, 4, 5]]);
        assert!(normals.iter().all(|n| *n == [0.0, 0.0, 1.0]));
        let lonely = smooth_normals(&[[0.0; 3], [1.0; 3]], &[]);
        assert_eq!(lonely[0], [0.0, 0.0, 1.0], "a vertex no face touches points up");
    }

    #[test]
    fn the_middle_of_no_points_is_the_origin() {
        assert_eq!(centre(&[]), [0.0; 3]);
        assert_eq!(centre(&[[0.0, 2.0, 4.0], [2.0, 4.0, 8.0]]), [1.0, 3.0, 6.0]);
    }
}
