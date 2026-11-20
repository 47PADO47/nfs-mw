//! The three coordinate systems the work crosses. The 2012 engine's is right-handed with **x left, y up, z forward**
//! (`docs/formats/chameleon-world.md`); the game's render space is x forward, y left, z up; its physics space is x right, y up,
//! z forward (a mirror image of the other two, as the game's own collision is).

/// A point or vector of engine space in the game's render space: `(z, x, y)`. A cyclic turn of the axes, so the winding of a
/// triangle is kept.
pub fn render([x, y, z]: [f32; 3]) -> [f32; 3] {
    [z, x, y]
}

/// A point or vector of engine space in the game's physics space: `(-x, y, z)`.
pub fn physics([x, y, z]: [f32; 3]) -> [f32; 3] {
    [-x, y, z]
}

/// A 4 by 4 matrix of a placed model (row vectors, the translation in the last row, `world = local * M`) as the game's scenery
/// instance takes it in render space: the three rows of the rotation (the object's x, y and z axes in render space) and the
/// position. The model's own vertices are in render space too, so the render x axis of the object is the engine z axis:
/// `R0 = render(M2)`, `R1 = render(M0)`, `R2 = render(M1)`. The fourth value of the last row is 1 in the model lists and the
/// uniform scale of a prop in the object lists (0.94 seen): it scales the rows when it is a sane number.
pub fn instance_rows(m: &[f32; 16]) -> ([[f32; 3]; 3], [f32; 3]) {
    let row = |n: usize| render([m[4 * n], m[4 * n + 1], m[4 * n + 2]]);
    let scale = if m[15].is_finite() && (0.05..=20.0).contains(&m[15]) { m[15] } else { 1.0 };
    let scaled = |n: usize| row(n).map(|v| v * scale);
    ([scaled(2), scaled(0), scaled(1)], row(3))
}

/// The position that keeps a model's vertices in place when they are moved by `-centre` (render space): the instance's rows
/// carry the centre to where it belongs.
pub fn recentred(position: [f32; 3], rows: &[[f32; 3]; 3], centre: [f32; 3]) -> [f32; 3] {
    [0, 1, 2]
        .map(|axis| position[axis] + centre[0] * rows[0][axis] + centre[1] * rows[1][axis] + centre[2] * rows[2][axis])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `local * M` in engine space, for the check.
    fn engine_world(local: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
        [0, 1, 2].map(|c| local[0] * m[c] + local[1] * m[4 + c] + local[2] * m[8 + c] + m[12 + c])
    }

    fn matrix() -> [f32; 16] {
        // A turn about the up axis (engine y) by 90 degrees, a scale of 2 on x, and a translation.
        [0.0, 0.0, -2.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 1.0]
    }

    #[test]
    fn the_axes_are_turned_in_a_cycle() {
        assert_eq!(render([1.0, 2.0, 3.0]), [3.0, 1.0, 2.0]);
        assert_eq!(physics([1.0, 2.0, 3.0]), [-1.0, 2.0, 3.0]);
    }

    #[test]
    fn an_instance_in_render_space_places_a_vertex_where_the_engine_does() {
        let m = matrix();
        let (rows, position) = instance_rows(&m);
        let local_engine = [0.5, -1.0, 2.0];
        let local = render(local_engine);
        let world =
            [0, 1, 2].map(|c| local[0] * rows[0][c] + local[1] * rows[1][c] + local[2] * rows[2][c] + position[c]);
        assert_eq!(world, render(engine_world(local_engine, &m)));
    }

    #[test]
    fn a_prop_scale_in_the_last_value_scales_the_rows_and_not_the_position() {
        let mut m = matrix();
        m[15] = 0.5;
        let (rows, position) = instance_rows(&m);
        let (full_rows, full_position) = instance_rows(&matrix());
        assert_eq!(position, full_position);
        for (half, full) in rows.iter().flatten().zip(full_rows.iter().flatten()) {
            assert_eq!(*half, full * 0.5);
        }
        m[15] = 0.0;
        assert_eq!(instance_rows(&m).0, full_rows, "a value that is not a scale leaves the rows alone");
    }

    #[test]
    fn moving_the_vertices_to_their_centre_keeps_them_in_place() {
        let (rows, position) = instance_rows(&matrix());
        let centre = [3.0, -2.0, 5.0];
        let moved = recentred(position, &rows, centre);
        let vertex = [4.0, 1.0, -1.0];
        let before =
            [0, 1, 2].map(|c| vertex[0] * rows[0][c] + vertex[1] * rows[1][c] + vertex[2] * rows[2][c] + position[c]);
        let shifted = [vertex[0] - centre[0], vertex[1] - centre[1], vertex[2] - centre[2]];
        let after =
            [0, 1, 2].map(|c| shifted[0] * rows[0][c] + shifted[1] * rows[1][c] + shifted[2] * rows[2][c] + moved[c]);
        for c in 0..3 {
            assert!((before[c] - after[c]).abs() < 1e-5, "axis {c}: {} vs {}", before[c], after[c]);
        }
    }
}
