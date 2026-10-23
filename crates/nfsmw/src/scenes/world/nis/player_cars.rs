//! The NIS player's side of the scene's cars: linking the car channels to the world's cars, their poses, and where
//! they are left when the scene ends.

use super::{CarMemory, CarSpot, CopWait, NisPlayer, Playing, Target};

impl NisPlayer {
    /// Whether a scene plays and its car channels still wait for their real cars (see [`NisPlayer::set_links`]).
    pub fn needs_links(&self) -> bool {
        self.play.as_ref().is_some_and(|playing| playing.links.is_none())
    }

    /// Gives the playing scene's car channels their real cars. Called once per scene: a channel missing from `links`
    /// stays off. Does nothing when no scene plays.
    pub fn set_links(&mut self, links: Vec<(usize, Target)>) {
        if let Some(playing) = self.play.as_mut() {
            playing.links = Some(links);
        }
    }

    /// The cop channels of the playing scene still waiting for a police car, in slot order. Empty when none waits or no
    /// scene plays.
    pub fn cop_waiting(&self) -> &[usize] {
        self.play.as_ref().map_or(&[][..], |playing| playing.cop_wait.slots())
    }

    /// Makes the cop channels `slots` of the playing scene wait for a police car (see [`CopWait`]). Called once per scene,
    /// after [`NisPlayer::set_links`]. Does nothing when no scene plays.
    pub fn wait_for_cops(&mut self, slots: Vec<usize>) {
        if let Some(playing) = self.play.as_mut() {
            playing.cop_wait = CopWait::new(slots);
        }
    }

    /// Gives the waiting cop channel `slot` its real car: the channel is linked and no longer waits.
    pub fn link_waiting_cop(&mut self, slot: usize, target: Target) {
        let Some(playing) = self.play.as_mut() else { return };
        playing.cop_wait.resolve(slot);
        if let Some(links) = playing.links.as_mut() {
            links.push((slot, target));
        }
    }

    /// The traffic indices of the police cars the linked channels take: the cars the scene borrows or spawns.
    pub fn linked_cars(&self) -> Vec<usize> {
        let Some(links) = self.play.as_ref().and_then(|playing| playing.links.as_ref()) else { return Vec::new() };
        links
            .iter()
            .filter_map(|&(_, target)| match target {
                Target::Traffic(index) | Target::Spawned(index) => Some(index),
                Target::Player => None,
            })
            .collect()
    }

    /// Spends `dt` real seconds on the waiting cop channels. Returns the channels that give up now, once their time is
    /// up (see [`CopWait::tick`]).
    pub fn tick_cop_wait(&mut self, dt: f32) -> Vec<usize> {
        self.play.as_mut().map_or_else(Vec::new, |playing| playing.cop_wait.tick(dt))
    }

    /// Each car channel of the playing scene with its slot and its pose now, in render space and before the ground lift.
    /// Empty when no scene plays.
    pub fn car_slots(&self) -> Vec<(usize, CarSpot)> {
        let Some(playing) = self.play.as_ref() else { return Vec::new() };
        let time = playing.run.scene_time();
        playing.cars.iter().map(|car| (car.slot, car.spot(&playing.anchor, time))).collect()
    }

    /// The linked cars' poses now, in render space and before the ground lift. Empty when no scene plays or none is
    /// linked.
    pub fn car_spots(&self) -> Vec<(Target, CarSpot)> {
        self.play.as_ref().map_or_else(Vec::new, Playing::spots)
    }

    /// The playing scene's memory of its cars; `None` when no scene plays.
    pub fn car_memory(&mut self) -> Option<&mut CarMemory> {
        self.play.as_mut().map(|playing| &mut playing.memory)
    }

    /// The linked cars' poses where the last scene ended, once: the next call gives `None` until another scene ends.
    pub fn take_final_cars(&mut self) -> Option<Vec<(Target, CarSpot)>> {
        self.final_cars.take()
    }

    /// Whether the scene that plays is a race's start, which leaves the player's car on its grid slot (see
    /// [`NisPlayer::keep_player_in_place`]).
    pub fn race_start_scene(&self) -> bool {
        self.player_stays && self.play.is_some()
    }

    /// The playing scene's time, seconds; `None` when no scene plays.
    pub fn scene_time(&self) -> Option<f32> {
        self.play.as_ref().map(|playing| playing.run.scene_time())
    }

    /// Leaves the player's car where it was when the scene that plays or loads ends: the scene's last pose of it is not
    /// handed back (a race's intro, whose car never left its grid slot). Cleared by the next [`NisPlayer::play`].
    pub fn keep_player_in_place(&mut self) {
        self.player_stays = true;
    }
}

/// The linked cars handed back at a scene's end: all of them, or without the player's car when it stays in place.
pub(super) fn handed_back(spots: Vec<(Target, CarSpot)>, player_stays: bool) -> Vec<(Target, CarSpot)> {
    match player_stays {
        true => spots.into_iter().filter(|&(target, _)| target != Target::Player).collect(),
        false => spots,
    }
}
