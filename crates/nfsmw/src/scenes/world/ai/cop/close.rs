//! Close pursuit action, reconstructed from the behavior in ai-pursuit-tactics §3.
//! Road queries and wall queries stay in the caller; these rules consume their results.

use blackbox_pursuit::goals::approach::{combine, limiter_step};
use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::{CopCar, CopThink, THINK_PERIOD};

pub(super) fn xz(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}
pub(super) fn lift(v: Vec2, y: f32) -> Vec3 {
    Vec3::new(v.x, y, v.y)
}

pub(super) struct RoadSteer {
    pub point: Option<Vec3>,
    pub current: Option<blackbox_roads::Cookie>,
}

pub(super) fn eligible(me: &Body, target: &Body, road_gap: f32, clear_target: bool, clear_nav: bool) -> bool {
    me.position.distance(target.position) <= 60.0 + road_gap
        && (target.velocity - me.velocity).length() <= 140.0 / 3.6
        && clear_target
        && clear_nav
}

#[derive(Debug, Clone)]
pub(super) struct Close {
    history: super::history::History,
    active: bool,
    limit: f32,
    think_delta: f32,
    nos_wait: f32,
    target_nos: bool,
    pub nos: bool,
    #[cfg(test)]
    diagnostic: String,
}

impl Default for Close {
    fn default() -> Self {
        Self {
            history: super::history::History::default(),
            active: false,
            limit: 0.0,
            think_delta: THINK_PERIOD,
            nos_wait: 0.0,
            target_nos: false,
            nos: false,
            #[cfg(test)]
            diagnostic: String::new(),
        }
    }
}

impl Close {
    pub(super) fn set_think_delta(&mut self, dt: f32) {
        self.think_delta = dt;
    }
    pub(super) fn hash_into(&self, hash: &mut crate::scenes::world::StateHasher) {
        self.history.hash_into(hash);
        hash.bool(self.active);
        hash.f32(self.limit);
        hash.f32(self.think_delta);
        hash.f32(self.nos_wait);
        hash.bool(self.target_nos);
        hash.bool(self.nos);
    }
    pub fn leave(&mut self) {
        self.active = false;
        self.nos = false;
    }
    pub fn active(&self) -> bool {
        self.active
    }

    #[cfg(test)]
    pub(super) fn diagnostic(&self) -> &str {
        &self.diagnostic
    }

    fn sample(&mut self, speed: f32, delay: f32) -> f32 {
        self.history.add(speed, self.think_delta);
        self.history.sample(delay)
    }

