//! Grouping a frame's instances by mesh without moving 80 byte records around while sorting.

use std::cell::RefCell;

use blackbox_gfx::Instance;

thread_local! {
    /// `(mesh, place)` pairs of the sort in progress, kept between frames so a frame allocates nothing.
    static ORDER: RefCell<Vec<(usize, u32)>> = const { RefCell::new(Vec::new()) };
}

/// Sorts `instances` by mesh: every copy of a mesh ends up adjacent, so the renderer draws each run as one instanced
/// draw. Call it after the last instance of a frame is appended. The copies of one mesh keep their order.
///
/// It sorts 16 byte `(mesh, place)` pairs and then moves each instance once, instead of moving whole instances through
/// the sort. A list that is already grouped is left alone.
pub fn sort_by_mesh(instances: &mut [Instance]) {
    if instances.windows(2).all(|pair| pair[0].mesh <= pair[1].mesh) {
        return;
    }
    assert!(u32::try_from(instances.len()).is_ok(), "more instances than a frame can index");
    ORDER.with_borrow_mut(|order| {
        order.clear();
        order.extend(instances.iter().enumerate().map(|(place, i)| (i.mesh.raw(), place as u32)));
        // The place breaks ties, so equal meshes keep their order.
        order.sort_unstable();
        apply_order(instances, order);
    });
}

/// Rearranges `instances` so that position `i` holds the instance that was at `order[i].1`. `order` is spent: it ends
/// as the identity.
fn apply_order(instances: &mut [Instance], order: &mut [(usize, u32)]) {
    for start in 0..order.len() {
        let mut at = start;
        loop {
            let from = order[at].1 as usize;
            order[at].1 = at as u32;
            if from == start {
                break;
            }
            instances.swap(at, from);
            at = from;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{InstanceKey, MeshHandle};
    use glam::Mat4;

    fn list(meshes: &[usize]) -> Vec<Instance> {
        meshes
            .iter()
            .enumerate()
            .map(|(place, &mesh)| {
                Instance::keyed(MeshHandle::from_raw(mesh), Mat4::IDENTITY, InstanceKey::new(1, place as u32))
            })
            .collect()
    }

    fn places(list: &[Instance]) -> Vec<(usize, u32)> {
        list.iter().map(|i| (i.mesh.raw(), i.key.parts().1)).collect()
    }

    #[test]
    fn sorting_pairs_matches_a_stable_sort_of_the_instances() {
        let mut seed = 12345u32;
        for len in [0usize, 1, 2, 3, 17, 500, 4000] {
            let meshes: Vec<usize> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 16) as usize % 23
                })
                .collect();
            let mut actual = list(&meshes);
            sort_by_mesh(&mut actual);
            let mut expected = list(&meshes);
            expected.sort_by_key(|i| i.mesh);
            assert_eq!(places(&actual), places(&expected), "{len} instances");
        }
    }

    #[test]
    fn a_list_that_is_already_grouped_is_untouched() {
        let mut grouped = list(&[1, 1, 2, 5, 5, 5]);
        let before = places(&grouped);
        sort_by_mesh(&mut grouped);
        assert_eq!(places(&grouped), before);
    }
}
