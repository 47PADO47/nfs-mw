//! Cop health bars as pure rules: the bar above each chasing cop car, its health and lagging segment, its death flash
//! and fade, its distance fade, its tier colour and its place in the draw order. Nothing here reads a world, a clock
//! or a renderer. The caller passes one [`Sample`] per cop each step, the camera and the [`Gates`], and draws the
//! [`Bar`]s it gets back with [`segment_quads`]. Colours are linear RGBA.
//!
//! Spec: `docs/specs/cop-health-bars.md`, sections 3 to 11 and 14. Helicopters are not represented: the caller
//! samples only cop cars, so no bar is made for a helicopter.

mod bar;
mod colour;
mod death;
mod fade;
mod gates;
mod geometry;
mod lag;
mod params;
mod sample;
mod state;
mod tier;

pub use bar::{Bar, sort_back_to_front};
pub use colour::{BORDER_COLOUR, BORDER_DESTROYED_COLOUR, LAG_COLOUR, rgba, srgb_to_linear};
pub use death::{Death, death_alpha, death_phase, flash_on};
pub use fade::distance_alpha;
pub use gates::{Gates, shows};
pub use geometry::{Quad, bar_corners, segment_quads, span_corners};
pub use lag::LagTracker;
pub use params::BarParams;
pub use sample::{Sample, displayed_health, health_fraction};
pub use state::{CopBarState, CopBarStates};
pub use tier::{THRESHOLDS, Tier, tier_of, tier_rgb};

use glam::Vec3;

/// What one step of the bars needs from the frame: where the camera is, which way it looks, and the gates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub camera_pos: Vec3,
    /// The camera's unit view direction.
    pub camera_forward: Vec3,
    pub gates: Gates,
}

/// Steps the bar states by `dt` simulated seconds and returns the bars to draw this frame, back to front.
///
/// The states step whether or not the bars are shown, so turning the bars on mid-chase shows the current health.
/// `samples` must carry one entry per cop, with unique keys. Empty when a gate is shut.
pub fn build_bars(
    samples: &[Sample],
    frame: &Frame,
    states: &mut CopBarStates,
    dt: f32,
    params: &BarParams,
) -> Vec<Bar> {
    states.step(samples, dt, params);
    if !shows(frame.gates) {
        return Vec::new();
    }
    let right = billboard_right(frame.camera_forward);
    let mut bars: Vec<Bar> = samples
        .iter()
        .filter_map(|sample| {
            let state = states.get(sample.key)?;
            bar_for(sample, state, frame, right, params)
        })
        .collect();
    sort_back_to_front(&mut bars);
    bars
}

/// The camera's right vector, `forward x up`, as the spec takes it. A view straight up or down has no right, so the
/// fallback is the one a camera looking along +x has.
pub fn billboard_right(forward: Vec3) -> Vec3 {
    let right = forward.cross(Vec3::Z);
    match right.length_squared() > 1e-8 {
        true => right.normalize(),
        false => Vec3::NEG_Y,
    }
}

/// The bar of one cop, or `None` when it is gone (a destroyed cop past its hide time).
fn bar_for(sample: &Sample, state: &CopBarState, frame: &Frame, right: Vec3, params: &BarParams) -> Option<Bar> {
    let (death_alpha, flash_on, border) = death_look(state.dead_for(), params)?;
    let centre = sample.position + Vec3::Z * params.lift;
    let offset = centre - frame.camera_pos;
    let health = displayed_health(sample);
    let tier = tier_of(health);
    let alpha = params.alpha * distance_alpha(offset.length(), params) * death_alpha;
    Some(Bar {
        key: sample.key,
        centre,
        right,
        depth: offset.dot(frame.camera_forward),
        health,
        lag: state.lag(),
        tier,
        alpha,
        flash_on,
        destroyed: sample.destroyed,
        border_rgba: rgba(border, alpha),
        fill_rgba: rgba(tier_rgb(tier), alpha),
        lag_rgba: rgba(LAG_COLOUR, alpha),
    })
}

/// The death look of a cop: its alpha, whether the flash is on, and its border colour. `None` when the bar is gone.
fn death_look(dead_for: Option<f32>, params: &BarParams) -> Option<(f32, bool, [u8; 3])> {
    match dead_for {
        None => Some((1.0, false, BORDER_COLOUR)),
        Some(elapsed) => match death_phase(elapsed, params) {
            Death::Gone => None,
            Death::Flash { on } => Some((1.0, on, flash_border(on))),
            Death::Fade { alpha } => Some((alpha, false, BORDER_DESTROYED_COLOUR)),
        },
    }
}

/// The border during the flash: the destroyed colour in the "on" half, the normal border otherwise.
fn flash_border(on: bool) -> [u8; 3] {
    match on {
        true => BORDER_DESTROYED_COLOUR,
        false => BORDER_COLOUR,
    }
}

