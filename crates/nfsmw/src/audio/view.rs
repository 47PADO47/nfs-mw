//! Actual listener/camera context for the dynamic mixer and contact voices.
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioView {
    pub car_position: Vec3,
    pub car_forward: Vec3,
    pub camera_position: Vec3,
    pub camera_forward: Vec3,
    pub camera_state: u8,
    pub view_value: i32,
}

impl Default for AudioView {
    fn default() -> Self {
        Self {
            car_position: Vec3::ZERO,
            car_forward: Vec3::X,
            camera_position: Vec3::new(-5.6, 0.0, 0.0),
            camera_forward: Vec3::X,
            camera_state: 0,
            view_value: 32767,
        }
    }
}

impl AudioView {
    pub fn distances(self, point: Vec3) -> [i32; 4] {
        let car = point - self.car_position;
        let camera = point - self.camera_position;
        [
            (car.x.hypot(car.y) * 100.0) as i32,
            (camera.x.hypot(camera.y) * 100.0) as i32,
            azimuth(car, self.car_forward),
            azimuth(camera, self.camera_forward),
        ]
    }

    pub fn point(self, forward: f32, left: f32) -> Vec3 {
        self.car_position + self.car_forward * forward + Vec3::Z.cross(self.car_forward) * left
    }
}

fn azimuth(offset: Vec3, forward: Vec3) -> i32 {
    let forward = Vec3::new(forward.x, forward.y, 0.0).normalize_or_zero();
    let offset = Vec3::new(offset.x, offset.y, 0.0);
    let angle = offset.dot(Vec3::Z.cross(forward)).atan2(offset.dot(forward));
    let turn = (angle.abs() / std::f32::consts::TAU * 65536.0) as i32 & 65535;
    if angle > 0.0 { !turn & 65535 } else { turn }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn listener_mix_uses_planar_positions_even_with_a_high_camera_and_pitched_car() {
        let flat = AudioView::default();
        let raised = AudioView {
            camera_position: flat.camera_position + Vec3::Z * 100.0,
            camera_forward: Vec3::new(1.0, 0.0, -1.0).normalize(),
            car_forward: Vec3::new(1.0, 0.0, 1.0).normalize(),
            ..flat
        };
        let point = Vec3::new(2.0, 1.0, 30.0);
        assert_eq!(flat.distances(point), raised.distances(point));
    }
}
