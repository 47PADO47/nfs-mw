//! The grid a race's intro NIS leaves: a standing start whose intro plays a NIS at the race start puts each racer where
//! the scene's channel of its grid slot ends (`docs/specs/ai-racers-route.md` section 3). Racers without a channel keep
//! the planned grid.

use nfsmw_career::story::{IntroContext, IntroStep, intro_steps};
use nfsmw_data::gameplay::Race;

use super::RacerPlan;
use super::intro::story_fields;
use crate::scenes::world::road::Spawn;

/// The intro NIS whose end poses stand the grid of `race`: the scene its intro plays at the start (`intro.rs`), for a
/// standing start. A rolling start, a boss race and the NIS shell keep the planned grid: their intros' end poses are not
/// a grid in the install's data.
pub(super) fn grid_scene(race: &Race, career: bool) -> Option<String> {
    let story = &race.story;
    if race.params.rolling_start || story.boss_race || story.nis_shell {
        return None;
    }
    let ctx = IntroContext { career, boss_race: false, dday: story.dday, split_screen: false };
    intro_steps(&story_fields(story), ctx).into_iter().find_map(|step| match step {
        IntroStep::Nis { scene, .. } => Some(scene),
        _ => None,
    })
}

/// Stands each racer on the end pose of the channel of its physical grid slot (`car1` is slot 0). Returns how many
/// racers it moved.
pub(super) fn stand_on(plan: &mut [RacerPlan], ends: &[(usize, Spawn)]) -> usize {
    let mut moved = 0;
    for racer in plan.iter_mut() {
        let Some((_, spawn)) = ends.iter().find(|(slot, _)| *slot == racer.grid_slot) else { continue };
        racer.spawn = *spawn;
        moved += 1;
    }
    moved
}

impl crate::scenes::world::WorldScene {
    /// Seats the race cars on the authored intro handoff, when the scene provides one.
    pub(super) fn stand_on_intro_grid(
        &self,
        race: &Race,
        start: &nfsmw_data::gameplay::Placement,
        career: bool,
        plan: &mut [RacerPlan],
    ) {
        if self.stand_on_opening_roll(race, start, career, plan) {
            return;
        }
        let Some(scene) = grid_scene(race, career) else { return };
        let ends = match crate::scenes::world::nis::intro_grid(&self.dir, &scene, super::start::start_spawn(start)) {
            Ok(ends) => ends,
            Err(error) => {
                log::warn!("race: the grid of {scene} could not be read, so the racers keep the planned grid: {error}");
                return;
            }
        };
        let moved = stand_on(plan, &ends);
        log::info!("race: {moved} of {} racer(s) stand where {scene} leaves its cars", plan.len());
    }

    /// Uses the opening D-Day scene's rolling handoff for its two authored race cars.
    fn stand_on_opening_roll(
        &self,
        race: &Race,
        start: &nfsmw_data::gameplay::Placement,
        career: bool,
        plan: &mut [RacerPlan],
    ) -> bool {
        if !career || !race.params.rolling_start || crate::career::race_event_id(race) != "16.1.0" || plan.len() != 2 {
            return false;
        }
        let Some(scene) = race.story.intro_nis.as_deref().filter(|s| s.eq_ignore_ascii_case("IntroNisDD")) else {
            return false;
        };
        let grid =
            match crate::scenes::world::nis::rolling_intro_grid(&self.dir, scene, super::start::start_spawn(start)) {
                Ok(grid) => grid,
                Err(error) => {
                    log::warn!("race: no rolling handoff for {scene}: {error}");
                    return false;
                }
            };
        stand_on_rolling(plan, &grid)
    }
}

fn stand_on_rolling(plan: &mut [RacerPlan], grid: &[(usize, Spawn, f32)]) -> bool {
    if !plan.iter().all(|racer| grid.iter().any(|&(slot, _, _)| slot == racer.grid_slot)) {
        return false;
    }
    for racer in plan {
        if let Some(&(_, spawn, speed)) = grid.iter().find(|&&(slot, _, _)| slot == racer.grid_slot) {
            racer.spawn = spawn;
            racer.rolling_speed = Some(speed);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use glam::Vec3;
    use nfsmw_data::gameplay::RaceStory;

    use super::*;

    fn racer(grid_slot: usize) -> RacerPlan {
        RacerPlan {
            name: format!("racer {grid_slot}"),
            name_label: None,
            model: None,
            spawn: Spawn { position: Vec3::new(grid_slot as f32, 0.0, 0.0), heading: 0.0 },
            rolling_speed: None,
            grid_slot,
            skill: 1.0,
            bias: 1.0,
            tried: Vec::new(),
            person: None,
        }
    }

    fn race(story: RaceStory) -> Race {
        Race {
            key: 1,
            name: "test".into(),
            event_icon: "circuit".into(),
            race_type: None,
            event: None,
            region: None,
            laps: 1,
            start: None,
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: Default::default(),
            story,
        }
    }

    #[test]
    fn each_slot_takes_its_channels_end_and_the_rest_keep_the_grid() {
        // The player stands in slot 2 (a custom grid), so it takes car3's end; slot 4 has no channel.
        let mut plan = vec![racer(2), racer(1), racer(0), racer(3), racer(4)];
        let end = |x: f32| Spawn { position: Vec3::new(x, 5.0, 1.0), heading: 0.5 };
        let ends = [(0, end(10.0)), (1, end(11.0)), (2, end(12.0)), (3, end(13.0))];
        assert_eq!(stand_on(&mut plan, &ends), 4);
        let xs: Vec<f32> = plan.iter().map(|r| r.spawn.position.x).collect();
        assert_eq!(xs, [12.0, 11.0, 10.0, 13.0, 4.0]);
        assert!(plan[..4].iter().all(|r| r.spawn.heading == 0.5 && r.spawn.position.y == 5.0));
    }

    #[test]
    fn the_opening_roll_keeps_pose_and_speed_with_the_channel_slot() {
        let grid = [(0, racer(0).spawn, 31.0), (1, racer(1).spawn, 34.0)];
        let mut plan = vec![racer(1), racer(0)];
        assert!(!stand_on_rolling(&mut plan, &grid[..1]));
        assert!(plan.iter().all(|racer| racer.rolling_speed.is_none()));
        assert!(stand_on_rolling(&mut plan, &grid));
        assert_eq!(plan[0].rolling_speed, Some(34.0));
        assert_eq!(plan[1].rolling_speed, Some(31.0));
        assert_eq!(plan[0].spawn.position.x, 1.0);
        assert_eq!(plan[1].spawn.position.x, 0.0);
    }

    #[test]
    fn the_scene_is_the_intro_nis_of_a_standing_start() {
        let story = RaceStory { intro_nis: Some("IntroNis10".into()), ..Default::default() };
        assert_eq!(grid_scene(&race(story.clone()), false).as_deref(), Some("IntroNis10"));
        assert_eq!(grid_scene(&race(story.clone()), true).as_deref(), Some("IntroNis10"));
        let mut rolling = race(story.clone());
        rolling.params.rolling_start = true;
        assert_eq!(grid_scene(&rolling, false), None);
        let boss = RaceStory { boss_race: true, ..story.clone() };
        assert_eq!(grid_scene(&race(boss), true), None);
        let shell = RaceStory { nis_shell: true, ..story };
        assert_eq!(grid_scene(&race(shell), true), None);
        assert_eq!(grid_scene(&race(RaceStory::default()), false), None);
    }
}
