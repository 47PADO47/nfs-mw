//! Releasing a placed model restores its placement, unless that placement stood on a structure that is still
//! destroyed. Spec: `docs/specs/prop-lifetime.md` ("Placements on destroyed structures").

use super::PropWorld;
use blackbox_scene::Aabb;

/// How far apart two placement boxes may be and still count as touching (metres).
const TOUCH: f32 = 0.25;
/// How far above the supporting model's base a placement must start to count as resting on it (metres). Props
/// that stand on the same ground beside the model start at about its base and are not affected.
const ABOVE_BASE: f32 = 0.5;
/// How far a supporting placement may have been pushed and still hold what rested on it (metres).
const STILL_IN_PLACE: f32 = 0.5;

impl PropWorld {
    /// Ends a placed model's simulation. Its placement is restored, except when the placement rested on a placed
    /// model that is hidden (a collapsed booth, roof or support): restoring it there would leave it hanging in
    /// the air, so it is retired with that structure and comes back when the section restores both.
    pub(super) fn release_placement(&mut self, id: u32) {
        if self.rests_on_destroyed_model(id) {
            self.hide_model(id);
            return;
        }
        let Some(collider) = self.colliders.get_mut(&id) else {
            return;
        };
        if let (Some(body), Some(spec)) = (&mut collider.body, &collider.shape.dynamics) {
            body.restore(spec);
        }
        self.reset_model(id);
    }

    /// Whether the original placement of `id` rested on placed models of which none is still there: hidden
    /// (collapsed) or knocked out of place, like a stop sign on a booth's lamp. A placement that also rests on
    /// one still in place (an arm reaching the next booth) keeps its support.
    fn rests_on_destroyed_model(&self, id: u32) -> bool {
        let Some(own) = self.colliders.get(&id).map(|c| c.original_bounds) else {
            return false;
        };
        let mut destroyed = false;
        for (&other, model) in &self.scenery.models {
            if other == id || !model.is_placed() {
                continue;
            }
            let Some(support) = self.colliders.get(&other).filter(|c| rests_on(&own, &c.original_bounds)) else {
                continue;
            };
            let displaced = support.body.as_ref().is_some_and(|b| b.displacement() > STILL_IN_PLACE);
            if !model.is_hidden() && !displaced {
                return false;
            }
            destroyed |= model.is_hidden();
        }
        destroyed
    }
}

/// Whether a placement with world box `own` rests on, or hangs from, the placement with box `support` (render
/// space, z up): the boxes touch and `own` starts above the support's base.
pub(super) fn rests_on(own: &Aabb, support: &Aabb) -> bool {
    if own.is_empty() || support.is_empty() || own.min.z < support.min.z + ABOVE_BASE {
        return false;
    }
    (0..3).all(|axis| own.min[axis] <= support.max[axis] + TOUCH && own.max[axis] >= support.min[axis] - TOUCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(min: [f32; 3], max: [f32; 3]) -> Aabb {
        Aabb::new(min, max)
    }

    #[test]
    fn a_lamp_on_a_booth_roof_rests_on_it_and_a_cone_beside_it_does_not() {
        let booth = aabb([0.0, 0.0, 0.0], [2.0, 2.0, 3.0]);
        assert!(rests_on(&aabb([0.5, 0.5, 3.0], [1.5, 1.5, 4.0]), &booth), "on the roof");
        assert!(rests_on(&aabb([2.1, 0.5, 2.0], [6.0, 1.0, 2.5]), &booth), "an arm hung from its side");
        assert!(!rests_on(&aabb([2.1, 0.5, 0.0], [2.6, 1.0, 0.8]), &booth), "a cone on the same ground");
        assert!(!rests_on(&aabb([3.0, 0.5, 3.0], [4.0, 1.5, 4.0]), &booth), "a lamp over the next lane");
        assert!(!rests_on(&Aabb::EMPTY, &booth));
        assert!(!rests_on(&booth, &Aabb::EMPTY));
    }
}
