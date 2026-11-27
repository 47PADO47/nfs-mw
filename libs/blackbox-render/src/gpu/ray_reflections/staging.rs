//! Spreading the hit-table work of a changed scene over frames.
//!
//! A mesh joins the scene by copying its hit data, resampling the textures it needs into the atlases and building
//! its BLAS. When streaming brings in a section, or the camera's travel re-ranks the detail cache, that work used
//! to land in one frame. Now new meshes join nearest first until [`ADD_BUDGET`] is spent or [`LAYER_BUDGET`]
//! textures were resampled, and the detail cache swaps [`DETAIL_SWAPS`] textures a frame. A resample is cheap to
//! record but costs one render pass per mip when its encoder is finished, which the frame waits for.
//!
//! A mesh that waits is left out of the TLAS for those frames without marking the scene incomplete, which would
//! hand every sun-shadow pixel back to the cascades. Meshes that were in the scene already (re-added after a
//! detail swap) are never held back, so nothing that was drawn disappears.
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// The time one frame spends adding meshes that are new to the scene.
pub(super) const ADD_BUDGET: Duration = Duration::from_millis(2);
/// Atlas layers resampled per frame.
pub(super) const LAYER_BUDGET: usize = 12;
/// Detail-cache textures swapped in or out per frame.
pub(super) const DETAIL_SWAPS: usize = 4;

/// Up to `limit` handles whose detail-cache membership moves `applied` towards `target`: textures leaving first,
/// as they free layers for the ones joining, each group in handle order.
pub(super) fn detail_step(applied: &HashSet<usize>, target: &HashSet<usize>, limit: usize) -> Vec<usize> {
    let mut leaving: Vec<_> = applied.difference(target).copied().collect();
    leaving.sort_unstable();
    let mut joining: Vec<_> = target.difference(applied).copied().collect();
    joining.sort_unstable();
    leaving.into_iter().chain(joining).take(limit).collect()
}

/// `new` (meshes not in the scene) nearest first, by the closest of their instances in `distances`; ties and
/// meshes without a distance in id order.
pub(super) fn nearest_first(new: &mut [usize], distances: &HashMap<usize, f32>) {
    new.sort_unstable_by(|a, b| {
        let da = distances.get(a).copied().unwrap_or(f32::INFINITY);
        let db = distances.get(b).copied().unwrap_or(f32::INFINITY);
        da.total_cmp(&db).then(a.cmp(b))
    });
}

/// Which new meshes join the scene this frame.
pub(super) struct AddBudget {
    started: Instant,
    limit: Duration,
    layers: usize,
    added: usize,
}

impl AddBudget {
    pub fn new(limit: Duration, layers: usize) -> Self {
        Self { started: Instant::now(), limit, layers, added: 0 }
    }

    /// Whether one more new mesh may join now, `resampled` atlas layers into the frame. The first always may, so a
    /// busy frame still makes progress.
    pub fn admit(&mut self, resampled: usize) -> bool {
        if self.added > 0 && (self.started.elapsed() >= self.limit || resampled >= self.layers) {
            return false;
        }
        self.added += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_swaps_free_layers_before_filling_them_and_stop_at_the_limit() {
        let applied = HashSet::from([1, 2, 3]);
        let target = HashSet::from([3, 4, 5, 6]);
        assert_eq!(detail_step(&applied, &target, 3), vec![1, 2, 4]);
        assert_eq!(detail_step(&applied, &target, 99), vec![1, 2, 4, 5, 6]);
        assert!(detail_step(&target, &target, 8).is_empty(), "a settled cache swaps nothing");
    }

    #[test]
    fn the_nearest_new_mesh_joins_first() {
        let mut new = vec![7, 3, 9, 1];
        let distances = HashMap::from([(7, 5.0), (3, 500.0), (9, 5.0)]);
        nearest_first(&mut new, &distances);
        assert_eq!(new, vec![7, 9, 3, 1]);
    }

    #[test]
    fn the_first_mesh_always_joins_and_the_budget_stops_the_rest() {
        let mut budget = AddBudget::new(Duration::ZERO, 99);
        assert!(budget.admit(0), "progress even when the frame is already over budget");
        assert!(!budget.admit(0));
        let mut roomy = AddBudget::new(Duration::from_secs(60), 12);
        assert!((0..100).all(|_| roomy.admit(11)));
        assert!(!roomy.admit(12), "the resampled layers stop it as well");
    }
}
