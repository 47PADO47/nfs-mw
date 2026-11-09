//! Ground-following track segments and one reusable stamp for a stationary marking wheel.
//!
//! The game's own marks are dark untextured strips. A mark remembers what a skid mark variant (`marks_variant.rs`) needs to
//! draw it with a picture: which kind of road and weather it was made on, whether the tyre slid sideways or spun, how far along
//! its strip it starts and ends, and how long it stays.

use std::collections::VecDeque;

use blackbox_gfx::{EffectLayer, EffectVertex};
use blackbox_modding::content::{MarkSurface, SkidMarksProfile};
use glam::Vec3;

use super::{Contact, MAX_MARKS};

const OFFSET: f32 = 0.012;
const MIN_DISTANCE: f32 = 0.18;
const MAX_DISTANCE: f32 = 3.0;
const LIFETIME: f32 = 45.0;
/// The seconds before the end of its life in which a mark fades.
const FADE: f32 = 5.0;
/// The sideways share of the slide (`Contact::slide`) above which a mark is a slip mark and not a spin mark.
const SLIP_SHARE: f32 = 0.4;
/// The strongest a mark's opacity gets (the skid intensity times it); a variant's picture is scaled by the share of it.
const STRONGEST: f32 = 0.65;

/// One quad of a tyre's track.
#[derive(Clone, Copy)]
pub(super) struct Mark {
    corners: [Vec3; 4],
    section: u32,
    opacity: f32,
    age: f32,
    id: u64,
    stationary: bool,
    /// Where the strip's start and end are along its track, metres from where the track began.
    along: [f32; 2],
    /// The road's normal where the mark starts.
    normal: Vec3,
    pub(super) slip: bool,
    pub(super) loose: bool,
    pub(super) wet: bool,
    /// How long the mark stays, seconds.
    life: f32,
}

impl Mark {
    /// The kind of road the mark was made on.
    pub(super) fn surface(&self) -> MarkSurface {
        match self.loose {
            true => MarkSurface::Loose,
            false => MarkSurface::Paved,
        }
    }

    /// The corners of the quad (start left, start right, end right, end left), lifted off the road by `lift` metres.
    pub(super) fn lifted(&self, lift: f32) -> [Vec3; 4] {
        self.corners.map(|corner| corner + self.normal * lift)
    }

    /// Where the strip starts and ends along its track, metres.
    pub(super) fn along(&self) -> [f32; 2] {
        self.along
    }

    /// How strong the mark is, 0 to 1, as the tyre's skid made it.
    pub(super) fn strength(&self) -> f32 {
        (self.opacity / STRONGEST).clamp(0.0, 1.0)
    }

    /// The share of the mark that shows: 1 until its last seconds, then falling to 0.
    pub(super) fn fade(&self) -> f32 {
        ((self.life - self.age) / FADE).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy)]
struct Anchor {
    contact: Contact,
    stamp: u64,
    /// Metres of track since it began.
    along: f32,
}

pub(super) struct Marks {
    marks: VecDeque<Mark>,
    anchors: [Option<Anchor>; 4],
    pub created: u64,
}

impl Default for Marks {
    fn default() -> Self {
        Self { marks: VecDeque::with_capacity(MAX_MARKS), anchors: [None; 4], created: 0 }
    }
}

fn edge(c: Contact) -> [Vec3; 2] {
    let side = c.forward.cross(c.normal).normalize_or_zero() * (c.width * 0.5);
    let point = c.point + c.normal * OFFSET;
    [point - side, point + side]
}

fn surface_of(contact: Contact) -> MarkSurface {
    match contact.loose {
        true => MarkSurface::Loose,
        false => MarkSurface::Paved,
    }
}

/// What a skid mark variant changes about a mark made now: the width of the tread and how long the mark stays.
fn styled(look: Option<&SkidMarksProfile>, wet: bool, mut contact: Contact) -> (Contact, f32) {
    let set = look.and_then(|profile| profile.set_for(surface_of(contact), wet));
    contact.width *= set.map_or(1.0, |set| set.width);
    (contact, set.map_or(LIFETIME, |set| set.life))
}

/// A quad of the track and what it was made of.
struct Piece {
    corners: [Vec3; 4],
    contact: Contact,
    along: [f32; 2],
    stationary: bool,
    life: f32,
    wet: bool,
}

impl Marks {
    pub fn len(&self) -> usize {
        self.marks.len()
    }

