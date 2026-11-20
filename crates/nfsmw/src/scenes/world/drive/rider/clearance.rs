//! Keeping the rider's body out of his bike's body when it leans (`docs/specs/ride6-rider-clearance.md`).
//!
//! The animation is authored on one bike and its lean clips sweep the whole body over by the lean; on another bike (a taller tank, a wider
//! fairing, a scooter's body) the outer knee, the arms and the chest end up inside the bike's solid. The converter measures **the bike's
//! cross-section**: for each cell of the side view (x along the bike, z up) the half width of the body there, the largest `|y|` of the
//! chassis, the tank and the exhaust ([`Section`], `contacts.json` `body_section`). The rider's own physics asset gives his body as
//! capsules on his bones (`ragdoll.json`); this samples three points along each capsule of an arm, a leg and the torso and asks whether the
//! capsule's skin is inside the section. What is inside is pushed out sideways, by what a rider does: a knee or an elbow is swung out
//! about the line from hip (shoulder) to foot (hand), which is the two-bone solve's pole ([`Clearance::limb_pole`]), and a chest that sits
//! on one side of the tank is moved over a little ([`Clearance::torso`]). Nothing here moves a hand off its grip or a foot off its peg.
//!
//! Every number below is a choice of this method, not data of the game.

use blackbox_skeleton::{Graph, Pack, Skeleton, Transform};
use glam::Vec3;
use serde::Deserialize;

use super::attach::Attach;
use super::fall_file::RagdollFile;
use super::graph_params::param_values;
use super::params::Values;
use super::pose::{self, Frame, Resolved};

/// How far into the bike's body the rider's skin may be, metres, before it is pushed out: the suit's padding and the section's cell
/// (3 cm, read as the largest of four) give a centimetre either way. This is also the tolerance of the numeric check
/// (`clearance_probe_tests.rs`): no skin vertex of the rider deeper than this plus the probe's own slack.
pub const TOLERANCE_M: f32 = 0.01;
/// The most a knee or an elbow is pushed out in one frame, metres: a bound for a section that is wrong, not a modelling value.
pub const MAX_PUSH_M: f32 = 0.12;
/// The fraction of a limb segment a push at the knee (elbow) moves: a point nearer than this to the hip, the shoulder, the foot or the hand
/// is not helped by swinging the knee and is left to the body's shift.
pub const MIN_LEVER: f32 = 0.25;
/// The most the chest is moved over, metres, and the share of the section's half width from the centre line beyond which a chest point is
/// on one side of the tank (nearer the centre line it is above it, which a sideways push cannot cure).
pub const MAX_TORSO_PUSH_M: f32 = 0.05;
pub const TORSO_OUTER: f32 = 0.5;
/// The most the whole body is moved over for a limb that swinging its joint could not clear (a straight leg has no knee to swing), metres.
pub const MAX_BODY_PUSH_M: f32 = 0.06;
/// The sample points along a capsule's axis, as fractions of its half length from the centre.
const SAMPLES: [f32; 3] = [-1.0, 0.0, 1.0];
/// A lateral place nearer than this to the centre line (m) has no side of its own: the limb's side, or the left, is taken.
const SIDELESS_M: f32 = 1e-3;

/// The bike's cross-section: the half width of its body in each cell of the side view, in the bike skeleton's frame (x forward, z up,
/// metres), written by the converter in whole millimetres, 0 for no body there.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Section {
    /// The corner of the grid and the cell's size, metres.
    pub x0: f32,
    pub z0: f32,
    pub step: f32,
    pub nx: usize,
    pub nz: usize,
    /// Half widths in millimetres, `nx` columns of `nz` cells (z fastest).
    pub half_width_mm: Vec<u16>,
}

impl Section {
    /// Whether the grid is what it says it is (a damaged file is not used).
    pub fn is_valid(&self) -> bool {
        let finite = [self.x0, self.z0, self.step].iter().all(|v| v.is_finite());
        finite && self.step > 0.0 && self.nx > 0 && self.nz > 0 && self.half_width_mm.len() == self.nx * self.nz
    }

    fn cell(&self, i: i64, j: i64) -> f32 {
        let inside = (0..self.nx as i64).contains(&i) && (0..self.nz as i64).contains(&j);
        match inside {
            true => f32::from(self.half_width_mm[i as usize * self.nz + j as usize]) * 0.001,
            false => 0.0,
        }
    }

