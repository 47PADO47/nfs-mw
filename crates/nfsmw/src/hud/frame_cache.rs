//! What the HUD plugin keeps from one frame to the next, so a frame in which nothing changed builds nothing.
//!
//! The runtime counts the changes to a package (`Runtime::revision`). A tree in the frame cell was built from a package
//! at one revision and one placement (the viewport scale and shift), so while those stay the same the tree in the cell
//! is still the right one and the plugin neither rebuilds nor stores it. The cell then keeps its generation, and the
//! layers composed from it are reused too (Original theme only: its slots read nothing else; the other themes draw
//! from live feeds and the clock, so their layers are composed every frame).

use blackbox_feng::PackageId;
use blackbox_ui::UiTree;

use crate::ui::theme::original::{FengSource, FrameCell};

/// What a tree in the frame cell was built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TreeKey {
    package: PackageId,
    revision: u64,
    /// The bits of the viewport scale and shift (`viewport::parameters`).
    scale: u32,
    shift: u32,
}

impl TreeKey {
    pub fn new(package: PackageId, revision: u64, (scale, shift): (f32, f32)) -> Self {
        Self { package, revision, scale: scale.to_bits(), shift: shift.to_bits() }
    }
}

/// The three composed layers of a frame: the radio layer, the HUD, the busted overlay.
struct Composed {
    generation: u64,
    reusable: bool,
    trees: [UiTree; 3],
}

#[derive(Default)]
pub(super) struct FrameCache {
    hud: Option<TreeKey>,
    chyron: Option<TreeKey>,
    overlay: Option<TreeKey>,
    layers: Option<Composed>,
}

impl FrameCache {
    fn built(&mut self, source: FengSource) -> &mut Option<TreeKey> {
        match source {
            FengSource::Hud => &mut self.hud,
            FengSource::Chyron => &mut self.chyron,
            FengSource::BustedOverlay => &mut self.overlay,
        }
    }

    /// Makes the cell tree for `source` the one for `key`: `build` rewrites it in place when the key is not the one it
    /// was built for, and is not called when it is. Returns whether it was built.
    pub fn refresh(
        &mut self,
        frame: &FrameCell,
        source: FengSource,
        key: TreeKey,
        build: impl FnOnce(&mut UiTree),
    ) -> bool {
        let built = self.built(source);
        if *built == Some(key) {
            return false;
        }
        frame.fill(source, build);
        *built = Some(key);
        true
    }

    /// Empties `source` in the cell (a package that is not drawn now). The next `refresh` builds it again.
    pub fn clear(&mut self, frame: &FrameCell, source: FengSource) {
        *self.built(source) = None;
        frame.clear(source);
    }

    /// The composed layers for the cell at `generation`. `compose` runs unless the layers kept from an earlier frame
    /// were composed at the same generation and both frames are `reusable` (the slots read nothing but the cell).
    pub fn layers(&mut self, generation: u64, reusable: bool, compose: impl FnOnce() -> [UiTree; 3]) -> &[UiTree; 3] {
        let keep = self.layers.as_ref().is_some_and(|c| reusable && c.reusable && c.generation == generation);
        if !keep {
            self.layers = Some(Composed { generation, reusable, trees: compose() });
        }
        &self.layers.as_ref().expect("the layers were just composed").trees
    }
}

#[cfg(test)]
mod tests {
    use blackbox_feng::package::{ObjectData, ObjectDef, ObjectKind, Package, StringDef};
    use blackbox_feng::{Runtime, fe_hash_upper};

    use super::*;

    fn key(revision: u64) -> TreeKey {
        TreeKey::new(PackageId(1), revision, (1.0, 0.0))
    }

    fn tree_of(count: usize) -> UiTree {
        UiTree { nodes: vec![Default::default(); count], draw_order: Vec::new() }
    }

    #[test]
    fn an_unchanged_key_builds_nothing_and_leaves_the_cell_alone() {
        let (mut cache, frame) = (FrameCache::default(), FrameCell::new());
        let mut builds = 0;
        let mut first = |cache: &mut FrameCache, k| {
            cache.refresh(&frame, FengSource::Hud, k, |tree| {
                builds += 1;
                *tree = tree_of(2);
            })
        };
        assert!(first(&mut cache, key(1)), "the first frame builds");
        assert!(!first(&mut cache, key(1)), "the same revision does not");
        assert!(first(&mut cache, key(2)), "a new revision does");
        assert_eq!(builds, 2);
        let settled = frame.generation();
        let skipped = cache.refresh(&frame, FengSource::Hud, key(2), |_| unreachable!("the tree is current"));
        assert!(!skipped);
        assert_eq!(frame.generation(), settled, "a skipped frame leaves the generation");
        assert_eq!(frame.get(FengSource::Hud).map(|t| t.nodes.len()), Some(2));
    }