    pub fn think(&mut self, input: Input<'_>, mut snap: impl FnMut(Vec3) -> RoadSteer) -> CopThink {
        let Input {
            car,
            me,
            target,
            aim,
            road_direction,
            separation,
            avoidance,
            performance,
            forward_speed,
            grade,
            reaction,
            target_nos,
            target_nos_input,
            jerk,
        } = input;
        let speed = xz(target.velocity).length();
        if !self.active {
            self.history.reset(speed);
            self.limit = forward_speed;
            self.active = true;
        }
        let delayed = self.sample(speed, reaction);
        let direction = road_direction.unwrap_or_else(|| xz(target.velocity).try_normalize().unwrap_or(target.forward));
        let mut offset = xz(aim - me.position);
        if offset.length() > 0.0001 {
            let ahead = offset.dot(direction) / offset.length();
            let slide = (ahead * 10.0).clamp(-1.0, 1.0) * 0.5 + 0.5;
            let rscale = ((20.0_f32 / 3.6).max(0.2 * delayed) / offset.length()).min(1.0);
            let scale = slide * 1.8 + (1.0 - slide) * rscale;
            let along = direction * offset.dot(direction);
            offset += along * (scale - 1.0);
        }
        // Stock cop TETHER_WEIGHT is zero, so prediction follows the road when available.
        let mut seek = offset + direction * delayed;
        #[cfg(test)]
        let seek_before_snap = seek;
        let road = snap(me.position + lift(seek, 0.0));
        if let Some(point) = road.point {
            let along_road = xz(point - me.position).normalize_or_zero();
            seek = along_road * seek.length();
        }
        #[cfg(test)]
        let seek_after_snap = seek;
        let affinity = road.current.as_ref().map_or(Vec2::ZERO, |cookie| affinity(me.position, me.velocity, cookie));
        let (ceiling_scale, speed_scale, acceleration_scale) = match jerk {
            true => (1.1, 1.2, 1.5),
            false => (1.0, 1.0, 1.0),
        };
        let ceiling = (car.ai.max_speed_kmh / 3.6 * ceiling_scale)
            .min(performance.top_speed * car.ai.top_speed_multiplier * speed_scale);
        let mut acceleration =
            performance.acceleration_at(forward_speed.abs()) * car.ai.acceleration_multiplier * acceleration_scale
                - 9.81 * grade;
        if target_nos {
            acceleration *= 0.2;
        }
        #[cfg(test)]
        let previous_limit = self.limit;
        self.limit = limiter_step(self.limit, forward_speed, ceiling, acceleration, self.think_delta);
        seek = seek.clamp_length_max(self.limit.max(0.0));
        let mut steer = combine(seek, separation + avoidance + affinity, 35.0, 25.0);
        #[cfg(test)]
        let steer_before_corner = steer;
        #[cfg(test)]
        let mut corner_diagnostic = None;
        let own_speed = xz(me.velocity).length();
        if own_speed > 0.1 && steer.length() > 0.1 {
            let cosine = xz(me.velocity).normalize_or_zero().dot(steer.normalize_or_zero());
            let curvature = (1.0 - cosine) * own_speed / steer.length();
            let corner_limit = (40.0_f32 / 3.6).max(450.0 / 3.6 - curvature * 950.0 / 3.6);
            #[cfg(test)]
            {
                corner_diagnostic = Some((cosine, curvature, corner_limit));
            }
            steer = steer.clamp_length_max(corner_limit);
        }
        let think = CopThink { target: me.position + lift(steer, 0.0), speed: steer.length() };
        self.update_nos(me, &think, target_nos_input);
        #[cfg(test)]
        {
            self.diagnostic = format!(
                "action=close position={:?} target_position={:?} target_velocity={:?} aim={aim:?} direction={direction:?} target_speed={speed:.3} delayed_speed={delayed:.3} reaction={reaction:.3} offset={offset:?} seek_before_snap={seek_before_snap:?} snap_point={:?} seek_after_snap={seek_after_snap:?} current_cookie={:?} affinity={affinity:?} separation={separation:?} wall={avoidance:?} previous_limit={previous_limit:.3} ceiling={ceiling:.3} accel={acceleration:.3} grade={grade:.3} limiter={:.3} limited_seek={seek:?} combined_steer={steer_before_corner:?} corner_cosine_curvature_limit={corner_diagnostic:?} final_steer={steer:?} final_target={:?} final_speed={:.3} forward_speed={forward_speed:.3} planar_speed={own_speed:.3} target_nos={target_nos} target_nos_input={target_nos_input} nos={}",
                me.position,
                target.position,
                target.velocity,
                road.point,
                road.current,
                self.limit,
                think.target,
                think.speed,
                self.nos,
            );
        }
        think
    }

    fn update_nos(&mut self, me: &Body, think: &CopThink, target_nos: bool) {
        let rising = target_nos && !self.target_nos;
        self.target_nos = target_nos;
        if self.nos_wait > 0.0 {
            self.nos_wait = (self.nos_wait - self.think_delta).max(0.0);
            self.nos = false;
            return;
        }
        if rising && !self.nos {
            self.nos_wait = 1.5;
            return;
        }
        let (wanted, own, deficit, alignment) = match self.nos {
            true => (90.0, 50.0, 10.0, 0.8),
            false => (120.0, 80.0, 60.0, 0.95),
        };
        let velocity = xz(me.velocity);
        let direction = xz(think.target - me.position);
        self.nos = think.speed > wanted / 3.6
            && velocity.length() > own / 3.6
            && think.speed - velocity.length() > deficit / 3.6
            && direction.dot(velocity) > alignment * direction.length() * velocity.length();
    }
}

