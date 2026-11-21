//! The controls hook of the mod: the aids (aids.md) and the steering stage (steering.md) behind one callback. Pure: one call of
//! [`controls_step`] is one tick of the hook, from the host's query to the host's answer.

use blackbox_modding::sdk::{ControlsQuery, ControlsResult, Role};

use crate::aids::{AidCar, AidIn, AidOut, AidState, Levels, aids_step, turns_to_rad};
use crate::keys::SteeringKeys;
use crate::steering::{Device, SteerState, ffb_step, steer_step};

/// The longest tick the hook accepts: the host's 0.1 s with the `f32` slack.
const MAX_DT: f64 = 0.1 * (1.0 + 1.0e-6);

/// What the glue sets for the call: which parts are on and how they are tuned.
#[derive(Debug, Clone)]
pub struct ControlsCfg {
    /// The aids part is on.
    pub aids: bool,
    /// The steering part is on (the answer then carries the steering target).
    pub steering: bool,
    /// The aids' levels.
    pub levels: Levels,
    /// The vehicle's constants for the aids, when the car is known.
    pub car: Option<AidCar>,
    /// The steering model's keys.
    pub steer_keys: SteeringKeys,
    /// The steering device.
    pub device: Device,
}

/// What the mod knows beyond the controls query (its own tyre results and the suspension's steering angles).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Feed {
    /// Aligning torque of the front tyres, last tick, N m.
    pub mz: [f64; 2],
    /// Spin of the front tyres, last tick, rad/s.
    pub omega: [f64; 2],
    /// The angle the steering model starts from (the last suspension call's inside front wheel, else 0), rad.
    pub seed_delta: f64,
}

/// The state kept between calls.
#[derive(Debug, Clone, Default)]
pub struct ControlsState {
    /// The aids' state.
    pub aids: AidState,
    /// The steering state; `None` until the first call with the steering part on seeds it.
    pub steer: Option<SteerState>,
    /// The last aids' answer: for the HUD and the status.
    pub last_aid: Option<AidOut>,
    /// The last force-feedback value, -1 to 1 (positive pulls the wheel to the right); 0 with the steering part off.
    pub ffb: f64,
    /// The column torque behind it, N m.
    pub col_torque: f64,
}

impl ControlsState {
    /// Forget everything: the next call starts the aids and the steering again.
    pub fn reset(&mut self) {
        *self = ControlsState::default();
    }
}

fn finite(query: &ControlsQuery) -> bool {
    let scalars = [
        query.dt,
        query.throttle,
        query.brake,
        query.handbrake,
        query.steer,
        query.forward_speed,
        query.yaw_rate,
        query.lateral_accel,
        query.longitudinal_accel,
    ];
    scalars.iter().all(|value| value.is_finite())
        && query.slip_ratio.iter().all(|value| value.is_finite())
        && query.slip_angle.iter().all(|value| value.is_finite())
}

/// The mean rear slip angle in radians over the rear wheels that were loaded; 0 when neither was.
fn alpha_rear(query: &ControlsQuery) -> f64 {
    let (mut sum, mut count) = (0.0, 0.0);
    for i in 2..4 {
        if query.wheel_loaded[i] {
            sum += turns_to_rad(f64::from(query.slip_angle[i]));
            count += 1.0;
        }
    }
    if count == 0.0 { 0.0 } else { sum / count }
}

/// One call of the controls hook. `None` is "not handled": the controls stay as the host shaped them.
pub fn controls_step(
    cfg: &ControlsCfg,
    st: &mut ControlsState,
    query: &ControlsQuery,
    feed: &Feed,
) -> Option<ControlsResult> {
    if query.role != Some(Role::Player) || !finite(query) {
        return None;
    }
    let dt = f64::from(query.dt);
    if dt <= 0.0 || dt > MAX_DT || (!cfg.aids && !cfg.steering) {
        return None;
    }
    let mut result = ControlsResult {
        throttle: query.throttle.clamp(0.0, 1.0),
        brake: [query.brake.clamp(0.0, 1.0); 4],
        steer_target_rad: None,
    };
    if cfg.aids {
        let car = cfg.car.as_ref()?;
        let input = AidIn {
            dt,
            throttle: f64::from(query.throttle),
            brake: f64::from(query.brake),
            v: f64::from(query.forward_speed),
            kappa: query.slip_ratio.map(f64::from),
            alpha: query.slip_angle.map(|turns| turns_to_rad(f64::from(turns))),
            loaded: query.wheel_loaded,
        };
        let out = aids_step(car, cfg.levels, &mut st.aids, &input);
        result.throttle = out.throttle as f32;
        result.brake = out.brake.map(|value| value as f32);
        st.last_aid = Some(out);
    }
    if cfg.steering {
        let steer = st.steer.get_or_insert_with(|| SteerState::seeded(feed.seed_delta));
        let delta = steer_step(
            &cfg.steer_keys,
            cfg.device,
            steer,
            f64::from(query.steer),
            f64::from(query.forward_speed),
            alpha_rear(query),
            dt,
        );
        let (ffb, col) = ffb_step(&cfg.steer_keys, steer, feed.mz, feed.omega, f64::from(query.forward_speed), dt);
        st.ffb = ffb;
        st.col_torque = col;
        result.steer_target_rad = Some(delta as f32);
    }
    Some(result)
}