    /// The half width of the body at `(x, z)`: the largest of the four cells that the point is nearest to, so that a point near a wall is
    /// taken for being at it. 0 outside the grid.
    pub fn half_width(&self, x: f32, z: f32) -> f32 {
        let (fx, fz) = ((x - self.x0) / self.step - 0.5, (z - self.z0) / self.step - 0.5);
        if !(fx.is_finite() && fz.is_finite()) {
            return 0.0;
        }
        let (i, j) = (fx.floor() as i64, fz.floor() as i64);
        [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)].into_iter().map(|(a, b)| self.cell(a, b)).fold(0.0, f32::max)
    }
}

/// What a capsule belongs to: a segment of a limb (`upper` is the root bone's, the thigh or the upper arm; else the middle bone's) or
/// the torso.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Group {
    Limb { limb: usize, upper: bool },
    Torso,
}

/// A capsule of the rider's body on a bone: the sample points on its axis (the bone's frame) and its radius.
#[derive(Debug, Clone, PartialEq)]
struct Guard {
    bone: usize,
    points: [Vec3; 3],
    radius: f32,
    group: Group,
}

/// The rider's body against the bike's section.
#[derive(Debug, Clone, PartialEq)]
pub struct Clearance {
    section: Section,
    guards: Vec<Guard>,
    /// How deep each guard's sample points are in the upright, unleaned pose (`rest_on`), which the animation was authored to fit: only
    /// what a lean adds to it is pushed out. Zero until set.
    baseline: Vec<[f32; 3]>,
    /// Which side of the bike each limb is on (1 left, -1 right), in the order of `Resolved::limbs`, from the upright pose: a leg that a
    /// lean has taken across the centre line is still the one on its own side. 1 until `rest_on`.
    limb_sides: Vec<f32>,
}

/// How deep a capsule point at `p` (the bike's frame) is inside the section, metres: how far it would have to go sideways, outward, for the
/// capsule's skin to be within [`TOLERANCE_M`] of the body's wall; 0 when it is clear. Also the half width there.
fn depth(section: &Section, p: Vec3, radius: f32) -> (f32, f32) {
    let wall = section.half_width(p.x, p.z);
    if wall <= 0.0 {
        return (0.0, 0.0);
    }
    ((wall + radius - TOLERANCE_M - p.y.abs()).max(0.0), wall)
}

/// Whether the keeping-out is turned off (`NFSMW_RIDER_CLEARANCE=0`): the rider is then posed as before, to compare.
fn turned_off() -> bool {
    std::env::var("NFSMW_RIDER_CLEARANCE").is_ok_and(|value| value == "0")
}

impl Clearance {
    /// How much deeper than in the upright pose sample `k` of guard `g` is at `point` (the bike's frame): `(excess, half width)`.
    fn excess(&self, g: usize, k: usize, point: Vec3) -> (f32, f32) {
        let (deep, wall) = depth(&self.section, point, self.guards[g].radius);
        ((deep - self.baseline[g][k]).max(0.0), wall)
    }

    /// Takes the upright pose `component` (rider space) as the one the animation fits: the depths in it are no fault of a lean.
    fn set_baseline(&mut self, component: &[Transform], frame: &Frame, roots: &[usize]) {
        let sides = roots.iter().map(|&root| frame.to_bike.transform_point3(component[root].translation).y);
        self.limb_sides = sides.map(|y| if y.abs() < SIDELESS_M { 1.0 } else { y.signum() }).collect();
        let depths: Vec<[f32; 3]> = self
            .guards
            .iter()
            .map(|guard| {
                let mut out = [0.0; 3];
                for (k, (_, point)) in Self::points(guard, component, frame).enumerate() {
                    out[k] = depth(&self.section, point, guard.radius).0;
                }
                out
            })
            .collect();
        self.baseline = depths;
    }

    /// Poses the rider upright and unleaned on his bike and takes that as the baseline ([`Clearance::set_baseline`]).
    pub fn rest_on(&mut self, pack: &Pack, graph: &Graph, attach: &Attach, resolved: &Resolved, frame: &Frame) {
        let values = Values { straight: 1.0, ..Values::default() };
        let params = param_values(&graph.param_names, &values);
        let targets = pose::rider_targets(attach, frame, None);
        let goals = pose::Goals { targets: &targets, body: pose::Body::REST, frame, release: [0.0; 2] };
        let posed = pose::animate(&pack.skeleton, &pack.clips, graph, (&params, 0.0), resolved, &goals);
        if let Ok(component) = posed {
            let roots: Vec<usize> = resolved.limbs.iter().map(|limb| limb.chain.root).collect();
            self.set_baseline(&component, frame, &roots);
        }
    }

