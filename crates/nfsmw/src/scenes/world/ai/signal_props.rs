//! The real traffic-light props of the map as the stop lights of the junctions: the game-specific half of
//! [`blackbox_roads::match_props`]. The scenery holds lamp posts and mast arms (`XO_TrafficLight*`,
//! `SGN_Trafficlights_Pole`). This module finds them by name, works out where their lamp heads stand and which way
//! they face, and hands the poses to the engine-generic matcher.
//!
//! Frames: placed scenery is in render space (x forward, y left, z up; see `space.rs`). The approaches and the
//! matcher are in physics space, so poses are converted with `space::to_physics`.
//!
//! Assumptions that are not yet checked against the model data (calibration is listed in the spec, section
//! "Signals on real props"): the lamp head offsets in [`POLE`], [`ARM`] and [`CABLE`], and the sign of
//! [`LAMP_FACING`].

use blackbox_roads::{Approach, MatchParams, PropPose, match_props};
use glam::{Mat4, Vec3};

use super::TrafficWorld;
use crate::scenes::world::resident::Placed;
use crate::scenes::world::space;

/// The model axis a lamp head faces, in model axes. Scenery models are x forward, y left, z up (the frame the
/// `blackbox-roads` bounds conversion uses: "forward = length"). A lamp head faces along the road, so this is the
/// road axis.
///
/// UNVERIFIED: the sign. `+X` assumes the heads of these models face the way their +X axis points. If no light
/// matches any approach, flip this to `Vec3::NEG_X` first.
pub const LAMP_FACING: Vec3 = Vec3::X;

/// A traffic-light model: whether it is a mast arm (a bar that spans the road, so it may serve several approaches
/// of one direction), and where its lamp head stands in model axes (metres): `lamp_left` along the y axis and
/// `lamp_up` above the origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalModel {
    pub mast_arm: bool,
    pub lamp_left: f32,
    pub lamp_up: f32,
}

/// A lamp post with its head on top.
const POLE: SignalModel = SignalModel { mast_arm: false, lamp_left: 0.0, lamp_up: 4.5 };
/// `XO_TrafficLightB`: a pole with an arm. The data (`nfsmw-data` shapes, `pole_of_arm`) has the arm from the pole at
/// the origin to about 3.5 m along the model's negative y axis, and the whole model 8.7 m tall. The head hangs
/// under the middle of the arm.
const ARM: SignalModel = SignalModel { mast_arm: true, lamp_left: -1.7, lamp_up: 7.5 };
/// `XO_TrafficLightCableB`: a cable strung across the road, the heads hang from it.
const CABLE: SignalModel = SignalModel { mast_arm: true, lamp_left: 0.0, lamp_up: 6.0 };

/// The signal model of a scenery object, by its name (ASCII case does not matter), or `None` for anything that is
/// not a stop light. Arrow and cross signs (`XS_TrafficSignal*`, `SGN_Signal_*`) are lane-control signs over the
/// lanes, not stop lights, so they are left as scenery.
pub fn signal_model(name: &str) -> Option<SignalModel> {
    let name = name.to_ascii_lowercase();
    if name.starts_with("xo_trafficlightcable") {
        return Some(CABLE);
    }
    if let Some(variant) = name.strip_prefix("xo_trafficlight") {
        return match variant.chars().next()? {
            'b' => Some(ARM),
            'a' | 'c' | 'd' => Some(POLE),
            _ => None,
        };
    }
    if name.starts_with("sgn_trafficlights_pole") {
        return Some(POLE);
    }
    None
}

/// The way a placed model faces, in render space: the model's [`LAMP_FACING`] axis turned by the rotation of its
/// placement. Translation and the length of the vector do not matter, so the result is a unit vector (zero when the
/// placement collapses the axis).
pub fn facing(transform: Mat4) -> Vec3 {
    transform.transform_vector3(LAMP_FACING).normalize_or_zero()
}

/// A traffic-light prop placed in the world: where its lamp head is and the way it faces, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalHead {
    pub position: Vec3,
    pub forward: Vec3,
    pub mast_arm: bool,
}

/// The signal head of a scenery object from its name and its placement (`transform`, model to render space), or
/// `None` when it is not a traffic light.
pub fn signal_head(name: &str, transform: Mat4) -> Option<SignalHead> {
    let model = signal_model(name)?;
    Some(SignalHead {
        position: transform.transform_point3(Vec3::new(0.0, model.lamp_left, model.lamp_up)),
        forward: facing(transform),
        mast_arm: model.mast_arm,
    })
}

