//! The career in the world: the pursuit and bust results, the zones, the closed regions, the HUD rows and the phone,
//! for the career being played (`crate::career`). Outside career mode none of this runs.
//!
//! Each concern is its own file with an `impl WorldScene` block; [`WorldScene::update_career`] calls them once a
//! frame after the simulation and the entry zones.

mod bust;
mod cooling;
mod events;
mod hud;
mod phone;
mod pursuit;
mod rap_tally;
mod regions;
mod safehouse;

use std::collections::VecDeque;

pub(super) use events::RaceEntry;

use super::WorldScene;
use crate::career::{CareerHandle, CareerRequest, ProfileDir};

/// The career state the world keeps between frames.
#[derive(Default)]
pub(super) struct WorldCareer {
    /// The career being played; empty outside career mode.
    pub handle: CareerHandle,
    /// Where autosaves go; `None` without a per-user folder.
    pub profiles: Option<ProfileDir>,
    /// What the front end has to do, oldest first (`Scene::take_career_request`).
    pub requests: VecDeque<CareerRequest>,
    pub pursuit: pursuit::PursuitWatch,
    pub regions: regions::RegionState,
    pub phone: phone::PhoneState,
    pub hud: hud::CareerHud,
    /// The paint cooling table of the install, read when the garage first fits a part (`None` inside: not readable).
    pub cooling: Option<Option<nfsmw_career::economy::cooling::CoolingTable>>,
}

impl WorldScene {
    /// Plays `handle`'s career in this world (the front end calls it before the world is shown).
    pub fn set_career(&mut self, handle: CareerHandle, profiles: Option<ProfileDir>) {
        self.career.handle = handle;
        self.career.profiles = profiles;
    }

    /// The career's part of a frame.
    pub(super) fn update_career(&mut self, dt: f32) {
        if !self.career.handle.is_active() {
            return;
        }
        self.career_pursuit(dt);
        self.career_regions();
        self.career_hud();
        self.career_phone(dt);
    }
}
