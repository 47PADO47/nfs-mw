//! A value derived from the resident scenery, rebuilt only when the scenery changes.
use crate::scenes::world::scenery_groups::SceneryGroups;

/// What a cached value was built for.
struct Stamp {
    /// The residency's `visual_revision`: streaming and zone changes bump it.
    revision: u64,
    /// The switched-on scenery groups. Switching one does not bump the revision, but it changes what is placed.
    groups: SceneryGroups,
    /// A setting the value also depends on, as bits.
    extra: u32,
}

/// `value`, and the scenery state it was built for.
#[derive(Default)]
pub(super) struct Cached<T> {
    stamp: Option<Stamp>,
    pub value: T,
}

impl<T> Cached<T> {
    fn is_current(&self, revision: u64, groups: &SceneryGroups, extra: u32) -> bool {
        let Some(stamp) = &self.stamp else { return false };
        stamp.revision == revision && stamp.extra == extra && stamp.groups == *groups
    }

    /// Runs `build` on the value unless it was already built for this scenery state. Returns whether it ran.
    pub fn refresh(&mut self, revision: u64, groups: &SceneryGroups, extra: u32, build: impl FnOnce(&mut T)) -> bool {
        if self.is_current(revision, groups, extra) {
            return false;
        }
        build(&mut self.value);
        self.stamp = Some(Stamp { revision, groups: groups.clone(), extra });
        true
    }

    /// Forces the next [`refresh`](Self::refresh) to build.
    pub fn invalidate(&mut self) {
        self.stamp = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unchanged_scenery_state_does_not_rebuild() {
        let groups = SceneryGroups::default();
        let mut cache = Cached::<Vec<u32>>::default();
        let mut builds = 0;
        for _ in 0..3 {
            cache.refresh(7, &groups, 0, |v| {
                builds += 1;
                v.push(1);
            });
        }
        assert_eq!(builds, 1);
        assert_eq!(cache.value, vec![1]);
    }

    #[test]
    fn a_changed_revision_group_setting_or_invalidation_rebuilds() {
        let mut groups = SceneryGroups::default();
        let mut cache = Cached::<u32>::default();
        let build = |cache: &mut Cached<u32>, revision, groups: &SceneryGroups, extra| {
            cache.refresh(revision, groups, extra, |v| *v += 1)
        };
        assert!(build(&mut cache, 1, &groups, 0));
        assert!(!build(&mut cache, 1, &groups, 0));
        assert!(build(&mut cache, 2, &groups, 0), "a new revision");
        groups.set(5, true);
        assert!(build(&mut cache, 2, &groups, 0), "a switched scenery group");
        assert!(!build(&mut cache, 2, &groups, 0));
        assert!(build(&mut cache, 2, &groups, 9), "a changed setting");
        cache.invalidate();
        assert!(build(&mut cache, 2, &groups, 9));
        assert_eq!(cache.value, 5);
    }
}