/// The matcher's pose of a head: physics space, kind 0 (the caller keeps the head's index).
pub fn pose(head: &SignalHead) -> PropPose {
    PropPose {
        position: Vec3::from(space::to_physics(head.position)),
        forward: Vec3::from(space::to_physics(head.forward)),
        kind: 0,
        mast_arm: head.mast_arm,
    }
}

/// The head each approach is served by (same order as `approaches`), as an index into `heads`, or `None` for an
/// approach with no matching light. Pure: the same input always gives the same matches.
pub fn match_heads(approaches: &[Approach], heads: &[SignalHead]) -> Vec<Option<usize>> {
    let poses: Vec<PropPose> = heads.iter().map(pose).collect();
    match_props(approaches, &poses, &MatchParams::default())
}

/// A traffic light that serves an approach: its lamp head and the way it faces (render space), and the prop it stands
/// on (its id in the prop world), `None` when the object has no collision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    pub position: Vec3,
    pub forward: Vec3,
    pub prop: Option<u32>,
}

/// The matches of the signalled approaches to the placed traffic lights. Rebuilt when the scenery changes.
#[derive(Debug, Default)]
pub struct SignalLamps {
    /// The residency's `visual_revision` the matches were built for; `None` before the first build.
    revision: Option<u64>,
    /// Per approach, in the controller's order: the light that serves it, if any.
    lamps: Vec<Option<Lamp>>,
    /// Traffic-light props that were placed at the last build.
    candidates: usize,
}

impl SignalLamps {
    pub fn lamp(&self, approach: usize) -> Option<Lamp> {
        self.lamps.get(approach).copied().flatten()
    }

    fn matched(&self) -> usize {
        self.lamps.iter().filter(|lamp| lamp.is_some()).count()
    }
}

/// The signal head of a placed object and its prop id, when it is a traffic light. The prop id is only given when
/// the object has collision: only those can be knocked over.
pub fn candidate(placed: &Placed) -> Option<(SignalHead, Option<u32>)> {
    let head = placed.signal?;
    Some((head, placed.prop.as_ref().map(|_| placed.prop_id)))
}

impl TrafficWorld {
    /// An extended junction signal replaces the authored blink on this matched head only.
    pub fn controls_signal_head(&self, head: SignalHead) -> bool {
        self.signal_lamps
            .lamps
            .iter()
            .flatten()
            .any(|lamp| lamp.position == head.position && lamp.forward == head.forward)
    }

    /// Matches the placed traffic lights to the signalled approaches, when the scenery changed since the last match
    /// (`revision` is the residency's `visual_revision`). The candidates are read only when a match is due.
    pub fn update_signal_props(&mut self, revision: u64, candidates: impl Iterator<Item = (SignalHead, Option<u32>)>) {
        if self.signal_lamps.revision == Some(revision) {
            return;
        }
        let found: Vec<(SignalHead, Option<u32>)> = candidates.collect();
        let heads: Vec<SignalHead> = found.iter().map(|(head, _)| *head).collect();
        let served = match_heads(self.signals.approaches(), &heads);
        self.signal_lamps.lamps = served
            .into_iter()
            .map(|index| {
                index.map(|i| {
                    let (head, prop) = found[i];
                    Lamp { position: head.position, forward: head.forward, prop }
                })
            })
            .collect();
        self.signal_lamps.candidates = found.len();
        self.signal_lamps.revision = Some(revision);
        log::debug!(
            "signals: {} of {} approaches matched to {} traffic-light props",
            self.signal_lamps.matched(),
            self.signals.approaches().len(),
            found.len()
        );
    }

