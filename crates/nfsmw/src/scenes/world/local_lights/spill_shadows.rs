//! Admit nearby spill cones before fixtures, within the existing shadow pool.
use blackbox_gfx::LocalLight;
use glam::Vec3;

pub(super) fn prioritize(
    lights: &mut Vec<LocalLight>,
    tails: &mut Vec<LocalLight>,
    bars: &mut Vec<LocalLight>,
    camera: Vec3,
    distance: f32,
    count: u32,
) {
    let slots = (count as usize).saturating_sub(lights.len()).min(2);
    for slot in 0..slots {
        // First slot goes to the nearest active bar, second to a rear lamp if available.
        let (first, second) = match slot {
            0 => (&mut *bars, &mut *tails),
            _ => (&mut *tails, &mut *bars),
        };
        let within_range =
            |light: &LocalLight| Vec3::from_array(light.position).distance_squared(camera) < distance * distance;
        let take = |source: &mut Vec<LocalLight>| {
            let index = source.iter().position(within_range)?;
            Some(source.remove(index))
        };
        let Some(light) = take(first).or_else(|| take(second)) else { break };
        lights.push(LocalLight { casts_shadow: true, ..light });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lamp(x: f32) -> LocalLight {
        LocalLight {
            position: [x, 0.0, 0.0],
            direction: [0.0, 0.0, -1.0],
            range: 14.0,
            outer_cos: 0.1,
            inner_cos: 0.55,
            color: [1.0; 3],
            upper_plane: None,
            casts_shadow: false,
            shadow_exclude: None,
        }
    }

    #[test]
    fn keeps_priority_beams_and_promotes_only_near_spill_within_budget() {
        let mut lights = vec![lamp(0.0)];
        let mut tails = vec![lamp(1.0), lamp(2.0)];
        let mut bars = vec![lamp(81.0), lamp(3.0)];
        prioritize(&mut lights, &mut tails, &mut bars, Vec3::ZERO, 80.0, 1);
        assert_eq!((lights.len(), tails.len(), bars.len()), (1, 2, 2));
        prioritize(&mut lights, &mut tails, &mut bars, Vec3::ZERO, 80.0, 8);
        assert_eq!((lights.len(), tails.len(), bars.len()), (3, 1, 1));
        assert_eq!(lights[1].position[0], 3.0);
        assert_eq!(lights[2].position[0], 1.0);
        assert!(lights[1..].iter().all(|light| light.casts_shadow));
        assert!(tails.iter().chain(&bars).all(|light| !light.casts_shadow));
    }
}