pub(super) struct Input<'a> {
    pub car: &'a CopCar,
    pub me: &'a Body,
    pub target: &'a Body,
    pub aim: Vec3,
    pub road_direction: Option<Vec2>,
    pub separation: Vec2,
    pub avoidance: Vec2,
    pub performance: &'a blackbox_vehicle::performance::Performance,
    pub forward_speed: f32,
    pub grade: f32,
    pub reaction: f32,
    pub target_nos: bool,
    pub target_nos_input: bool,
    pub jerk: bool,
}

/// Normal-directed wall avoidance, scaled by time until impact.
pub(super) fn wall(velocity: Vec3, normal: Vec3, distance: f32) -> Vec2 {
    let speed = velocity.length();
    if speed < 2.0 {
        return Vec2::ZERO;
    }
    let c = velocity.dot(normal) / speed;
    if c >= 0.0 {
        return Vec2::ZERO;
    }
    xz(normal) * (c * c * (10.0 / 3.6) / (distance / speed).max(0.01).powi(2))
}

pub(super) fn affinity(position: Vec3, velocity: Vec3, cookie: &blackbox_roads::Cookie) -> Vec2 {
    let side = cookie.right_axis();
    let left = ((xz(position) - cookie.left).dot(side) + 1.0).max(0.1);
    let right = ((cookie.right - xz(position)).dot(side) + 1.0).max(0.1);
    let side_speed = xz(velocity).dot(side);
    let a = (0.5 / left.powi(2) + 2.0 * ((-side_speed).max(0.0) / left).powi(2)).min(100.0 / 3.6);
    let b = (0.5 / right.powi(2) + 2.0 * (side_speed.max(0.0) / right).powi(2)).min(100.0 / 3.6);
    side * (a - b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wall_avoidance_grows_as_impact_approaches_and_ignores_receding_motion() {
        let far = wall(Vec3::Z * 20.0, -Vec3::Z, 20.0);
        let near = wall(Vec3::Z * 20.0, -Vec3::Z, 10.0);
        assert!((near.length() - 4.0 * far.length()).abs() < 0.0001);
        assert_eq!(wall(Vec3::Z * 20.0, Vec3::Z, 10.0), Vec2::ZERO);
    }

    fn body(position: Vec3, speed: f32) -> Body {
        Body { position, velocity: Vec3::Z * speed, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
    }

    #[test]
    fn close_chase_requires_both_rays_and_the_relative_speed_gate() {
        let me = body(Vec3::ZERO, 20.0);
        let mut target = body(Vec3::Z * 59.0, 20.0);
        assert!(eligible(&me, &target, 0.0, true, true));
        assert!(!eligible(&me, &target, 0.0, false, true));
        assert!(!eligible(&me, &target, 0.0, true, false));
        target.position.z = 61.0;
        assert!(!eligible(&me, &target, 0.0, true, true));
        assert!(eligible(&me, &target, 2.0, true, true));
        target.velocity.z = 60.0;
        assert!(!eligible(&me, &target, 2.0, true, true));
    }

    #[test]
    fn player_nitrous_rising_edge_delays_cop_nitrous_and_goals_can_clear_it() {
        let me = body(Vec3::ZERO, 30.0);
        let think = CopThink { target: Vec3::Z * 100.0, speed: 60.0 };
        let mut state = Close::default();
        state.update_nos(&me, &think, false);
        assert!(state.nos);
        state.nos = false;
        state.update_nos(&me, &think, true);
        assert!(!state.nos);
        for _ in 0..(1.5 / THINK_PERIOD).ceil() as usize {
            state.update_nos(&me, &think, true);
            assert!(!state.nos);
        }
        state.update_nos(&me, &think, true);
        assert!(state.nos);
        state.leave();
        assert!(!state.nos);
    }
}
