//! The finish line's flares: which effect the finish trigger names and where it stands. The flares themselves are
//! `finish_flares.rs`. Spec: `docs/specs/race-finish-effects.md` section 1.

use glam::Vec3;
use nfsmw_data::gameplay::{BoxTrigger, GameplayData, Race};

use super::setup::gate_of;
use crate::scenes::world::{WorldScene, space};

/// The effect a finish trigger shows and the points it stands at (physics space), or `None` for a trigger with no
/// `ParticleEffect` (every checkpoint). `FlareSpacing` is the distance of each flare from the centre.
pub(super) fn flare_points(trigger: &BoxTrigger) -> Option<(String, Vec<Vec3>)> {
    let effect = trigger.particle_effect.as_deref().map(str::trim).filter(|name| !name.is_empty())?;
    let spacing = trigger.flare_spacing.unwrap_or(0.0);
    Some((effect.to_owned(), gate_of(trigger).flare_points(spacing)))
}

impl WorldScene {
    /// Lights the flares of `race`'s finish line, from the start of the race until it is taken down.
    pub(super) fn show_finish_flares(&mut self, data: &GameplayData, race: &Race) {
        let trigger = race.finish.and_then(|key| data.trigger(key));
        let Some((effect, points)) = trigger.and_then(flare_points) else {
            self.finish_flares.clear();
            return;
        };
        let render: Vec<Vec3> = points.into_iter().map(|p| space::to_render(p.to_array())).collect();
        log::info!("race: {} flare(s) of {effect} at the finish line", render.len());
        self.finish_flares.show(&effect, render);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(effect: Option<&str>, spacing: Option<f32>) -> BoxTrigger {
        BoxTrigger {
            key: 1,
            name: "finish".into(),
            position: Vec3::new(100.0, 5.0, 200.0),
            heading_deg: 0.0,
            dimensions: [3.0, 65.0, 5.0],
            one_shot: false,
            fire_on_exit: false,
            particle_effect: effect.map(str::to_owned),
            flare_spacing: spacing,
            time_bonus: 0.0,
            resets_player: false,
        }
    }

    #[test]
    fn a_finish_line_with_spacing_gets_a_flare_each_side_of_its_centre() {
        let (effect, points) = flare_points(&trigger(Some("fxgame_flare_red"), Some(5.0))).unwrap();
        assert_eq!(effect, "fxgame_flare_red");
        assert_eq!(points.len(), 2);
        // Heading 0 faces +z in physics space, so the width axis is x: 5 m to each side, at the trigger's height.
        let mut xs = [points[0].x, points[1].x];
        xs.sort_by(f32::total_cmp);
        assert!((xs[0] - 95.0).abs() < 1e-3 && (xs[1] - 105.0).abs() < 1e-3, "{points:?}");
        assert!(points.iter().all(|p| (p.z - 200.0).abs() < 1e-3 && (p.y - 5.0).abs() < 1e-3));
    }

    #[test]
    fn the_flares_follow_the_gates_heading() {
        let mut turned = trigger(Some("fxgame_flare_red"), Some(5.0));
        turned.heading_deg = 90.0;
        let (_, points) = flare_points(&turned).unwrap();
        // Turned a quarter, the width axis is z.
        assert!(points.iter().all(|p| (p.x - 100.0).abs() < 1e-3), "{points:?}");
        assert!(((points[0].z - points[1].z).abs() - 10.0).abs() < 1e-3);
    }

    #[test]
    fn no_spacing_puts_one_flare_at_the_centre_and_no_effect_none() {
        let (_, points) = flare_points(&trigger(Some("fxgame_flare_red"), None)).unwrap();
        assert_eq!(points, [Vec3::new(100.0, 5.0, 200.0)]);
        assert!(flare_points(&trigger(None, Some(5.0))).is_none(), "a checkpoint shows nothing");
        assert!(flare_points(&trigger(Some("  "), Some(5.0))).is_none());
    }
}