    #[test]
    fn another_placement_or_package_builds_again() {
        let (mut cache, frame) = (FrameCache::default(), FrameCell::new());
        assert!(cache.refresh(&frame, FengSource::Hud, key(1), |_| {}));
        let wide = TreeKey::new(PackageId(1), 1, (1.0, 120.0));
        assert!(cache.refresh(&frame, FengSource::Hud, wide, |_| {}), "the viewport moved");
        let drag = TreeKey::new(PackageId(2), 1, (1.0, 120.0));
        assert!(cache.refresh(&frame, FengSource::Hud, drag, |_| {}), "the drag package took over");
    }

    #[test]
    fn a_cleared_source_is_built_again() {
        let (mut cache, frame) = (FrameCache::default(), FrameCell::new());
        assert!(cache.refresh(&frame, FengSource::Chyron, key(1), |_| {}));
        cache.clear(&frame, FengSource::Chyron);
        assert!(frame.get(FengSource::Chyron).is_none());
        assert!(cache.refresh(&frame, FengSource::Chyron, key(1), |_| {}), "the cell was emptied, so it is rebuilt");
    }

    #[test]
    fn clearing_an_empty_source_does_not_move_the_generation() {
        let (mut cache, frame) = (FrameCache::default(), FrameCell::new());
        let before = frame.generation();
        cache.clear(&frame, FengSource::BustedOverlay);
        assert_eq!(frame.generation(), before);
    }

    #[test]
    fn layers_are_reused_only_at_the_same_generation_and_only_when_reusable() {
        let mut cache = FrameCache::default();
        let composed = std::cell::Cell::new(0);
        let layers = |cache: &mut FrameCache, generation, reusable| {
            cache
                .layers(generation, reusable, || {
                    composed.set(composed.get() + 1);
                    [tree_of(1), tree_of(2), tree_of(3)]
                })
                .len()
        };
        layers(&mut cache, 5, true);
        layers(&mut cache, 5, true);
        assert_eq!(composed.get(), 1, "the same generation reuses the layers");
        layers(&mut cache, 6, true);
        assert_eq!(composed.get(), 2, "a new generation composes again");
        layers(&mut cache, 6, false);
        assert_eq!(composed.get(), 3, "a theme that draws from live feeds never reuses");
        layers(&mut cache, 6, true);
        assert_eq!(composed.get(), 4, "layers composed for such a theme are not reused when Original comes back");
    }

    fn text_package() -> Package {
        let mut words = vec![0u32; 17];
        for word in words.iter_mut().take(4) {
            *word = 255;
        }
        words[13] = 1.0f32.to_bits();
        let object = ObjectDef {
            kind: ObjectKind::String,
            guid: 1,
            name_hash: fe_hash_upper("Digits"),
            flags: 0,
            resource: None,
            parent: None,
            data: ObjectData { words },
            string: Some(StringDef { text: "0".into(), ..Default::default() }),
            multi: None,
            scripts: Vec::new(),
            responses: Vec::new(),
        };
        Package {
            name: "T.fng".into(),
            file_name: "T.fng".into(),
            version: 0,
            resources: Vec::new(),
            objects: vec![object],
            button_count: 0,
            responses: Vec::new(),
            targets: Vec::new(),
        }
    }

    /// The plugin frame: bind the same values, update, and rebuild only when the revision moved.
    #[test]
    fn a_package_that_binds_the_same_values_is_not_rebuilt() {
        let mut runtime = Runtime::new();
        let package = runtime.load(text_package());
        let object = runtime.find(package, fe_hash_upper("Digits")).unwrap();
        let (mut cache, frame) = (FrameCache::default(), FrameCell::new());
        let mut builds = 0;
        let mut frame_with = |runtime: &mut Runtime, text: &str| {
            runtime.set_text_str(object, text);
            runtime.update(1.0 / 60.0);
            let key = TreeKey::new(package, runtime.revision(package), (1.0, 0.0));
            let rebuilt = cache.refresh(&frame, FengSource::Hud, key, |tree| {
                builds += 1;
                runtime.tree_into(package, tree);
            });
            (rebuilt, frame.get(FengSource::Hud).and_then(|t| t.nodes[0].text.clone()))
        };
        assert_eq!(frame_with(&mut runtime, "10"), (true, Some("10".into())));
        assert_eq!(frame_with(&mut runtime, "10"), (false, Some("10".into())), "nothing changed");
        assert_eq!(frame_with(&mut runtime, "10"), (false, Some("10".into())));
        assert_eq!(frame_with(&mut runtime, "11"), (true, Some("11".into())), "a digit changed");
        assert_eq!(builds, 2);
    }
}
