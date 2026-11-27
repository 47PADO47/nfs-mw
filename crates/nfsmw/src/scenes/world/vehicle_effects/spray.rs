//! Advected cross sections of water shed by one tire, in render-world metres.
use std::collections::VecDeque;

use blackbox_gfx::EffectVertex;
use glam::Vec3;

mod tread;
mod volume;
pub(super) use tread::Tread;

#[derive(Clone, Copy)]
struct Section {
    tread: Tread,
    point: Vec3,
    outward: Vec3,
    normal: Vec3,
    velocity: Vec3,
    age: f32,
    load: f32,
    impact: f32,
    speed: f32,
    phase: f32,
    connected: bool,
    serial: u64,
}

#[derive(Default)]
pub(super) struct Spray {
    sections: VecDeque<Section>,
    tip: Option<Section>,
    connected: bool,
    phase: f32,
    previous_load: f32,
    impact: f32,
    pub births: u64,
}

impl Spray {
    pub fn stop(&mut self) {
        self.previous_load = 0.0;
        self.impact = 0.0;
        self.connected = false;
        self.tip = None;
    }

    pub fn age(&mut self, dt: f32) {
        self.impact = (self.impact - dt * 4.5).max(0.0);
        for section in self.sections.iter_mut().chain(&mut self.tip) {
            section.age += dt;
        }
        if self.tip.is_some_and(|s| s.age > 0.72) {
            self.tip = None;
        }
        while self.sections.front().is_some_and(|s| s.age > 0.72) {
            self.sections.pop_front();
        }
    }

    pub fn emit(&mut self, tread: Tread, point: Vec3, normal: Vec3, outward: Vec3, velocity: Vec3, water: f32) {
        let speed = velocity.length();
        if water < 0.015 || speed < 1.4 {
            self.stop();
            return;
        }
        self.impact = (self.impact + (water - self.previous_load).max(0.0)).min(1.0);
        self.previous_load = water;
        let mut section = Section {
            tread,
            point,
            normal,
            outward,
            velocity,
            load: water,
            impact: self.impact,
            speed,
            age: 0.0,
            phase: self.phase,
            serial: self.births,
            connected: self.connected,
        };
        if let Some(previous) = self.sections.back() {
            let distance = previous.point.distance(point);
            section.phase += distance.min(4.0);
            if section.connected && previous.age < 0.028 && distance < 1.35 {
                self.tip = Some(section);
                return;
            }
            section.connected &= distance <= speed * previous.age * 1.7 + 0.65 && previous.age < 0.14;
        }
        if section.phase > 2048.0 {
            section.phase -= 2048.0;
            section.connected = false;
        }
        self.phase = section.phase;
        self.tip = None;
        if self.sections.len() >= 40 {
            self.sections.pop_front();
        }
        self.sections.push_back(section);
        self.connected = true;
        self.births += 1;
    }

