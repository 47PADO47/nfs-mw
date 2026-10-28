//! Short presentation-pose history. Length uses seconds and metres, never tick count.
use std::collections::VecDeque;

use glam::{Mat4, Vec3};
mod curve;
#[cfg(test)]
mod tests;

#[derive(Default)]
pub(super) struct Smooth {
    poses: VecDeque<(f64, Mat4)>,
    clock: f64,
    release: f32,
    burning: bool,
    ignition: f32,
}

impl Smooth {
    pub(super) fn preview(&mut self, pose: Mat4, config: crate::settings::TailPreview, time: f64) {
        self.poses.clear();
        self.clock = time;
        self.release = 0.0;
        self.burning = true;
        let phase = time.rem_euclid(2.0) as f32;
        self.ignition = match config.cycle {
            true => phase,
            false => 1.0,
        };
        if config.cycle && phase > 1.7 {
            self.release = phase - 1.7;
            self.burning = false;
        }
        let speed = f32::from(config.speed_kmh) / 3.6;
        let rate = f32::from(config.bend) * 0.03;
        for index in (0..=60).rev() {
            let age = index as f32 / 240.0;
            let angle = rate * age;
            let offset = match rate.abs() < 0.001 {
                true => Vec3::new(-speed * age, 0.0, 0.0),
                false => Vec3::new(-speed * angle.sin() / rate, speed * (1.0 - angle.cos()) / rate, 0.0),
            };
            let past = Mat4::from_rotation_translation(glam::Quat::from_rotation_z(-angle), offset);
            self.poses.push_back((time - f64::from(age), pose * past));
        }
    }
    pub(super) fn time(&self) -> f32 {
        self.clock as f32
    }
    pub(super) fn advance(&mut self, dt: f32, pose: Mat4, burning: bool) {
        if !dt.is_finite() || dt < 0.0 || !pose.is_finite() {
            *self = Self::default();
            return;
        }
        if dt > 0.1 || self.poses.back().is_some_and(|(_, p)| p.w_axis.distance(pose.w_axis) > 8.0) {
            *self = Self::default();
        }
        self.clock += f64::from(dt);
        if !burning {
            self.burning = false;
            self.release += dt;
            if self.release >= 0.12 {
                self.poses.clear();
            }
            return;
        }
        if !self.burning {
            self.ignition = 0.0;
            self.poses.clear();
        }
        self.burning = true;
        self.ignition += dt;
        self.release = 0.0;
        if self.poses.back().is_some_and(|(time, _)| *time == self.clock) {
            self.poses.pop_back();
        }
        self.poses.push_back((self.clock, pose));
        // Keep one bracketing sample before the longest offered exposure window.
        while self.poses.len() > 128 || (self.poses.len() > 2 && self.poses[1].0 < self.clock - 0.25) {
            self.poses.pop_front();
        }
    }

    pub(super) fn engagement(&self, amount: u16) -> f32 {
        let pulse = (1.0 - self.ignition / 0.16).clamp(0.0, 1.0);
        1.0 + 0.22 * f32::from(amount.min(200)) / 100.0 * pulse * pulse
    }
    pub(super) fn fade(&self) -> f32 {
        (1.0 - self.release / 0.12).clamp(0.0, 1.0)
    }

    pub(super) fn path(&self, lamp: Vec3, length: u16, exposure: u16) -> Vec<Vec3> {
        let factor = f32::from(length.min(200)) / 100.0;
        let Some(&(head_time, head)) = self.poses.back() else { return Vec::new() };
        if factor <= 0.0 || exposure == 0 || self.fade() <= 0.0 {
            return Vec::new();
        }
        let window = 0.12 * f64::from(exposure.min(200)) / 100.0 * f64::from(self.fade());
        let mut remaining = 6.0 * factor;
        let mut result = vec![head.transform_point3(lamp)];
        let mut previous_age = 0.0;
        for &(time, pose) in self.poses.iter().rev().skip(1) {
            let age = head_time - time;
            let previous = *result.last().unwrap();
            let mut point = pose.transform_point3(lamp);
            let fraction = ((window - previous_age) / (age - previous_age).max(1e-9)).clamp(0.0, 1.0);
            point = previous.lerp(point, fraction as f32);
            let distance = point.distance(previous);
            if distance > remaining {
                point = previous.lerp(point, remaining / distance);
            }
            if point.distance_squared(previous) > 1e-8 {
                result.push(point);
            }
            remaining -= distance;
            if age >= window || remaining <= 0.0 {
                break;
            }
            previous_age = age;
        }
        curve::resample(&result)
    }
}
