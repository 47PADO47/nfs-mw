//! `contacts.json`: the seat, the hubs and the grip and peg points of one bike, laid over the shared `attach.json`.
//!
//! A mod ships one rider rig for all its bikes (`files/RIDER/`: skeleton, clips, meshes, `attach.json`). Each bike has its
//! own seat, handlebar and pegs, so a bike's own `files/CARS/<ID>/RIDER/contacts.json` says where they are on that bike
//! (`docs/formats/ride6-rider.md`, "contacts.json"); the targets the limbs reach for, the seat the rider's root sits on and the
//! hubs the bike's frame is joined to the model by are replaced, everything else (limbs, offsets, inertia) stays the rig's.
//! A bike without the file keeps the shared targets, and a file that is damaged is logged and ignored.

use anyhow::{Context, Result, ensure};
use game_install::GameDir;
use glam::Vec3;
use serde::Deserialize;

use super::attach::{Attach, Hubs, Rigid, Target};

/// The only version of the file this reads.
pub const VERSION: u32 = 1;

/// How far from the seat a target may be, metres. A bound for damaged files, not a modelling value: the real bikes' grips are
/// 0.5 to 0.74 m from the seat and the pegs 0.46 to 0.62 m.
pub const MAX_REACH_M: f32 = 1.5;

/// The bike's rider row of the animation correction table, as the converter copied it. Carried with the contacts; the rider
/// runtime does not apply it yet (the knee, elbow, tank and spine corrections need the animation blueprint's control rigs).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[allow(dead_code)]
pub struct Correction {
    pub rider_max_lean: f32,
    pub aero_tank: f32,
    pub lean_knee: f32,
    pub lean_elbow: f32,
    pub knee: f32,
    pub elbow: f32,
    pub muffler: f32,
    pub spine_1_deg: f32,
    pub spine_2_deg: f32,
    pub arms_ik_spine_alpha: f32,
    pub pole_elbow_cm: [f32; 3],
    pub bike_type: Option<String>,
    pub quick_shift: Option<String>,
}

/// One bike's contact points, in the bike skeleton's frame (x forward, y left, z up, metres).
#[derive(Debug, Clone, Deserialize)]
pub struct Contacts {
    pub version: u32,
    pub rider_root_in_bike: Rigid,
    pub hubs: Hubs,
    pub targets: Vec<Target>,
    #[serde(default)]
    pub correction: Option<Correction>,
    /// The seat's top along the centre line (`[x, z]`, x ascending), measured on the bike's mesh by the converter.
    #[serde(default)]
    pub seat_surface: Vec<[f32; 2]>,
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}

impl Contacts {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let contacts: Self = serde_json::from_slice(bytes).context("contacts.json")?;
        contacts.check()?;
        Ok(contacts)
    }

    /// Refuses a file that would put the rider somewhere absurd: another version, a number that is not finite, a seat with no
    /// turn, hubs the wrong way round, a target far from the seat.
    fn check(&self) -> Result<()> {
        ensure!(self.version == VERSION, "contacts.json is version {}, this reads version {VERSION}", self.version);
        let seat = &self.rider_root_in_bike;
        ensure!(finite(&seat.translation) && finite(&seat.rotation), "the seat is not a finite transform");
        ensure!(seat.rotation.iter().any(|c| c.abs() > 1e-6), "the seat has no rotation");
        ensure!(finite(&self.hubs.front) && finite(&self.hubs.rear), "a hub is not finite");
        ensure!(self.hubs.front[0] > self.hubs.rear[0], "the front hub is not ahead of the rear one");
        for point in &self.seat_surface {
            ensure!(finite(point), "a point of the seat's surface is not finite");
        }
        ensure!(
            self.seat_surface.windows(2).all(|w| w[0][0] < w[1][0]),
            "the seat's surface is not in order along the bike"
        );
        let seat = Vec3::from_array(seat.translation);
        for target in &self.targets {
            ensure!(finite(&target.position), "target {} is not finite", target.name);
            ensure!(
                target.rotation.is_none_or(|q| finite(&q) && q.iter().any(|c| c.abs() > 1e-6)),
                "target {} has no usable turn",
                target.name
            );
            let reach = Vec3::from_array(target.position).distance(seat);
            ensure!(reach <= MAX_REACH_M, "target {} is {reach:.2} m from the seat", target.name);
        }
        Ok(())
    }
}

impl Attach {
    /// The rig's attachment with this bike's seat, hubs and targets: a target the bike names replaces the rig's of the same
    /// name, one it leaves out stays. Fails if a limb is left with no target to reach for.
    pub fn with_contacts(mut self, contacts: &Contacts) -> Result<Self> {
        self.rider_root_in_bike = contacts.rider_root_in_bike;
        self.hubs = contacts.hubs;
        self.seat_surface.clone_from(&contacts.seat_surface);
        for target in &contacts.targets {
            match self.target_index(&target.name) {
                Some(at) => self.targets[at] = target.clone(),
                None => self.targets.push(target.clone()),
            }
        }
        for limb in &self.limbs {
            ensure!(self.target_index(&limb.target).is_some(), "limb {} has no target {}", limb.name, limb.target);
        }
        Ok(self)
    }
}

/// `attach` with the contacts of the car `car` laid over it, if the car has a `contacts.json`. A file that cannot be used is
/// logged and the rig's own targets are kept.
pub fn lay_over(attach: Attach, dir: &GameDir, car: &str) -> Attach {
    let path = format!("{}/contacts.json", super::folders(car)[0]);
    if !dir.exists(&path) {
        log::info!("rider: {car} has no {path}; the rider reaches the rig's shared grips and pegs");
        return attach;
    }
    let merged = dir
        .read(&path)
        .with_context(|| format!("reading {path}"))
        .and_then(|bytes| Contacts::parse(&bytes))
        .and_then(|contacts| attach.clone().with_contacts(&contacts).map(|merged| (merged, contacts)));
    match merged {
        Ok((merged, contacts)) => {
            let kind = contacts.correction.as_ref().and_then(|c| c.bike_type.as_deref()).unwrap_or("unknown type");
            log::info!("rider: {car} rides on its own seat, grips and pegs ({path}, {kind})");
            merged
        }
        Err(error) => {
            log::warn!("rider: {path} is not used: {error:#}");
            attach
        }
    }
}

#[cfg(test)]
#[path = "contacts_tests.rs"]
mod tests;