    pub fn oldest(&self) -> f32 {
        self.marks.iter().map(|m| m.age).fold(0.0, f32::max)
    }

    pub fn clear(&mut self) {
        self.marks.clear();
        self.disconnect();
    }

    pub fn disconnect(&mut self) {
        self.anchors = [None; 4];
    }

    pub fn age(&mut self, dt: f32) {
        for m in &mut self.marks {
            m.age += dt;
        }
        self.marks.retain(|m| m.age < m.life);
    }

    pub fn retain_sections(&mut self, loaded: impl Fn(u32) -> bool) {
        self.marks.retain(|m| loaded(m.section));
        for anchor in &mut self.anchors {
            if anchor.is_some_and(|a| !loaded(a.contact.section)) {
                *anchor = None;
            }
        }
    }

    /// The marks, oldest first.
    pub(super) fn iter(&self) -> impl Iterator<Item = &Mark> {
        self.marks.iter()
    }

    fn push(&mut self, piece: Piece) -> u64 {
        if self.marks.len() == MAX_MARKS {
            self.marks.pop_front();
        }
        let Piece { corners, contact, along, stationary, life, wet } = piece;
        self.created += 1;
        self.marks.push_back(Mark {
            corners,
            section: contact.section,
            opacity: contact.skid * STRONGEST,
            age: 0.0,
            id: self.created,
            stationary,
            along,
            normal: contact.normal,
            slip: contact.slide >= SLIP_SHARE,
            loose: contact.loose,
            wet,
            life,
        });
        self.created
    }

    fn stamp(&mut self, c: Contact, life: f32, wet: bool) -> u64 {
        let piece =
            Piece { corners: stamp_corners(c), contact: c, along: [0.0, MIN_DISTANCE], stationary: true, life, wet };
        self.push(piece)
    }

    /// One physics step of a wheel. `wet` is whether the road is wet, `look` the skid mark variant in effect (it sets the width
    /// of the tread and how long a mark stays; where marks begin and end is the physics' own).
    pub fn sample(&mut self, wheel: usize, contact: Option<Contact>, wet: bool, look: Option<&SkidMarksProfile>) {
        let Some(c) = contact.filter(|c| c.skid > 0.0 && c.point.is_finite()) else {
            self.anchors[wheel] = None;
            return;
        };
        let (c, life) = styled(look, wet, c);
        let previous = self.anchors[wheel];
        let Some(a) = previous.filter(|a| {
            a.contact.section == c.section
                && a.contact.normal.dot(c.normal) >= 0.8
                && a.contact.point.distance(c.point) <= MAX_DISTANCE
        }) else {
            let stamp = self.stamp(c, life, wet);
            self.anchors[wheel] = Some(Anchor { contact: c, stamp, along: 0.0 });
            return;
        };
        let distance = a.contact.point.distance(c.point);
        if distance < MIN_DISTANCE {
            if let Some(stamp) = self.marks.iter_mut().rev().find(|m| m.id == a.stamp) {
                if stamp.stationary {
                    stamp.corners = stamp_corners(c);
                }
                stamp.opacity = stamp.opacity.max(c.skid * STRONGEST);
                stamp.age = 0.0;
                return;
            }
            let stamp = self.stamp(c, life, wet);
            self.anchors[wheel] = Some(Anchor { contact: c, stamp, along: 0.0 });
            return;
        }
        let [al, ar] = edge(a.contact);
        let [bl, br] = edge(c);
        let along = [a.along, a.along + distance];
        let piece = Piece { corners: [al, ar, br, bl], contact: c, along, stationary: false, life, wet };
        let stamp = self.push(piece);
        self.anchors[wheel] = Some(Anchor { contact: c, stamp, along: along[1] });
    }

    /// The game's own dark strips for the marks `covered` leaves out (a skid mark variant draws the others with pictures).
    pub fn geometry(&self, out: &mut Vec<EffectVertex>, covered: impl Fn(&Mark) -> bool) {
        for mark in self.marks.iter().filter(|mark| !covered(mark)) {
            EffectLayer::quad(out, mark.corners, [16, 14, 12, (mark.opacity * mark.fade() * 255.0) as u8]);
        }
    }
}

fn stamp_corners(c: Contact) -> [Vec3; 4] {
    let [left, right] = edge(c);
    let along = c.forward * MIN_DISTANCE * 0.5;
    [left - along, right - along, right + along, left + along]
}
