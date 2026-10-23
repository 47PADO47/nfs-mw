//! The career in the world: the pursuit and bust results, the zones, the closed regions, the HUD rows and the phone,
//! for the career being played (`crate::career`). Outside career mode none of this runs.
//!
//! Each concern is its own file with an `impl WorldScene` block; [`WorldScene::update_career`] calls them once a
//! frame after the simulation and the entry zones.

mod bust;
mod cooling;
mod coop_bust;
mod drive_to_target;
mod events;
mod hud;
mod ladder;
mod phone;
mod pursuit;
mod rap_tally;
mod regions;
mod respawn;
mod rewards;
mod safehouse;
pub(super) mod sms_popup;
mod speedtraps;
pub(super) mod story_command;

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
    /// The respawn markers visited, and a respawn waiting for the game to run again after a bust.
    pub respawn: respawn::Respawns,
    /// The paint cooling table of the install, read when the garage first fits a part (`None` inside: not readable).
    pub cooling: Option<Option<nfsmw_career::economy::cooling::CoolingTable>>,
    /// The events of the challenge bin the player has won this session (`events.rs`).
    pub side_won: std::collections::BTreeSet<String>,
    /// The speed traps the car is inside (`speedtraps.rs`).
    pub traps: speedtraps::TrapWatch,
    /// The drive to the first safe house, asked for and running (`drive_to_target.rs`).
    pub safe_house_drive: drive_to_target::SafeHouseDrive,
    /// The scripted chase of race 16.2.1 while that race runs (`race/scripted_chase.rs`).
    pub chase: Option<super::race::scripted_chase::ChaseRun>,
    /// The places the scripted chase of 16.2.1 asked cops at (physics space: position and facing), until the police step places
    /// them (`ai::scripted_cops`).
    pub scripted_cops: Vec<(glam::Vec3, glam::Vec3)>,
    /// The milestones the last chase banked, until the front end takes them for the rewards screen.
    pub rewards: Option<crate::viewer::RewardsReport>,
    /// The speed trap the car has just crossed, until the front end takes it for the photo.
    pub trap_photo: Option<crate::viewer::PhotoReport>,
}

impl WorldScene {
    /// Plays `handle`'s career in this world (the front end calls it before the world is shown).
    pub fn set_career(&mut self, handle: CareerHandle, profiles: Option<ProfileDir>) {
        self.career.handle = handle;
        self.career.profiles = profiles;
        self.seed_side_won();
    }

    /// The career's part of a frame.
    pub(super) fn update_career(&mut self, dt: f32) {
        self.coop_auto_pay_ticket();
        if !self.career.handle.is_active() {
            return;
        }
        self.career_respawn();
        self.career_pursuit(dt);
        self.career_speed_traps();
        self.screen_fade.advance(dt);
        self.update_scripted_chase(dt);
        self.career_drive_to_target(dt);
        self.career_regions();
        self.career_hud();
        self.career_phone(dt);
    }
}
