//! Where another object (a cop, a racer, an event icon) is drawn on the minimap. Nothing calls this yet: the
//! objects need the race and pursuit state of a later milestone. Spec: `docs/specs/hud-minimap.md` section 7.

use crate::projection::bearing_degrees;
use crate::view::{Orientation, View};

/// An element within this distance of the player (in units of the picture's width) is drawn where it is; farther
/// ones are pinned to the edge of the circle.
const EDGE: f32 = 0.06;
/// Beyond this distance an element starts to fade; at `GONE` it has faded out.
const FADE_FROM: f32 = 0.125;
const GONE: f32 = 0.23;
/// `1 / (GONE - FADE_FROM)`: the game's constant, 9.5238.
const FADE_SLOPE: f32 = 9.523_81;

/// What to do with the element's object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blip {
    /// Where its centre goes, in HUD units from the centre of the minimap, y down.
    pub offset: [f32; 2],
    /// Its rotation, degrees, clockwise on the screen.
    pub rotation: f32,
    /// Its opacity, 0..1.
    pub alpha: f32,
}

/// Places an element at `map` (its place on the picture, `Calibration::to_map`) pointing along `direction`
/// (world x, y). `picture_width` is the width of the whole picture on the HUD (1024). `pulse` keeps it opaque at
/// the edge (the selected GPS icon). An element is always shown: one outside the circle is pinned to its edge
/// and fades out between 0.125 and 0.23 picture widths from the player (alpha 0 beyond).
pub fn place(
    view: &View,
    orientation: Orientation,
    map: [f32; 2],
    direction: [f32; 2],
    pulse: bool,
    picture_width: f32,
) -> Blip {
    let turn = view.picture_turn(orientation);
    let (sin, cos) = turn.to_radians().sin_cos();
    let e = [(map[0] - view.player[0]) * view.zoom, (map[1] - view.player[1]) * view.zoom];
    let mut r = [e[0] * cos + e[1] * sin, e[1] * cos - e[0] * sin];
    let distance = (r[0] * r[0] + r[1] * r[1]).sqrt();
    let mut alpha = 1.0;
    if distance > EDGE {
        let scale = EDGE / distance;
        r = [r[0] * scale, r[1] * scale];
        if distance > FADE_FROM {
            alpha = 1.0 - (distance - FADE_FROM) * FADE_SLOPE;
        }
        if distance > GONE {
            alpha = 0.0;
        }
        if pulse {
            alpha = 1.0;
        }
    }
    Blip {
        offset: [r[0] * picture_width, r[1] * picture_width],
        rotation: bearing_degrees(direction) - turn,
        alpha: alpha.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::Calibration;
    use crate::view::Grid;

    const GRID: Grid = Grid { tiles_per_side: 8, tile_size: 128.0 };
    // 1 m = 1/8000 of the picture.
    const MAP: Calibration = Calibration { origin: [0.0, 0.0], width: 8000.0 };

    fn at(x: f32, y: f32) -> [f32; 2] {
        MAP.to_map([x, y])
    }

    fn view(heading: [f32; 2]) -> View {
        View::new(GRID, &MAP, [4000.0, 4000.0], heading, 1.0)
    }

    #[test]
    fn something_inside_the_circle_is_where_it_is() {
        let v = view([0.0, 1.0]);
        // 200 m east and 100 m north of the player: 0.025 and 0.0125 of the picture, 25.6 and 12.8 HUD units
        // at 1024 units across; north is up, so y is negative.
        let b = place(&v, Orientation::North, at(4200.0, 4100.0), [0.0, 1.0], false, 1024.0);
        assert!((b.offset[0] - 25.6).abs() < 0.01 && (b.offset[1] + 12.8).abs() < 0.01, "{b:?}");
        assert_eq!(b.alpha, 1.0);
        assert!(b.rotation.abs() < 1e-3, "pointing north on a north-up picture");
    }

    #[test]
    fn the_rotation_is_relative_to_the_picture() {
        let v = view([1.0, 0.0]);
        let east = [1.0, 0.0];
        let north_up = place(&v, Orientation::North, at(4100.0, 4000.0), east, false, 1024.0);
        assert!((north_up.rotation - 90.0).abs() < 1e-3);
        // With the picture turned to the heading, an object pointing the same way points up.
        let heading_up = place(&v, Orientation::Heading, at(4100.0, 4000.0), east, false, 1024.0);
        assert!(heading_up.rotation.abs() < 1e-3, "{}", heading_up.rotation);
        // And what lies ahead of the car is up the screen.
        assert!(heading_up.offset[0].abs() < 1e-3 && heading_up.offset[1] < 0.0, "{heading_up:?}");
    }

    #[test]
    fn far_elements_are_pinned_to_the_circle_and_fade() {
        let v = view([0.0, 1.0]);
        // 600 m away east = 0.075 of the picture: pinned at 0.06, still opaque.
        let b = place(&v, Orientation::North, at(4600.0, 4000.0), [0.0, 1.0], false, 1024.0);
        assert!((b.offset[0] - 0.06 * 1024.0).abs() < 0.01 && b.offset[1].abs() < 0.01, "{b:?}");
        assert_eq!(b.alpha, 1.0);
        // 1250 m = 0.15625: half way into the fade (0.03125 past 0.125 -> 1 - 0.2976).
        let b = place(&v, Orientation::North, at(5250.0, 4000.0), [0.0, 1.0], false, 1024.0);
        assert!((b.alpha - (1.0 - 0.03125 * 9.52381)).abs() < 1e-3, "{}", b.alpha);
        // 2000 m = 0.25: gone, unless it pulses.
        let gone = place(&v, Orientation::North, at(6000.0, 4000.0), [0.0, 1.0], false, 1024.0);
        assert_eq!(gone.alpha, 0.0);
        let pulse = place(&v, Orientation::North, at(6000.0, 4000.0), [0.0, 1.0], true, 1024.0);
        assert_eq!(pulse.alpha, 1.0);
    }
}
