//! One emitter and its live particles (spawning: spec section 5; update and drawing: section 6).

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::{Curve, EmitterSpec, Rng};

/// The most live particles one emitter holds (its limit can be set lower); a spawn that would exceed it is
/// skipped.
pub const MAX_PARTICLES: usize = 1024;

/// Longest life a particle may be given, seconds.
const MAX_LIFE: f32 = 60.0;
/// The steps a second that [`EmitterSpec::damping`] is counted in.
const DAMPING_STEPS_PER_SECOND: f32 = 60.0;

/// Where the emitter is and what it hands to new particles for one step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The emitter's frame in the world: its `z` axis is the direction particles are shot along.
    pub to_world: Mat4,
    /// Velocity new particles may inherit (see [`EmitterSpec::inherit`]).
    pub inherit_velocity: Vec3,
    /// Scales the spawn rate; 0 or less spawns nothing.
    pub intensity: f32,
}

impl Default for Frame {
    fn default() -> Self {
        Self { to_world: Mat4::IDENTITY, inherit_velocity: Vec3::ZERO, intensity: 1.0 }
    }
}

/// One billboard to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    /// Quantized atlas rectangle, minimum and maximum UV coordinates.
    pub uv: [[f32; 2]; 2],
    pub position: Vec3,
    /// Half the width of the square; the corners are `position +- right * half_size +- up * half_size`
    /// after turning `right` and `up` by `angle` about the view direction.
    pub half_size: f32,
    /// Radians.
    pub angle: f32,
    /// RGBA, straight (not premultiplied).
    pub color: [u8; 4],
}

impl Sprite {
    /// The four corners in perimeter order for a camera looking along `forward` with unit `right` and
    /// `up` vectors.
    pub fn corners(&self, right: Vec3, up: Vec3, forward: Vec3) -> [Vec3; 4] {
        let turn = Quat::from_axis_angle(forward.normalize_or(Vec3::Y), self.angle);
        let (r, u) = (turn * right * self.half_size, turn * up * self.half_size);
        [self.position - r - u, self.position + r - u, self.position + r + u, self.position - r + u]
    }
}

#[derive(Debug, Clone, Copy)]
struct Particle {
    animation_phase: u16,
    animation_cell: u16,
    position: Vec3,
    velocity: Vec3,
    accel: Vec3,
    /// Seconds left.
    life: f32,
    initial_angle: f32,
    turn_back: bool,
    rotation_offset: f32,
    half_size: f32,
    angle: f32,
    color: [u8; 4],
}

/// A running emitter.
#[derive(Debug, Clone)]
pub struct Emitter {
    spec: EmitterSpec,
    size: Curve,
    angle: Curve,
    color: [Curve; 4],
    particles: Vec<Particle>,
    /// Fractional particles owed while the rate is below one per step.
    owed: f32,
    rng: Rng,
    enabled: bool,
    /// Live particles allowed; a spawn past it is skipped.
    limit: usize,
    cycle: crate::control::Control,
    forced_one_shot: bool,
}

impl Emitter {
    pub fn new(spec: EmitterSpec, seed: u64) -> Self {
        let curve = |values: [f32; 4]| Curve::new(spec.keys, values);
        let channel = |c: usize| curve(std::array::from_fn(|i| f32::from(spec.colors[i][c]) / 255.0));
        Self {
            size: curve(spec.size),
            angle: curve(spec.angle),
            color: std::array::from_fn(channel),
            spec,
            particles: Vec::new(),
            owed: 0.0,
            rng: Rng::new(seed),
            enabled: true,
            limit: MAX_PARTICLES,
            cycle: crate::control::Control::default(),
            forced_one_shot: false,
        }
    }

    /// Cap the live particles at `limit` (at least 1, at most [`MAX_PARTICLES`]) and reserve their storage, so
    /// that stepping never allocates. Particles alive beyond a lowered limit are left to die.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.clamp(1, MAX_PARTICLES);
        self.particles.reserve_exact(self.limit.saturating_sub(self.particles.len()));
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Particles the storage holds without growing.
    pub fn capacity(&self) -> usize {
        self.particles.capacity()
    }

