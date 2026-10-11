//! Ground-following track segments and one reusable stamp for a stationary marking wheel.

use std::collections::VecDeque;

use blackbox_gfx::{EffectLayer, EffectVertex};
use glam::Vec3;

use super::{Contact, MAX_MARKS};

const OFFSET: f32 = 0.012;
const MIN_DISTANCE: f32 = 0.18;
const MAX_DISTANCE: f32 = 3.0;
const LIFETIME: f32 = 45.0;

#[derive(Clone, Copy)]
struct Mark {
    corners: [Vec3; 4],
    section: u32,
    opacity: f32,
    age: f32,
    id: u64,
    stationary: bool,
}

#[derive(Clone, Copy)]
struct Anchor {
    contact: Contact,
    stamp: u64,
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
        self.marks.retain(|m| m.age < LIFETIME);
    }

    pub fn retain_sections(&mut self, loaded: impl Fn(u32) -> bool) {
        self.marks.retain(|m| loaded(m.section));
        for anchor in &mut self.anchors {
            if anchor.is_some_and(|a| !loaded(a.contact.section)) {
                *anchor = None;
            }
        }
    }

    fn push(&mut self, corners: [Vec3; 4], contact: Contact, stationary: bool) -> u64 {
        if self.marks.len() == MAX_MARKS {
            self.marks.pop_front();
        }
        self.created += 1;
        self.marks.push_back(Mark {
            corners,
            section: contact.section,
            opacity: contact.skid * 0.65,
            age: 0.0,
            id: self.created,
            stationary,
        });
        self.created
    }

    fn stamp(&mut self, c: Contact) -> u64 {
        self.push(stamp_corners(c), c, true)
    }

    pub fn sample(&mut self, wheel: usize, contact: Option<Contact>) {
        let Some(c) = contact.filter(|c| c.skid > 0.0 && c.point.is_finite()) else {
            self.anchors[wheel] = None;
            return;
        };
        let previous = self.anchors[wheel];
        let Some(a) = previous.filter(|a| {
            a.contact.section == c.section
                && a.contact.normal.dot(c.normal) >= 0.8
                && a.contact.point.distance(c.point) <= MAX_DISTANCE
        }) else {
            let stamp = self.stamp(c);
            self.anchors[wheel] = Some(Anchor { contact: c, stamp });
            return;
        };
        if a.contact.point.distance(c.point) < MIN_DISTANCE {
            if let Some(stamp) = self.marks.iter_mut().rev().find(|m| m.id == a.stamp) {
                if stamp.stationary {
                    stamp.corners = stamp_corners(c);
                }
                stamp.opacity = stamp.opacity.max(c.skid * 0.65);
                stamp.age = 0.0;
                return;
            }
            let stamp = self.stamp(c);
            self.anchors[wheel] = Some(Anchor { contact: c, stamp });
            return;
        }
        let [al, ar] = edge(a.contact);
        let [bl, br] = edge(c);
        let stamp = self.push([al, ar, br, bl], c, false);
        self.anchors[wheel] = Some(Anchor { contact: c, stamp });
    }

    pub fn geometry(&self, out: &mut Vec<EffectVertex>) {
        for mark in &self.marks {
            let fade = ((LIFETIME - mark.age) / 5.0).min(1.0);
            EffectLayer::quad(out, mark.corners, [16, 14, 12, (mark.opacity * fade * 255.0) as u8]);
        }
    }
}

fn stamp_corners(c: Contact) -> [Vec3; 4] {
    let [left, right] = edge(c);
    let along = c.forward * MIN_DISTANCE * 0.5;
    [left - along, right - along, right + along, left + along]
}
