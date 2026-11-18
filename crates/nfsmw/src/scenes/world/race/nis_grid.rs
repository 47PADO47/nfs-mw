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
