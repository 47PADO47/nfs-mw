//! Remote players joining and leaving the world. A remote player is a co-op guest on its own `PlayerId` (ADR 0011
//! section 3): it comes in through the co-op spawn path (`players/spawn.rs`), the world's co-op session holds the shared
//! chase it takes part in, and it is busted and sent to the safe house by the co-op rules (`career/coop_bust.rs`). The
//! co-op session is opened with the first remote player and closed with the last.

use anyhow::{Context, Result};
use blackbox_gfx::RenderBackend;
use blackbox_pursuit::targets::TargetId;
use glam::Vec3;

use super::super::players::coop::{CoopSession, GUEST};
use super::super::players::tether::{Tether, behind};
use super::super::players::{LEAD, PlayerId};
use super::super::{WorldScene, car_loader};
use crate::input::CoopDevice;
use crate::settings::Transmission;

/// The most players a session seats: the player cars' ids are 0 to 15 (ADR 0009 section 1).
pub const MAX_PLAYERS: u8 = 16;

/// How far behind the lead a remote player starts when the session picks no spot, per place in the line, metres.
const JOIN_BEHIND: f32 = 12.0;

/// The name a remote player has in the world until the session names it.
pub fn seat_name(id: PlayerId) -> String {
    format!("Player {}", u16::from(id.0) + 1)
}

impl WorldScene {
    /// Bring remote player `id` (not the lead) into the world in the car in folder `folder` (`CAR[:PRESET]`): at `spawn`
    /// (render space) when the session picks the spot, else in a line behind the lead.
    pub(in crate::scenes::world) fn seat_remote(
        &mut self,
        renderer: &mut dyn RenderBackend,
        id: PlayerId,
        folder: &str,
        spawn: Option<[f32; 3]>,
    ) -> Result<()> {
        anyhow::ensure!(id != LEAD && id.0 < MAX_PLAYERS, "player {} is not a remote seat", id.0);
        let (folder, model) =
            car_loader::load_named_car(&self.dir, folder).with_context(|| format!("the car {folder} did not load"))?;
        let lead = self.players.lead().map(|drive| (drive.position(), drive.heading()));
        let (near, heading) = match (spawn, lead) {
            (Some(at), _) => ([at[0], at[1]], lead.map(|(_, heading)| heading)),
            (None, Some((position, heading))) => {
                let forward = Vec3::new(heading.cos(), heading.sin(), 0.0);
                let at = behind(position, forward, JOIN_BEHIND * f32::from(id.0));
                ([at.x, at.y], Some(heading))
            }
            (None, None) => ([self.camera.position.x, self.camera.position.y], None),
        };
        self.add_player(renderer, id, folder.clone(), model, near, heading)?;
        self.open_remote_session(id, &folder);
        log::info!("net: player {} joins in {folder}", u16::from(id.0) + 1);
        Ok(())
    }

    /// Take remote player `id` out of the world; the lead stays. Returns whether a player left.
    pub(in crate::scenes::world) fn unseat_remote(&mut self, renderer: &mut dyn RenderBackend, id: PlayerId) -> bool {
        if !self.remove_player(renderer, id) {
            return false;
        }
        if let Some(session) = self.coop.as_mut() {
            if id == GUEST {
                session.guest_car.clear();
            }
            session.chase.targets.leave(TargetId(id.0));
            session.catch_up.retain(|&offered| offered != id);
            session.cutscene_parked.retain(|&parked| parked != id);
        }
        let others = self.players.iter().any(|player| player.id != LEAD);
        let parked = (1..MAX_PLAYERS).any(|n| self.players.is_parked(PlayerId(n)));
        if !others && !parked {
            self.coop = None;
            log::info!("net: the last remote player left; the world is single player again");
        }
        log::info!("net: player {} leaves", u16::from(id.0) + 1);
        true
    }

    /// The co-op session that carries the remote players' shared chase, opened if there is none. Player 2's car is the
    /// guest car a co-op race seats (`players/race.rs`).
    fn open_remote_session(&mut self, id: PlayerId, folder: &str) {
        let session = self.coop.get_or_insert_with(|| CoopSession {
            names: (0..MAX_PLAYERS).map(|n| seat_name(PlayerId(n))).collect(),
            guest_handle: None,
            guest_car: String::new(),
            transmission: Transmission::default(),
            tether: Tether::default(),
            catch_up: Vec::new(),
            chase: Default::default(),
            suspended: false,
            cutscene_parked: Vec::new(),
            device: CoopDevice::Arrows,
        });
        if id == GUEST {
            session.guest_car = folder.to_owned();
        }
    }
}