    /// The clearance of the rider whose bike's `attach` has a measured section and whose ragdoll is `ragdoll`; `None` without either,
    /// or with `NFSMW_RIDER_CLEARANCE=0`.
    pub fn of_rig(
        attach: &Attach,
        ragdoll: Option<&RagdollFile>,
        resolved: &Resolved,
        skeleton: &Skeleton,
    ) -> Option<Self> {
        if turned_off() {
            return None;
        }
        Self::new(attach.body_section.clone()?, ragdoll?, resolved, skeleton)
    }

    /// The rider's capsules that matter, found for the limbs and the torso `resolved` names: the thighs and calves, the upper arms and
    /// forearms (the bodies on each limb's first two bones) and the capsules on the torso's bones other than its first (which is the
    /// seat's own). `None` when the rider has no ragdoll or the section is damaged.
    pub fn new(section: Section, ragdoll: &RagdollFile, resolved: &Resolved, skeleton: &Skeleton) -> Option<Self> {
        if !section.is_valid() {
            return None;
        }
        let mut guards = Vec::new();
        for body in &ragdoll.bodies {
            let Some(bone) = skeleton.find(&body.bone) else { continue };
            let group = resolved.limbs.iter().enumerate().find_map(|(limb, l)| match bone {
                b if b == l.chain.root => Some(Group::Limb { limb, upper: true }),
                b if b == l.chain.mid => Some(Group::Limb { limb, upper: false }),
                _ => None,
            });
            let torso = resolved.torso.as_ref().is_some_and(|t| t.lean.iter().skip(1).any(|&b| b == bone));
            let Some(group) = group.or(torso.then_some(Group::Torso)) else { continue };
            let (centre, axis) = (Vec3::from_array(body.centre), Vec3::from_array(body.axis).normalize_or_zero());
            guards.push(Guard {
                bone,
                points: SAMPLES.map(|s| centre + axis * (s * body.half_length)),
                radius: body.radius,
                group,
            });
        }
        (!guards.is_empty()).then(|| Self {
            section,
            baseline: vec![[0.0; 3]; guards.len()],
            guards,
            limb_sides: vec![1.0; resolved.limbs.len()],
        })
    }