    /// One line for the `signals` command.
    pub fn signal_props_status(&self) -> String {
        let approaches = self.signals.approaches().len();
        let matched = self.signal_lamps.matched();
        format!(
            "{approaches} signalled approaches: {matched} matched to a real light, {} with no light; \
             {} traffic-light props placed",
            approaches - matched,
            self.signal_lamps.candidates
        )
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use blackbox_roads::right_of;
    use glam::Quat;

    use super::*;

    #[test]
    fn only_the_stop_light_models_are_matched_by_name() {
        assert_eq!(signal_model("XO_TrafficLightA"), Some(POLE));
        assert_eq!(signal_model("xo_trafficlightc_1a_LL"), Some(POLE));
        assert_eq!(signal_model("XO_TrafficLightD"), Some(POLE));
        assert_eq!(signal_model("XO_TrafficLightB_03"), Some(ARM));
        assert_eq!(signal_model("XO_TrafficLightCableB"), Some(CABLE));
        assert_eq!(signal_model("SGN_Trafficlights_Pole"), Some(POLE));
        assert_eq!(signal_model("XO_TrafficLightE"), None, "no such model");
        assert_eq!(signal_model("XS_TrafficSignalArrow_LL"), None, "lane sign, not a stop light");
        assert_eq!(signal_model("SGN_Signal_Cross"), None, "lane sign, not a stop light");
        assert_eq!(signal_model("XO_TrafficConeA"), None);
        assert_eq!(signal_model("XS_StopSignB"), None);
    }

    #[test]
    fn the_facing_turns_with_the_rotation_and_ignores_the_translation() {
        assert_eq!(facing(Mat4::IDENTITY), Vec3::X);
        let turned =
            Mat4::from_translation(Vec3::new(5.0, 6.0, 7.0)) * Mat4::from_quat(Quat::from_rotation_z(FRAC_PI_2));
        assert!(facing(turned).distance(Vec3::Y) < 1e-5, "{:?}", facing(turned));
        let scaled = Mat4::from_scale(Vec3::splat(3.0));
        assert!(facing(scaled).distance(Vec3::X) < 1e-5, "the length of the axis does not matter");
        assert_eq!(facing(Mat4::from_scale(Vec3::new(0.0, 1.0, 1.0))), Vec3::ZERO, "a flattened axis faces nothing");
    }

    #[test]
    fn the_lamp_head_is_at_its_offset_in_the_placed_model() {
        // The arm's head: 1.7 m to the model's right (negative y), 7.5 m up. Turned a quarter about up, the model's
        // left (+y) points to -x, so the -1.7 m offset lands at +x.
        let placed =
            Mat4::from_translation(Vec3::new(10.0, 20.0, 0.0)) * Mat4::from_quat(Quat::from_rotation_z(FRAC_PI_2));
        let head = signal_head("XO_TrafficLightB", placed).expect("a traffic light");
        assert!(head.position.distance(Vec3::new(11.7, 20.0, 7.5)) < 1e-4, "{:?}", head.position);
        assert!(head.mast_arm);
        let pole = signal_head("SGN_Trafficlights_Pole", Mat4::IDENTITY).expect("a traffic light");
        assert_eq!(pole.position, Vec3::new(0.0, 0.0, POLE.lamp_up));
        assert!(!pole.mast_arm);
        assert!(signal_head("XS_StopSignB", Mat4::IDENTITY).is_none());
    }

    /// An approach that drives along physics +z through the origin, with its stop point at the origin.
    fn approach() -> Approach {
        Approach {
            junction: 0,
            segment: 0,
            node: 0,
            node_ind: 0,
            stop_position: Vec3::ZERO,
            heading: Vec3::Z,
            phase: 0,
            kerb: 4.0,
        }
    }

    /// A head standing `side` metres to the right (negative for left) of the lane and 6 m up, facing `facing`
    /// (physics space), given in render space as the scenery gives it.
    fn head_at(side: f32, facing: Vec3) -> SignalHead {
        let physics = right_of(Vec3::Z) * side + Vec3::Y * 6.0;
        SignalHead {
            position: space::to_render(physics.to_array()),
            forward: space::to_render(facing.to_array()),
            mast_arm: false,
        }
    }

    #[test]
    fn a_pole_on_the_right_facing_back_along_the_approach_serves_it() {
        let heads = [head_at(5.0, -Vec3::Z)];
        assert_eq!(match_heads(&[approach()], &heads), vec![Some(0)]);
    }

    #[test]
    fn a_head_facing_with_the_traffic_or_on_the_far_side_does_not_serve_it() {
        let facing_away = [head_at(5.0, Vec3::Z)];
        assert_eq!(match_heads(&[approach()], &facing_away), vec![None]);
        let left_side = [head_at(-5.0, -Vec3::Z)];
        assert_eq!(match_heads(&[approach()], &left_side), vec![None]);
    }
}
