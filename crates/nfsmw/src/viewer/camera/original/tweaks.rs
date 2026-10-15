//! Speed-dependent follow responses, separate from viewport attribute pairs.

use glam::Vec3;

#[derive(Clone, Copy)]
pub(super) struct Modifier {
    scale: Vec3,
    minimum: Vec3,
    maximum: Vec3,
}

impl Modifier {
    fn new(scale: [f32; 3], minimum: [f32; 3], maximum: [f32; 3]) -> Self {
        Self { scale: Vec3::from_array(scale), minimum: Vec3::from_array(minimum), maximum: Vec3::from_array(maximum) }
    }

    pub fn apply(self, value: Vec3) -> Vec3 {
        (value * self.scale).clamp(self.minimum, self.maximum)
    }
}

pub(super) struct Tweaks {
    pub eye_duration: f32,
    pub look_duration: f32,
    pub fov_duration: f32,
    pub direction_duration: f32,
    pub up_duration: f32,
    pub eye: Modifier,
    pub look: Modifier,
    pub up_scale: Vec3,
}

impl Tweaks {
    pub fn at(mode: usize, speed: f32) -> Self {
        let t = (speed / 71.0).clamp(0.0, 1.0);
        let mix = |a: f32, b: f32| (1.0 - t) * a + t * b;
        let zero = Modifier::new([0.0; 3], [0.0; 3], [0.0; 3]);
        if mode == 0 {
            return Self {
                eye_duration: 0.0,
                look_duration: 0.0,
                fov_duration: mix(1.0, 0.5),
                direction_duration: 0.0,
                up_duration: 0.0,
                eye: zero,
                look: zero,
                up_scale: Vec3::ZERO,
            };
        }
        if mode == 1 {
            return Self {
                eye_duration: mix(0.7, 0.5),
                look_duration: mix(0.7, 0.5),
                fov_duration: mix(1.0, 0.5),
                direction_duration: mix(0.4, 0.2),
                up_duration: mix(0.7, 0.5),
                eye: Modifier::new([mix(-0.01, 0.0), mix(-0.02, 0.0), mix(0.1, 0.0)], [-0.1; 3], [0.3, 0.1, 0.1]),
                look: Modifier::new([mix(-0.1, 0.0), mix(-0.02, 0.0), 0.0], [-0.05, -0.1, 0.0], [0.2, 0.1, 0.0]),
                up_scale: Vec3::ZERO,
            };
        }
        if mode == 2 {
            return Self {
                eye_duration: mix(1.25, 0.75),
                look_duration: mix(0.4, 0.3),
                fov_duration: mix(1.0, 0.5),
                direction_duration: mix(0.4, 0.2),
                up_duration: mix(0.8, 0.4),
                eye: Modifier::new(
                    [mix(-1.0, -0.75), 0.0, 0.0],
                    [mix(-1.25, -1.0), -1.0, -1.0],
                    [mix(1.5, 1.0), 1.0, 1.0],
                ),
                look: Modifier::new(
                    [mix(-0.5, -0.2), 0.0, 0.0],
                    [mix(-2.0, -0.5), 0.0, -1.0],
                    [mix(2.0, 0.5), 0.0, 1.0],
                ),
                up_scale: Vec3::new(mix(0.004, 0.002), mix(0.004, 0.002), 0.0),
            };
        }
        if mode == 3 {
            return Self {
                eye_duration: mix(1.25, 0.75),
                look_duration: mix(0.4, 0.3),
                fov_duration: mix(1.0, 0.5),
                direction_duration: mix(0.4, 0.2),
                up_duration: mix(0.8, 0.4),
                eye: Modifier::new([mix(-1.0, -0.5), 0.0, 0.0], [-1.0; 3], [2.0, 1.0, 1.0]),
                look: Modifier::new(
                    [mix(-0.1, -0.05), 0.0, 0.0],
                    [mix(-2.0, -0.5), 0.0, -1.0],
                    [mix(2.0, 0.5), 0.0, 1.0],
                ),
                up_scale: Vec3::new(mix(0.005, 0.003), mix(0.005, 0.003), 0.0),
            };
        }
        Self {
            eye_duration: mix(1.5, 0.5),
            look_duration: mix(0.5, 0.2),
            fov_duration: mix(1.0, 0.5),
            direction_duration: mix(0.5, 0.1),
            up_duration: mix(1.0, 0.5),
            eye: Modifier::new([mix(-0.25, -0.05), 0.0, 0.0], [mix(-2.5, -0.5), -1.0, -1.0], [mix(2.0, 0.5), 1.0, 1.0]),
            look: Modifier::new([mix(-0.1, -0.02), 0.0, 0.0], [mix(-2.0, -0.5), 0.0, -1.0], [mix(2.0, 0.5), 0.0, 1.0]),
            up_scale: Vec3::new(mix(0.006, 0.002), mix(0.006, 0.002), 0.0),
        }
    }
}

pub(super) fn acceleration(value: Vec3) -> Vec3 {
    fn axis(value: f32) -> f32 {
        let sign = value.signum();
        let magnitude = value.abs().min(30.0);
        if magnitude <= 15.0 {
            return sign * magnitude * 0.2;
        }
        sign * (3.0 + (magnitude - 15.0) * 1.8)
    }
    Vec3::new(axis(value.x), axis(value.y), axis(value.z))
}

pub(super) fn top_speed_scale(speed: f32, top_speed: f32) -> f32 {
    if top_speed <= 0.0 {
        return 1.0;
    }
    let t = (speed * (4.0 / top_speed)).clamp(0.0, 4.0);
    let segment = (t as usize).min(3);
    let a = [1.0, 1.0, 1.02, 1.125][segment];
    let b = [1.0, 1.02, 1.125, 1.2][segment];
    let blend = t - segment as f32;
    (1.0 - blend) * a + blend * b
}