    /// A guard's sample points for the pose `component`: in the rider's space and in the bike's frame.
    fn points<'a>(
        guard: &'a Guard,
        component: &'a [Transform],
        frame: &'a Frame,
    ) -> impl Iterator<Item = (Vec3, Vec3)> + 'a {
        let bone = component[guard.bone];
        guard.points.iter().map(move |p| {
            let rider = bone.transform_point(*p);
            (rider, frame.to_bike.transform_point3(rider))
        })
    }

    /// Where the middle joint of limb number `index` (`limb`, in the order of `Resolved::limbs`) is to be swung to in the pose `component`
    /// to clear the bike's body, as the pole of the two-bone solve; `None` when the limb is clear. The joint is moved outward, sideways in
    /// the bike's frame, by the deepest push any point of the limb's two segments asks for, each divided by how much of the segment the
    /// joint's move carries ([`MIN_LEVER`]), at most [`MAX_PUSH_M`].
    pub fn limb_pole(
        &self,
        (index, limb): (usize, &super::pose::Limb),
        component: &[Transform],
        frame: &Frame,
    ) -> Option<Vec3> {
        let at = |bone: usize| component[bone].translation;
        let side = self.limb_sides.get(index).copied().unwrap_or(1.0);
        let mut needed = 0.0_f32;
        for (g, guard) in
            self.guards.iter().enumerate().filter(|(_, g)| matches!(g.group, Group::Limb { limb, .. } if limb == index))
        {
            let Group::Limb { upper, .. } = guard.group else { continue };
            let (from, to) = match upper {
                true => (at(limb.chain.root), at(limb.chain.mid)),
                false => (at(limb.chain.mid), at(limb.chain.tip)),
            };
            let segment = to - from;
            let length2 = segment.length_squared().max(1e-8);
            for (k, (rider, point)) in Self::points(guard, component, frame).enumerate() {
                let (deep, _) = self.excess(g, k, point);
                if deep <= 0.0 {
                    continue;
                }
                let along = ((rider - from).dot(segment) / length2).clamp(0.0, 1.0);
                let lever = match upper {
                    true => along,
                    false => 1.0 - along,
                };
                if lever < MIN_LEVER {
                    continue;
                }
                needed = needed.max(deep / lever);
            }
        }
        if needed <= 0.0 {
            return None;
        }
        let outward = frame.to_rider.transform_vector3(Vec3::Y * side).normalize_or_zero();
        let moved = at(limb.chain.mid) + outward * needed.min(MAX_PUSH_M);
        // The pole is a point: the solve takes the direction from the limb's axis to it, so the joint's new place itself will do.
        Some(moved)
    }

    /// How far the whole body is to be moved over, a vector of the rider's space, for the limbs that are still inside the bike's body in the
    /// pose `component` after their joints were swung out: the deepest excess of any point of any limb, outward on that limb's side, at most
    /// [`MAX_BODY_PUSH_M`]. Zero when every limb is clear.
    pub fn leftover(&self, component: &[Transform], frame: &Frame) -> Vec3 {
        let mut best = (0.0_f32, 0.0_f32);
        for (g, guard) in self.guards.iter().enumerate() {
            let Group::Limb { limb, .. } = guard.group else { continue };
            let side = self.limb_sides.get(limb).copied().unwrap_or(1.0);
            for (k, (_, point)) in Self::points(guard, component, frame).enumerate() {
                let (deep, _) = self.excess(g, k, point);
                if deep > best.0 {
                    best = (deep, side);
                }
            }
        }
        match best.0 > 0.0 {
            true => {
                frame.to_rider.transform_vector3(Vec3::Y * best.1).normalize_or_zero() * best.0.min(MAX_BODY_PUSH_M)
            }
            false => Vec3::ZERO,
        }
    }

    /// How far the torso is to be moved over, a vector of the rider's space, in the pose `component`: the deepest push of a torso capsule
    /// point that is on one side of the section's centre (beyond [`TORSO_OUTER`] of the half width), outward, at most [`MAX_TORSO_PUSH_M`].
    /// Zero when the chest is clear.
    pub fn torso(&self, component: &[Transform], frame: &Frame) -> Vec3 {
        let mut best = (0.0_f32, 0.0_f32);
        for (g, guard) in self.guards.iter().enumerate().filter(|(_, g)| g.group == Group::Torso) {
            for (k, (_, point)) in Self::points(guard, component, frame).enumerate() {
                let (deep, wall) = self.excess(g, k, point);
                if deep > best.0 && point.y.abs() > TORSO_OUTER * wall {
                    best = (deep, point.y.signum());
                }
            }
        }
        match best.0 > 0.0 {
            true => {
                frame.to_rider.transform_vector3(Vec3::Y * best.1).normalize_or_zero() * best.0.min(MAX_TORSO_PUSH_M)
            }
            false => Vec3::ZERO,
        }
    }

    /// The deepest any capsule point of the rider is inside the section in the pose `component`, metres (0: clear): the number the numeric
    /// check reads, with the same capsules and the same tolerance the push works to.
    #[cfg(test)]
    pub fn deepest(&self, component: &[Transform], frame: &Frame) -> f32 {
        let mut worst = 0.0_f32;
        for (g, guard) in self.guards.iter().enumerate() {
            for (k, (_, point)) in Self::points(guard, component, frame).enumerate() {
                worst = worst.max(self.excess(g, k, point).0);
            }
        }
        worst
    }

    /// [`Clearance::deepest`] by group: the torso, the arms (limbs 0 and 1 of the rig) and the legs (2 and 3).
    #[cfg(test)]
    pub fn deepest_by_group(&self, component: &[Transform], frame: &Frame) -> [f32; 3] {
        let mut worst = [0.0_f32; 3];
        for (g, guard) in self.guards.iter().enumerate() {
            let at = match guard.group {
                Group::Torso => 0,
                Group::Limb { limb, .. } if limb < 2 => 1,
                Group::Limb { .. } => 2,
            };
            for (k, (_, point)) in Self::points(guard, component, frame).enumerate() {
                worst[at] = worst[at].max(self.excess(g, k, point).0);
            }
        }
        worst
    }

    /// How deep a skin vertex at `p` (the bike's frame) is inside the section, metres (0: clear): the numeric check's measure for the rider's
    /// mesh, which has no radius to add.
    #[cfg(test)]
    pub fn vertex_depth(&self, p: Vec3) -> f32 {
        let wall = self.section.half_width(p.x, p.z);
        match wall > 0.0 {
            true => (wall - p.y.abs()).max(0.0),
            false => 0.0,
        }
    }

    /// Whether any guard is on the rider (for tests).
    #[cfg(test)]
    pub fn guard_count(&self) -> usize {
        self.guards.len()
    }
}

#[cfg(test)]
#[path = "clearance_tests.rs"]
mod tests;
