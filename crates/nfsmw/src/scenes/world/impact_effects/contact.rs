use blackbox_attrib::RefSpec;
use glam::Vec3;
use std::{
    rc::Weak,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    World,
    Object,
    Ground,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawContact {
    pub kind: Kind,
    pub surface: u32,
    pub point: Vec3,
    pub normal: Vec3,
    /// Native second-owner detachment uses the original pair normal before its base callback flips it.
    pub detached_normal: Vec3,
    pub closing: Vec3,
    pub sliding: Vec3,
    pub delta_v: f32,
    pub owner_velocity: Vec3,
    pub detached: bool,
    pub bottom: bool,
}

impl RawContact {
    pub fn ground(contact: &blackbox_vehicle::rigid_body::GroundContact, owner_velocity: Vec3) -> Self {
        let mut raw = Self::new(
            Kind::Ground,
            contact.surface,
            contact.point,
            contact.normal,
            contact.velocity,
            contact.impulse_delta_v,
            owner_velocity,
        );
        raw.closing = -contact.normal * contact.closing_speed.abs();
        raw
    }
    pub fn new(
        kind: Kind,
        surface: u32,
        point: Vec3,
        normal: Vec3,
        relative: Vec3,
        delta_v: f32,
        owner_velocity: Vec3,
    ) -> Self {
        let closing = normal * relative.dot(normal);
        Self {
            kind,
            surface,
            point,
            normal,
            detached_normal: normal,
            closing,
            sliding: relative - closing,
            delta_v,
            owner_velocity,
            detached: false,
            bottom: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Contact {
    pub source: RefSpec,
    pub owner: u64,
    pub lifetime: Weak<()>,
    pub raw: RawContact,
}

pub fn next_owner() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