/// Approximate float equality for the tests of this module and its children.
#[cfg(test)]
pub(crate) fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(key: u32, x: f32, damage: f32, destroyed: bool) -> Sample {
        Sample { key, position: Vec3::new(x, 0.0, 0.0), damage, destroyed }
    }

    fn open() -> Gates {
        Gates { pursuing: true, shot_running: false, hud_visible: true }
    }

    fn frame(gates: Gates) -> Frame {
        Frame { camera_pos: Vec3::ZERO, camera_forward: Vec3::X, gates }
    }

    #[test]
    fn no_bar_without_a_pursuit_flag_but_the_state_still_steps() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        let gates = Gates { pursuing: false, ..open() };
        let bars = build_bars(&[sample(1, 10.0, 0.4, false)], &frame(gates), &mut states, 0.1, &params);
        assert!(bars.is_empty());
        assert_eq!(states.len(), 1);
    }

    #[test]
    fn a_running_shot_or_a_hidden_hud_hides_every_bar() {
        let params = BarParams::default();
        let samples = [sample(1, 10.0, 0.4, false)];
        let shot = Gates { shot_running: true, ..open() };
        let hidden = Gates { hud_visible: false, ..open() };
        let mut states = CopBarStates::default();
        assert!(build_bars(&samples, &frame(shot), &mut states, 0.1, &params).is_empty());
        assert!(build_bars(&samples, &frame(hidden), &mut states, 0.1, &params).is_empty());
        assert_eq!(states.len(), 1);
    }

    #[test]
    fn a_healthy_cop_gets_the_base_alpha_and_the_lift() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        let bars = build_bars(&[sample(1, 10.0, 0.0, false)], &frame(open()), &mut states, 0.0, &params);
        assert_eq!(bars.len(), 1);
        assert!(approx(bars[0].alpha, 0.7));
        assert!(approx(bars[0].centre.z, 1.15));
        assert!(approx(bars[0].depth, 10.0));
        assert_eq!(bars[0].tier, Tier::Green);
    }

    #[test]
    fn destroyed_bar_flashes_fades_and_then_is_gone() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        let samples = [sample(1, 10.0, 0.4, true)];
        // Steps of 0.5 s: the death at 0, then 0.5, 1.0 (fade starts), 1.5 (half faded), 2.0 (gone).
        let mut counts = Vec::new();
        let mut fade_alpha = None;
        for _ in 0..5 {
            let bars = build_bars(&samples, &frame(open()), &mut states, 0.5, &params);
            counts.push(bars.len());
            if let Some(bar) = bars.first() {
                fade_alpha = Some(bar.alpha);
            }
        }
        assert_eq!(counts, vec![1, 1, 1, 1, 0]);
        assert!(fade_alpha.is_some_and(|alpha| approx(alpha, 0.7 * 0.5)));
    }

    #[test]
    fn destroyed_bar_at_death_shows_the_flash_on_and_zero_health() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        let bars = build_bars(&[sample(1, 10.0, 0.4, true)], &frame(open()), &mut states, 0.1, &params);
        assert_eq!(bars.len(), 1);
        assert!(bars[0].flash_on);
        assert!(bars[0].destroyed);
        assert!(approx(bars[0].health, 0.0));
        assert_eq!(bars[0].tier, Tier::Red);
        assert_eq!(bars[0].border_rgba, rgba(BORDER_DESTROYED_COLOUR, 0.7));
    }

    #[test]
    fn distant_cop_fades_out_by_the_distance_rule() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        let bars = build_bars(&[sample(1, 90.0, 0.0, false)], &frame(open()), &mut states, 0.0, &params);
        // The bar sits 1.15 m up, so its distance is a little over 90 m; the expected value uses the same distance.
        let distance = Vec3::new(90.0, 0.0, 1.15).length();
        let expected = 0.7 * distance_alpha(distance, &params);
        assert!(bars.first().is_some_and(|bar| approx(bar.alpha, expected)));
    }

    #[test]
    fn bars_come_back_farther_first() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        let samples = [sample(1, 10.0, 0.0, false), sample(2, 30.0, 0.0, false)];
        let bars = build_bars(&samples, &frame(open()), &mut states, 0.0, &params);
        let keys: Vec<u32> = bars.iter().map(|bar| bar.key).collect();
        assert_eq!(keys, vec![2, 1]);
    }

    #[test]
    fn billboard_right_of_a_camera_looking_along_x_is_minus_y() {
        let right = billboard_right(Vec3::X);
        assert!(approx(right.x, 0.0) && approx(right.y, -1.0) && approx(right.z, 0.0));
    }

    #[test]
    fn billboard_right_of_a_vertical_view_has_a_fallback() {
        let right = billboard_right(Vec3::Z);
        assert!(approx(right.length(), 1.0));
    }
}