    pub fn spec(&self) -> &EmitterSpec {
        &self.spec
    }

    /// Whether new particles spawn. Switching off lets the live ones finish their life.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Set the original forced one-shot flag. A source without an on-cycle stays continuously on.
    pub fn force_one_shot(&mut self) {
        self.forced_one_shot = true;
    }

    pub fn finished_spawning(&self) -> bool {
        !self.enabled || self.cycle.finished()
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Remove every particle and forget the owed fraction.
    pub fn clear(&mut self) {
        self.particles.clear();
        self.owed = 0.0;
        self.cycle = crate::control::Control::default();
    }

    /// Spawn (when enabled) and then advance every particle, the new ones included, by `dt` seconds.
    pub fn step(&mut self, dt: f32, frame: &Frame) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if self.enabled
            && let Some(delta) = self
                .cycle
                .emission_delta(&self.spec, self.forced_one_shot, &mut self.rng, dt)
                .filter(|delta| *delta > 0.0)
        {
            self.spawn(delta, frame);
        }
        self.advance(dt);
    }

    pub fn sprites(&self) -> impl Iterator<Item = Sprite> + '_ {
        self.particles.iter().map(|p| Sprite {
            uv: self.spec.animation.uv(p.animation_cell),
            position: p.position,
            half_size: p.half_size,
            angle: p.angle,
            color: p.color,
        })
    }

    fn spawn(&mut self, dt: f32, frame: &Frame) {
        if frame.intensity <= 0.0
            || frame.intensity < self.spec.intensity_range[0]
            || frame.intensity > self.spec.intensity_range[1]
        {
            return;
        }
        let dt = match (self.forced_one_shot || self.spec.one_shot) && !crate::control::has_on_cycle(&self.spec, true) {
            true => (self.spec.life * (1.0 - self.spec.life_variance)).max(0.0),
            false => dt,
        };
        let rate = frame.intensity * self.spec.rate;
        let mut owed_now = (rate - self.spec.rate_variance * rate) * dt;
        let mut count = owed_now as usize;
        if count == 0 {
            // The fraction is only kept while it is the whole story, as in the original.
            self.owed += owed_now.max(0.0);
            owed_now = self.owed;
            if owed_now > 1.0 {
                count = 1;
                self.owed = owed_now - 1.0;
            }
        }
        for _ in 0..count {
            if self.particles.len() >= self.limit {
                return;
            }
            let particle = self.birth(frame);
            self.particles.push(particle);
        }
    }

    fn birth(&mut self, frame: &Frame) -> Particle {
        let s = &self.spec;
        let rng = &mut self.rng;
        let axes =
            [frame.to_world.x_axis.truncate(), frame.to_world.y_axis.truncate(), frame.to_world.z_axis.truncate()];
        let speed = s.speed - s.speed * s.speed_variance + rng.below(s.speed * s.speed_variance);
        let mut accel = Vec3::ZERO;
        let mut velocity;
        if s.speed != 0.0 {
            let half = s.spread * 0.5;
            let mut angle = || (rng.below(s.spread) - half).to_radians();
            let (x, y, z) = (angle(), angle(), angle());
            velocity = Quat::from_euler(EulerRot::XYZ, x, y, z) * axes[2] * speed;
        } else if s.world_axis_velocity {
            velocity = s.velocity_start
                + Vec3::new(
                    rng.below(s.velocity_delta.x),
                    rng.below(s.velocity_delta.y),
                    rng.below(s.velocity_delta.z),
                );
            accel = s.accel_start
                + Vec3::new(rng.below(s.accel_delta.x), rng.below(s.accel_delta.y), rng.below(s.accel_delta.z));
        } else {
            let mut along = |start: f32, delta: f32| start - delta + 2.0 * rng.below(delta);
            let local = Vec3::new(
                along(s.velocity_start.x, s.velocity_delta.x),
                along(s.velocity_start.y, s.velocity_delta.y),
                along(s.velocity_start.z, s.velocity_delta.z),
            );
            velocity = axes[0] * local.x + axes[1] * local.y + axes[2] * local.z;
            accel = Vec3::new(
                along(s.accel_start.x, s.accel_delta.x),
                along(s.accel_start.y, s.accel_delta.y),
                along(s.accel_start.z, s.accel_delta.z),
            );
        }
        let volume =
            Vec3::new(rng.below(s.volume_extent.x), rng.below(s.volume_extent.y), rng.below(s.volume_extent.z));
        let position = frame.to_world.transform_point3(s.volume_center + volume - s.volume_extent * 0.5);
        let life = (s.life - s.life * s.life_variance + rng.below(s.life * s.life_variance)).min(MAX_LIFE);
        if s.inherit != 0.0 && !s.live_motion {
            let share = s.inherit - s.inherit * s.inherit_variance + rng.below(s.inherit * s.inherit_variance);
            velocity += frame.inherit_velocity * share.clamp(0.0, 1.0);
        }
        let initial_angle = (rng.below(s.initial_angle_range) - s.initial_angle_range * 0.5).to_radians();
        let turn_back = s.random_rotation_direction && rng.bit();
        let rotation_offset = rng.below(s.rotation_variance).clamp(0.0, 1.0);
        let (animation_phase, animation_cell) = s.animation.birth(rng);
        let mut particle = Particle {
            animation_phase,
            animation_cell,
            position,
            velocity,
            accel,
            life,
            initial_angle,
            turn_back,
            rotation_offset,
            half_size: 0.0,
            angle: 0.0,
            color: [0; 4],
        };
        self.look(&mut particle, 0.0, 0.0);
        particle
    }

    /// Set the size, rotation and colour a particle has at curve position `t`; `extra` is the share of the
    /// rotation curve added on top (0 at birth).
    fn look(&self, p: &mut Particle, t: f32, extra: f32) {
        let tracked = crate::track_at(&self.spec.track, t);
        let size = tracked.map_or_else(|| self.size.value(t), |look| look.0);
        let angle = tracked.map_or_else(|| self.angle.value(t), |look| look.1);
        p.half_size = (size * 0.5).max(0.0);
        let delta = angle.to_radians() * (1.0 + extra);
        p.angle = if p.turn_back { p.initial_angle - delta } else { p.initial_angle + delta };
        p.color = match tracked {
            Some((_, _, color)) => color.map(|c| c.clamp(0.0, 255.0).round() as u8),
            None => std::array::from_fn(|c| (self.color[c].value(t) * 255.0).clamp(0.0, 255.0) as u8),
        };
    }

    fn advance(&mut self, dt: f32) {
        let mut particles = std::mem::take(&mut self.particles);
        particles.retain_mut(|p| self.age(p, dt));
        self.particles = particles;
    }

    /// Step one particle; `false` when it dies.
    fn age(&self, p: &mut Particle, dt: f32) -> bool {
        if p.life <= dt {
            return false;
        }
        p.life -= dt;
        let s = &self.spec;
        s.animation.advance(&mut p.animation_phase, &mut p.animation_cell, dt);
        if s.drag > 0.0 {
            p.velocity += p.velocity * (-dt * s.drag * p.velocity.length()).max(-1.0);
        }
        if s.damping > 0.0 {
            p.velocity *= (1.0 - s.damping.min(1.0)).powf(dt * DAMPING_STEPS_PER_SECOND);
        }
        match s.gravity != 0.0 {
            true => p.velocity.z -= s.gravity * dt,
            false => p.velocity += p.accel * dt,
        }
        p.position += p.velocity * dt;
        // The curve position counts down the nominal life, not the particle's own.
        let t = 1.0 - p.life / s.life;
        let offset = p.rotation_offset;
        self.look(p, t, offset);
        match s.kill_alpha {
            Some(floor) => p.color[3] > floor,
            None => true,
        }
    }
}
