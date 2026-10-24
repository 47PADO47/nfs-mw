//! Where the world lands on the overlay: the projection from render space (x forward, y left, metres) to canvas units
//! (x right, y down, from the canvas centre), and the overlay's ellipse.
//!
//! - **North-up** (`minimap` fixed or off): world +y (north) is up the screen and world +x is right; the player's arrow
//!   turns with the car.
//! - **Heading-up** (`minimap` rotating): the world turns about the car so its heading points up the screen; the arrow
//!   stays upright.
//!
//! The scale is the ellipse's horizontal half-axis over the view radius in metres, so the view radius is what the
//! widest part of the ellipse shows.

use std::f32::consts::FRAC_PI_2;

use blackbox_ui::Clip;
use glam::Vec2;

/// The projection of one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapView {
    /// The car, render space: the point drawn at `centre`.
    pub car: [f32; 2],
    /// The angle the world is turned by, radians, counter-clockwise: 0 north-up.
    pub turn: f32,
    /// Canvas units per metre.
    pub scale: f32,
    /// The overlay's centre on the canvas.
    pub centre: Vec2,
}

impl MapView {
    /// The view of a car at `car` pointing along `heading` (render space, any length), at `scale` canvas units per
    /// metre round `centre`. Heading-up turns the heading to the top of the screen; north-up does not turn.
    pub fn new(car: [f32; 2], heading: [f32; 2], heading_up: bool, scale: f32, centre: Vec2) -> Self {
        let turn = match heading_up {
            true => FRAC_PI_2 - heading[1].atan2(heading[0]),
            false => 0.0,
        };
        Self { car, turn, scale, centre }
    }

    /// The canvas point of the world point `p`.
    pub fn to_screen(self, p: [f32; 2]) -> Vec2 {
        let turned = self.turned([p[0] - self.car[0], p[1] - self.car[1]]);
        self.centre + Vec2::new(turned.x, -turned.y) * self.scale
    }

    /// The canvas direction (unit length) of the world direction `v`; up for a direction of no length.
    pub fn direction(&self, v: [f32; 2]) -> Vec2 {
        let turned = self.turned(v);
        Vec2::new(turned.x, -turned.y).try_normalize().unwrap_or(Vec2::NEG_Y)
    }

    /// `v` turned by the view's angle.
    fn turned(&self, v: [f32; 2]) -> Vec2 {
        let (sin, cos) = self.turn.sin_cos();
        Vec2::new(v[0] * cos - v[1] * sin, v[0] * sin + v[1] * cos)
    }
}

/// The overlay's ellipse round `centre` with half-axes `radii` (canvas units), and the width of its soft edge: the
/// last [`FADE_SHARE`] of the mean radius.
pub fn ellipse(centre: Vec2, radii: Vec2) -> (Clip, f32) {
    (Clip::Ellipse { centre, radii }, FADE_SHARE * (radii.x + radii.y) / 2.0)
}

/// The share of the radius the soft edge fades over.
pub const FADE_SHARE: f32 = 0.12;
/// A mark that must stay in view (the next checkpoint, the route's end) is held this far in from the ellipse's edge, as
/// a share of the radius along its ray.
pub const PIN_SHARE: f32 = 0.9;

/// `p` held inside the ellipse round `centre` with half-axes `radii`: a point farther out along its ray than
/// [`PIN_SHARE`] of the edge is moved in to it. A point inside is unchanged.
pub fn pin(p: Vec2, centre: Vec2, radii: Vec2) -> Vec2 {
    if radii.x <= 0.0 || radii.y <= 0.0 {
        return centre;
    }
    let reach = ((p - centre) / radii).length();
    if reach <= PIN_SHARE {
        return p;
    }
    centre + (p - centre) * (PIN_SHARE / reach)
}
