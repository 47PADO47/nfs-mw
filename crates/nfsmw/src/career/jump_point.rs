//! Where a rap-sheet jump puts the car: a bounty place or a milestone's or speed trap's spawn point
//! (`docs/specs/career-rapsheet-2.md` section 5.1). The places are tagged car markers with a `Position` and a `Rotation`;
//! the car is put exactly there, facing the marker's way, as a garage exit is put on its marker.
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpPoint {
    /// The marker's position in render space.
    pub at: Vec3,
    /// The marker's heading in render space, radians from +X. `None` (no `Rotation`): the car goes on the road
    /// nearest the place instead.
    pub heading: Option<f32>,
    /// A milestone's or a bounty place's jump: the roads are cleared and a chase starts once the car stands there. A
    /// speed trap's jump only moves the car (`docs/specs/career-rapsheet-2.md` section 5.2).
    pub chase: bool,
}

impl JumpPoint {
    /// The point of a marker at `physics` (physics space) with `rotation_deg` (its `Rotation`).
    pub fn from_marker(physics: Vec3, rotation_deg: Option<f32>) -> JumpPoint {
        // Physics (x, y, z) is render (z, -x, y). A `Rotation` R faces physics (-sin R, 0, cos R), which is render
        // (cos R, sin R): the render heading is R itself.
        JumpPoint {
            at: Vec3::new(physics.z, -physics.x, physics.y),
            heading: rotation_deg.map(f32::to_radians),
            chase: false,
        }
    }

    /// The same point, with the chase a milestone's or a bounty place's jump starts.
    pub fn with_chase(self) -> JumpPoint {
        JumpPoint { chase: true, ..self }
    }

    /// The map position, render x and y.
    pub fn xy(&self) -> [f32; 2] {
        [self.at.x, self.at.y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_rotation_faces_the_car_the_way_the_data_heading_points() {
        for rotation in [-102.6971_f32, 0.0, 46.68, 185.97] {
            let point = JumpPoint::from_marker(Vec3::new(1.0, 2.0, 3.0), Some(rotation));
            let heading = point.heading.unwrap();
            // The render forward taken back to physics space is the data's own heading.
            let render = Vec3::new(heading.cos(), heading.sin(), 0.0);
            let physics = Vec3::new(-render.y, render.z, render.x);
            assert!(physics.distance(nfsmw_data::gameplay::world_heading(rotation)) < 1e-5, "rotation {rotation}");
        }
        let point = JumpPoint::from_marker(Vec3::new(1.0, 2.0, 3.0), None);
        assert_eq!(point.at, Vec3::new(3.0, -1.0, 2.0));
        assert_eq!(point.heading, None);
        assert!(!point.chase && point.with_chase().chase);
        assert_eq!(point.xy(), [3.0, -1.0]);
    }
}
