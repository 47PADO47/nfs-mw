//! Placing a 2012 car in the game's car space.
//!
//! The body keeps the shape it has in the 2012 files: no axis is stretched. Only the axes are permuted into the car space
//! of the game and the car is centred on its wheelbase, so the hubs sit at plus and minus half of it along x. The host
//! draws the wheels at the hubs the car's own stats give (`WheelSetup::from_stats`).

use chameleon_vehicle::Vehicle;

/// Engine space of the 2012 files (x left, y up, z forward) to the 2005 car space (x forward, y left, z up).
pub fn to_car_space(p: [f32; 3]) -> [f32; 3] {
    [p[2], p[0], p[1]]
}

/// The wheel layout of one car, in 2005 car space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Front hubs minus rear hubs along x.
    pub wheelbase: f32,
    /// Mean x of the four hubs.
    pub x_mid: f32,
    /// Mean distance from the centre line of the wheel's outer face. The game puts the rim face at this distance, so
    /// for a 2012 car it is the hub plus how far the wheel reaches outwards, not the hub itself.
    pub track: f32,
    /// Hub height.
    pub hub_z: f32,
    pub radius: f32,
}

/// The layout of a 2012 car, measured from its wheel hubs and the height of its wheel meshes.
pub fn measure(vehicle: &Vehicle) -> Option<Layout> {
    if vehicle.wheels.len() != 4 {
        return None;
    }
    let hubs: Vec<[f32; 3]> = vehicle.wheels.iter().map(|wheel| to_car_space(wheel.position)).collect();
    let mean = |f: &dyn Fn(&[f32; 3]) -> f32| hubs.iter().map(f).sum::<f32>() / 4.0;
    let x_mid = mean(&|hub| hub[0]);
    let front = hubs.iter().filter(|hub| hub[0] > x_mid).map(|hub| hub[0]).sum::<f32>() / 2.0;
    let rear = hubs.iter().filter(|hub| hub[0] <= x_mid).map(|hub| hub[0]).sum::<f32>() / 2.0;
    let radius = wheel_radius(vehicle)?;
    let outward = wheel_outward(vehicle)?;
    let layout = Layout {
        wheelbase: front - rear,
        x_mid,
        track: mean(&|hub| hub[1].abs()) + outward,
        hub_z: mean(&|hub| hub[2]),
        radius,
    };
    (layout.wheelbase > 1.0 && layout.track > 0.3).then_some(layout)
}

/// Half the height of the first wheel's most detailed meshes: the tyre is the tallest part.
fn wheel_radius(vehicle: &Vehicle) -> Option<f32> {
    let lod = vehicle.wheels.first()?.lods.first()?;
    let heights = lod.iter().flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[1]));
    let (low, high) = heights.fold((f32::MAX, f32::MIN), |(low, high), y| (low.min(y), high.max(y)));
    (high > low).then_some((high - low) / 2.0)
}

/// How far the wheel reaches outwards from its hub: the models are authored as a left wheel, so that is the largest x of
/// the most detailed meshes of a wheel on the left (positive x).
fn wheel_outward(vehicle: &Vehicle) -> Option<f32> {
    let wheel = vehicle.wheels.iter().find(|wheel| wheel.position[0] > 0.0)?;
    let reach = wheel
        .lods
        .first()?
        .iter()
        .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[0]))
        .fold(f32::MIN, f32::max);
    (reach > 0.0).then_some(reach)
}

/// How a 2012 car is moved into the game's car space (and, if `to` differs from `from`, scaled onto other hubs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    pub from: Layout,
    pub to: Layout,
    /// Scale along x, y and z of the car space.
    pub scale: [f32; 3],
}

impl Fit {
    /// The car as it is, centred on its wheelbase: nothing is scaled.
    pub fn own(from: Layout) -> Self {
        Self::new(from, Layout { x_mid: 0.0, ..from })
    }

    pub fn new(from: Layout, to: Layout) -> Self {
        let radius = to.radius / from.radius;
        Self { from, to, scale: [to.wheelbase / from.wheelbase, to.track / from.track, radius] }
    }

    /// A point of the 2012 car in the game's car space, moved onto the hubs `to` names.
    pub fn apply(&self, car_space: [f32; 3]) -> [f32; 3] {
        let [x, y, z] = car_space;
        [
            self.scale[0] * (x - self.from.x_mid) + self.to.x_mid,
            self.scale[1] * y,
            self.scale[2] * (z - self.from.hub_z) + self.to.hub_z,
        ]
    }

    /// How far the fit stretches the car, as the largest log-ratio of any axis (0 is a perfect match).
    pub fn distortion(&self) -> f32 {
        self.scale.iter().map(|s| s.ln().abs()).fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUT: Layout = Layout { wheelbase: 2.6, x_mid: 0.2, track: 0.8, hub_z: 0.35, radius: 0.35 };

    #[test]
    fn axes_permute_without_changing_handedness() {
        assert_eq!(to_car_space([1.0, 2.0, 3.0]), [3.0, 1.0, 2.0]);
    }

    #[test]
    fn an_own_fit_centres_the_wheelbase_and_scales_nothing() {
        let fit = Fit::own(LAYOUT);
        assert_eq!(fit.distortion(), 0.0);
        assert_eq!(fit.apply([LAYOUT.x_mid + 1.3, 0.5, 0.9]), [1.3, 0.5, 0.9]);
    }

    #[test]
    fn a_car_fitted_to_its_own_layout_does_not_move() {
        let fit = Fit::new(LAYOUT, LAYOUT);
        assert_eq!(fit.distortion(), 0.0);
        assert_eq!(fit.apply([1.0, 0.5, 0.9]), [1.0, 0.5, 0.9]);
    }

    #[test]
    fn hubs_land_on_the_hubs_they_are_fitted_to() {
        let stock = Layout { wheelbase: 2.8, x_mid: 0.0, track: 0.9, hub_z: 0.0, radius: 0.33 };
        let fit = Fit::new(LAYOUT, stock);
        let front_left = fit.apply([LAYOUT.x_mid + LAYOUT.wheelbase / 2.0, LAYOUT.track, LAYOUT.hub_z]);
        assert!((front_left[0] - 1.4).abs() < 1e-5);
        assert!((front_left[1] - 0.9).abs() < 1e-5);
        assert!(front_left[2].abs() < 1e-5);
    }
}
