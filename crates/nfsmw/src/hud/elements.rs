//! Which parts of the HUD package are shown. The package holds every element of the in-game HUD; the game turns
//! each on or off by feature. Here an element is on when the state it shows exists in this build: the speedometer
//! and the tachometer always, the nitrous and turbo gauges when the car has the hardware. The rest of the package
//! (radar, pursuit bars, race timers) needs state that arrives with later milestones and stays hidden; the minimap
//! shows when the scene has a map position for it.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::minimap::{ARROW, BACKING, GROUP};

/// The speedometer group and the group of the tachometer (needle, face, gear, shift light, redline).
const SPEEDOMETER: [u32; 1] = [0x941f_ff09];
const TACHOMETER: [u32; 1] = [0x5164_d4ea];
/// The nitrous gauge: its group (frame and icon) and the bar, a multi image next to the group.
const NITROUS: [u32; 2] = [0x87c3_8e97, 0xedfb_6d37];

/// The elements and where they sit in the package.
pub struct Layout {
    package: PackageId,
    /// The parent of each object, as an index.
    parent: Vec<Option<usize>>,
    speedometer: Vec<usize>,
    tachometer: Vec<usize>,
    nitrous: Vec<usize>,
    turbo: Vec<usize>,
    /// The group of the minimap's pieces, its backing disc and the player's arrow.
    minimap: Vec<usize>,
    /// What the hidden flags were last set for: (nitrous, turbo, minimap).
    applied: Option<(bool, bool, bool)>,
}

impl Layout {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let objects = rt.package(package).map(|p| p.objects.clone()).unwrap_or_default();
        let index_of: std::collections::HashMap<u32, usize> =
            objects.iter().enumerate().map(|(i, o)| (o.guid, i)).collect();
        let parent = objects
            .iter()
            .enumerate()
            .map(|(i, o)| o.parent.and_then(|g| index_of.get(&g).copied()).filter(|p| *p != i))
            .collect();
        let find = |hashes: &[u32]| -> Vec<usize> {
            hashes.iter().filter_map(|h| rt.find(package, *h)).map(|o| o.index).collect()
        };
        Self {
            package,
            parent,
            speedometer: find(&SPEEDOMETER),
            tachometer: find(&TACHOMETER),
            nitrous: find(&NITROUS),
            turbo: find(&[fe_hash_upper("TURBO_GROUP")]),
            minimap: find(&[fe_hash_upper(GROUP), fe_hash_upper(BACKING), fe_hash_upper(ARROW)]),
            applied: None,
        }
    }

    /// Shows the speedometer, the tachometer and the gauges asked for, and hides everything else in the package.
    /// Nothing happens while the request is the one already applied.
    pub fn update(&mut self, rt: &mut Runtime, nitrous: bool, turbo: bool, minimap: bool) {
        if self.applied == Some((nitrous, turbo, minimap)) {
            return;
        }
        self.applied = Some((nitrous, turbo, minimap));
        let mut shown = vec![false; self.parent.len()];
        let mut roots: Vec<usize> = self.speedometer.iter().chain(&self.tachometer).copied().collect();
        if nitrous {
            roots.extend(&self.nitrous);
        }
        if turbo {
            roots.extend(&self.turbo);
        }
        if minimap {
            roots.extend(&self.minimap);
        }
        for root in roots {
            // The root's ancestors have to show for it to show; everything under it shows with it.
            let mut up = Some(root);
            for _ in 0..self.parent.len() {
                let Some(i) = up else { break };
                shown[i] = true;
                up = self.parent[i];
            }
            for (i, slot) in shown.iter_mut().enumerate() {
                *slot |= self.is_under(i, root);
            }
        }
        for (index, shown) in shown.into_iter().enumerate() {
            rt.set_hidden(ObjectRef { package: self.package, index }, !shown);
        }
    }

    /// `i` is `root` or lies below it.
    fn is_under(&self, mut i: usize, root: usize) -> bool {
        for _ in 0..self.parent.len() {
            if i == root {
                return true;
            }
            let Some(p) = self.parent[i] else { return false };
            i = p;
        }
        false
    }
}
