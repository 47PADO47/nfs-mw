//! Conservative two-dimensional light bins for the shared local-light shader.
//!
//! The grid is fitted to the horizontal bounds of the selected light spheres, so it always covers every light
//! whatever the selection distance. A receiver outside it is beyond the reach of every light.
use blackbox_gfx::LocalLight;
use glam::Vec2;

/// Cells per side; must match `local_light_mask` in `shadow_receiver.wgsl`.
const SIDE: i32 = 16;
/// Smallest cell edge in metres, so clustered lights do not produce needlessly tiny cells.
const MIN_CELL: f32 = 4.0;

/// The world-space placement of the grid: its minimum corner and the edge of one square cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct GridFrame {
    pub origin: Vec2,
    pub cell: f32,
}

/// Fill `grid` with one bit per light per overlapped cell, and return where the grid lies.
pub(super) fn populate(grid: &mut [[u32; 4]; 256], lights: &[LocalLight]) -> GridFrame {
    let lights = &lights[..lights.len().min(128)];
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for light in lights {
        let position = Vec2::new(light.position[0], light.position[1]);
        lo = lo.min(position - light.range);
        hi = hi.max(position + light.range);
    }
    if !lo.is_finite() || !hi.is_finite() {
        return GridFrame { origin: Vec2::ZERO, cell: MIN_CELL };
    }
    // A small margin keeps the far sphere edges inside the last cell after rounding.
    let cell = ((hi - lo).max_element() / SIDE as f32 * 1.001).max(MIN_CELL);
    let frame = GridFrame { origin: lo, cell };
    for (index, light) in lights.iter().enumerate() {
        let position = Vec2::new(light.position[0], light.position[1]);
        let first = ((position - light.range - lo) / cell).floor().as_ivec2();
        let last = ((position + light.range - lo) / cell).floor().as_ivec2();
        for y in first.y.max(0)..=last.y.min(SIDE - 1) {
            for x in first.x.max(0)..=last.x.min(SIDE - 1) {
                grid[(y * SIDE + x) as usize][index / 32] |= 1 << (index % 32);
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(x: f32, y: f32, range: f32) -> LocalLight {
        LocalLight {
            position: [x, y, 0.0],
            direction: [0.0, 0.0, -1.0],
            casts_shadow: true,
            shadow_exclude: None,
            upper_plane: None,
            range,
            outer_cos: 0.2,
            inner_cos: 0.8,
            color: [1.0; 3],
        }
    }

    /// The cell a world point falls in, as the shader computes it; `None` outside the grid.
    fn cell_of(frame: GridFrame, at: Vec2) -> Option<usize> {
        let cell = ((at - frame.origin) / frame.cell).floor().as_ivec2();
        if cell.x < 0 || cell.y < 0 || cell.x >= SIDE || cell.y >= SIDE {
            return None;
        }
        Some((cell.y * SIDE + cell.x) as usize)
    }

    #[test]
    fn bins_cover_spheres_across_cells_and_all_four_masks() {
        let mut grid = [[0; 4]; 256];
        let mut lights = vec![light(0.0, 0.0, 1.0); 128];
        lights[127] = light(256.0, 256.0, 1.0);
        let frame = populate(&mut grid, &lights);
        assert_eq!(frame.origin, Vec2::splat(-1.0));
        let near = grid[cell_of(frame, Vec2::ZERO).unwrap()];
        assert_eq!(near, [u32::MAX, u32::MAX, u32::MAX, u32::MAX >> 1]);
        assert_eq!(grid[cell_of(frame, Vec2::splat(256.0)).unwrap()], [0, 0, 0, 1 << 31]);
        assert_eq!(grid[cell_of(frame, Vec2::new(128.0, 0.0)).unwrap()], [0; 4]);
    }

    #[test]
    fn far_lights_stay_in_the_grid_and_outside_has_no_light() {
        // Lights up to 600 m away in every direction, beyond the former fixed 256 m half-width.
        let camera = Vec2::new(1000.0, -2000.0);
        let lights: Vec<_> = [(-600.0, 0.0), (600.0, 0.0), (0.0, -600.0), (0.0, 600.0), (420.0, 420.0)]
            .into_iter()
            .map(|(x, y)| light(camera.x + x, camera.y + y, 35.0))
            .collect();
        let mut grid = [[0; 4]; 256];
        let frame = populate(&mut grid, &lights);
        for (index, l) in lights.iter().enumerate() {
            let at = Vec2::new(l.position[0], l.position[1]);
            for offset in [Vec2::ZERO, Vec2::X * 34.9, -Vec2::X * 34.9, Vec2::Y * 34.9, -Vec2::Y * 34.9] {
                let cell = cell_of(frame, at + offset).expect("every lit point is inside the grid");
                assert_ne!(grid[cell][index / 32] & (1 << (index % 32)), 0, "light {index} at {offset:?}");
            }
        }
        for outside in [camera + Vec2::new(-700.0, 0.0), camera + Vec2::new(0.0, 700.0)] {
            assert_eq!(cell_of(frame, outside), None, "a receiver beyond every light is outside the grid");
        }
    }

    #[test]
    fn no_lights_leave_an_empty_grid() {
        let mut grid = [[0; 4]; 256];
        let frame = populate(&mut grid, &[]);
        assert!(frame.cell > 0.0);
        assert!(grid.iter().all(|cell| *cell == [0; 4]));
    }
}