    pub fn draw(&self, seed: f32, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        let mut previous = None;
        let across = forward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = across.cross(forward).normalize_or_zero();
        let mut previous_point = None;
        for s in self.sections.iter().chain(&self.tip) {
            let t = s.age;
            let speed = (s.speed / 32.0).clamp(0.0, 1.4);
            // Above highway speed the water atomizes sooner and the visible wake becomes thinner.
            let highway = ((s.speed - 26.0) / 30.0).clamp(0.0, 1.0);
            let highway = highway * highway * (3.0 - 2.0 * highway);
            let visibility = speed.min(1.0) * (1.0 - 0.24 * highway);
            let drag_time = t / (1.0 + 3.0 * t);
            let center = s.point + s.velocity * (0.46 * drag_time) + s.outward * (t * speed * s.load);
            let width = 0.12 + t * (0.35 + speed * (1.0 + (2.6 - 0.52 * highway) * s.load));
            let lift = 0.045 + drag_time * speed * (0.5 + (4.0 - 0.65 * highway) * s.load);
            // Water carried briefly by the rotating tread lifts the outer lip of the sheet.
            let path = s.tread.displacement(s.normal, s.velocity, t, 0.56);
            let rolled = (s.load * 0.8).min(0.68) * (1.0 - t / 0.22).max(0.0);
            let near = (1.0 - (camera.distance_squared(s.point) - 144.0) / 432.0).clamp(0.0, 1.0);
            // Each deposit sheds a finite cluster. Stagger release points along the
            // moving contact patch so drops separate naturally in the chase view.
            if near > 0.0 && s.load > 0.22 && t > 0.0 && t < 0.32 && s.tread.spin.abs() > 1.0 {
                let id = s.serial.wrapping_add(u64::from(seed.to_bits()));
                let depth = ((s.load - 0.22) / 0.78).clamp(0.0, 1.0);
                let impulse = depth.sqrt() * (0.55 + 0.75 * s.impact);
                let count = 2 + (5.0 * impulse.min(1.0)) as u64;
                for i in 0..count {
                    let rand = |slot| super::particle::uniform(id, 8 * i + slot);
                    let delay = rand(0) * 0.022;
                    let age = t - delay;
                    let life = 0.12 + 0.20 * rand(1);
                    if age <= 0.0 || age >= life {
                        continue;
                    }
                    let angle = 0.23 + 0.52 * rand(2);
                    let throw = (1.8 + 5.2 * rand(3)) * impulse * speed;
                    let at = |a: f32| {
                        s.tread.displacement(s.normal, s.velocity, a, angle) + s.outward * (throw * a / (1.0 + 2.8 * a))
                    };
                    let head = at(age);
                    if head.dot(s.normal) <= 0.0 {
                        continue;
                    }
                    let tail = at((age - 0.0025).max(0.0));
                    let length = head - tail;
                    let screen = glam::Vec2::new(length.dot(across), length.dot(up));
                    let axis = screen.try_normalize().unwrap_or(glam::Vec2::Y);
                    let direction = across * axis.x + up * axis.y;
                    let transverse = across * axis.y - up * axis.x;
                    // This quad bounds a small group of streaks. Each streak covers
                    // only a fraction of its width, even when viewed along its path.
                    let spread = (0.012 + 0.010 * rand(4)) * depth.sqrt();
                    let pos = s.point + s.velocity * delay + (head + tail) * 0.5;
                    let wide = transverse * spread;
                    let high = direction * (0.55 * spread + 0.5 * screen.length().min(0.026));
                    let alpha = (170.0 * near * visibility * impulse.min(1.0) * (1.0 - (age / life).powi(2))) as u8;
                    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                        out.push(EffectVertex {
                            position: (pos + wide * x + high * y).to_array(),
                            color: [255, 255, 255, alpha],
                            uv: [x, y],
                            detail: [t, -96.0 - rand(5)],
                        });
                    }
                }
            }
            // Lose sheet coherence first; older sections carry detached aerosol.
            let breakup = ((t - 0.06) / (0.23 - 0.045 * highway)).clamp(0.0, 1.0);
            let breakup = breakup * breakup * (3.0 - 2.0 * breakup);
            if breakup > 0.0 && s.serial.is_multiple_of(2) {
                // Each wheel sheds different parcels; the same parcel keeps its trajectory as the camera turns.
                let id = s.serial.wrapping_add(u64::from(seed.to_bits()));
                let spread = super::particle::uniform(id, 3);
                let roll = super::particle::uniform(id, 4) - 0.5;
                let drift = t * t * speed * (0.45 + s.load);
                // The slowly evolving eddy is sampled at emission in world space. Adjacent
                // parcels share the flow, even after the car turns away from them.
                let phase = s.point.dot(Vec3::new(0.41, 0.29, 0.13)) - 2.7 * t;
                let (swirl, rise) = phase.sin_cos();
                let eddy = (s.outward * swirl + s.normal * (0.35 + 0.65 * rise)) * drift * 1.65;
                let pos = center
                    + eddy
                    + s.outward * (width * (1.75 * spread - 0.55) + drift * roll)
                    + s.normal * (lift * (1.0 + t * spread) + drift * (0.55 + roll));
                let radius = (0.10 + speed * t * (0.85 + s.load)) * (0.66 + 0.65 * spread);
                let alpha = (153.0 * s.load * visibility * breakup * (1.0 - t / 0.72).max(0.0).powi(2)) as u8;
                // Project a stretched parcel onto the camera plane. Looking straight along travel
                // gives a round cross-section without an unstable normalized screen direction.
                let travel = s.velocity / s.speed;
                let projected = travel - forward * travel.dot(forward);
                let wide = (across + projected * travel.dot(across) * 0.9) * radius;
                let high = (up + projected * travel.dot(up) * 0.9) * radius * (0.74 + 0.15 * spread);
                for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    out.push(EffectVertex {
                        position: (pos + wide * x + high * y).to_array(),
                        color: [255, 255, 255, alpha],
                        uv: [x, y],
                        detail: [t, -40.0 - seed - 17.0 * spread],
                    });
                }
            }
            let sheet_density = (1.0 - t / 0.72).max(0.0).powi(2) * visibility * s.load * (1.0 - breakup);
            let separation = (t * s.speed * 0.62 - 0.08) / 0.40;
            let mix = separation.clamp(0.0, 1.0);
            let volume_share = near * mix * mix * (3.0 - 2.0 * mix);
            if volume_share > 0.0
                && sheet_density > 0.002
                && s.connected
                && let Some(point) = previous_point
            {
                let along = (s.velocity - s.normal * s.normal.dot(s.velocity)).normalize_or_zero();
                if along.length_squared() > 0.9 {
                    let spacing = s.point.distance(point).min(1.8);
                    let parcel = volume::Parcel {
                        center: center.lerp(s.point + path, rolled * 0.65) + s.normal * lift * (1.0 - rolled * 0.65)
                            - along * spacing * 0.24,
                        along,
                        normal: s.normal,
                        radii: Vec3::new(0.14 + spacing * 0.7, width, 0.07 + lift * 0.55),
                    };
                    parcel.draw(camera, t, seed + s.phase % 37.0, sheet_density * volume_share, out);
                }
            }
            previous_point = Some(s.point);
            let opacity = (sheet_density * 205.0 * (1.0 - volume_share)) as u8;
            let row = [-1.0_f32, 0.0, 1.0].map(|u| {
                let sheet = center + s.outward * (width * u) + s.normal * lift * (1.0 + 0.56 * u);
                let lip = s.point + path + s.outward * (width * u + t * speed * s.load);
                let point = sheet.lerp(lip, rolled * (0.65 + 0.35 * u));
                EffectVertex {
                    position: point.to_array(),
                    color: [255, 255, 255, opacity],
                    uv: [u, s.phase],
                    detail: [t, -1.0 - seed],
                }
            });
            if let Some(last) = previous
                && s.connected
                && t < 0.34
            {
                let last: [EffectVertex; 3] = last;
                for j in 0..2 {
                    out.extend([last[j], row[j], row[j + 1], last[j], row[j + 1], last[j + 1]]);
                }
            }
            previous = Some(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_impulse_tracks_entry_and_dies_away_in_standing_water() {
        let mut wake = Spray::default();
        let tire = Tread { forward: Vec3::X, radius: 0.33, spin: 120.0 };
        let step = |wake: &mut Spray, water| {
            wake.age(1.0 / 60.0);
            wake.emit(tire, Vec3::ZERO, Vec3::Z, Vec3::Y, Vec3::X * 35.0, water);
        };
        step(&mut wake, 0.15);
        step(&mut wake, 0.90);
        assert!(wake.impact > 0.7);
        for _ in 0..30 {
            step(&mut wake, 0.90);
        }
        assert_eq!(wake.impact, 0.0);
        step(&mut wake, 0.15);
        assert_eq!(wake.impact, 0.0);
    }

    #[test]
    fn wet_contact_advances_the_tip_between_deposits_and_disconnects_cleanly() {
        let mut wake = Spray::default();
        let velocity = Vec3::X * 40.0;
        let tread = Tread { forward: Vec3::X, radius: 0.33, spin: 120.0 };
        wake.emit(tread, Vec3::ZERO, Vec3::Z, Vec3::Y, velocity, 1.0);
        wake.age(0.01);
        let at = velocity * 0.01;
        wake.emit(tread, at, Vec3::Z, Vec3::Y, velocity, 0.6);
        assert_eq!(wake.births, 1);
        assert_eq!(wake.tip.unwrap().point, at);
        assert_eq!(wake.tip.unwrap().load, 0.6);
        wake.stop();
        assert!(wake.tip.is_none());
        wake.emit(tread, Vec3::X * 20.0, Vec3::Z, Vec3::Y, velocity, 1.0);
        assert!(!wake.sections.back().unwrap().connected);
    }
}
