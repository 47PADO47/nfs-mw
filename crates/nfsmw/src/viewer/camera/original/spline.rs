//! Retained rolling Hermite state; see the original-camera-follow spec.

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Scalar {
    pub value: f32,
    velocity: f32,
    target: f32,
    target_velocity: f32,
    time: f32,
    coefficients: [f32; 4],
    state: u8,
}

impl Scalar {
    pub fn target(&mut self, value: f32) {
        self.target = value;
        if value != self.value {
            self.state = 2;
        }
    }

    pub fn snap(&mut self) {
        self.value = self.target;
        self.velocity = self.target_velocity;
        self.state = 0;
    }

    pub fn advance(&mut self, dt: f32, duration: f32) {
        if self.state == 0 {
            return;
        }
        if self.state == 2 {
            self.time = 0.0;
            let distance = self.target - self.value;
            self.coefficients = [
                (self.target_velocity + self.velocity) - (distance + distance),
                (distance * 3.0 - self.target_velocity) - (self.velocity + self.velocity),
                self.velocity,
                self.value,
            ];
        }
        self.time = match duration <= 0.00001 {
            true => 1.0,
            false => self.time + dt / duration,
        };
        if self.time > 1.0 {
            self.time = 1.0;
            self.snap();
        }
        let [a, b, c, d] = self.coefficients;
        let t = self.time;
        self.value = ((t * a + b) * t + c) * t + d;
        self.velocity = (b + b + t * a * 3.0) * t + c;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Vector([Scalar; 3]);

impl Vector {
    pub fn value(&self) -> glam::Vec3 {
        glam::Vec3::new(self.0[0].value, self.0[1].value, self.0[2].value)
    }

    pub fn target(&mut self, value: glam::Vec3) {
        for (axis, value) in self.0.iter_mut().zip(value.to_array()) {
            axis.target(value);
        }
    }

    pub fn advance(&mut self, dt: f32, duration: f32) {
        for axis in &mut self.0 {
            axis.advance(dt, duration);
        }
    }

    pub fn snap(&mut self) {
        for axis in &mut self.0 {
            axis.snap();
        }
    }
}
