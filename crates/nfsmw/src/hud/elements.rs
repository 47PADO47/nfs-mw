//! Which parts of the HUD package are shown. The package holds every element of the in-game HUD; the game turns
//! each on or off by feature. Here an element is on when the state it shows exists in this build: the speedometer
//! and the tachometer always, the nitrous and turbo gauges when the car has the hardware, the pursuit board and its
//! cool-down bar during a chase. The race timers and the heat meter stay hidden until their state is bound; the
//! minimap shows when the scene has a map position for it.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::minimap::{ARROW, BACKING, GROUP};

/// The speedometer group and the group of the tachometer (needle, face, gear, shift light, redline).
const SPEEDOMETER: [u32; 1] = [0x941f_ff09];
const TACHOMETER: [u32; 1] = [0x5164_d4ea];
/// The nitrous gauge: its group (frame and icon) and the bar, a multi image next to the group.
const NITROUS: [u32; 2] = [0x87c3_8e97, 0xedfb_6d37];
/// The rear-view mirror's group (its two bezel images and the picture drawn under them), and the radar sensor under
/// it: the backing and the two LED strips. The sensor shows its idle state; it does not detect cops yet.
const MIRROR: [u32; 1] = [0x9ee0_6631];
/// The radar's base is its dark backing; the green LED strips are not shown (the reference has only the dark grille).
const RADAR: [u32; 1] = [0x839e_7d77];
/// The radar's green laser (`Radar_Icon.tga`): it shows over the base radar while a cop is nearby.
const LASER: [u32; 1] = [0x6ccb_bbf1];
/// The pursuit board (`PursuitBoardGroup`): the backing, the timer and the bounty row. Shown while a chase runs.
const PURSUIT_BOARD: [u32; 1] = [0xde89_e070];
/// The cops group (`0xe0b1430b`): the cops icon with its count, and the taken-out icon with the wrecked count. It shows
/// with the board, as the cops it counts are the chase's.
const PURSUIT_COPS: [u32; 1] = [0xe0b1_430b];
/// The cool-down bar (`Cooldown_Meter_Group`): its track and fill. It sits inside the radar's group in the package,
/// so [`Layout::update`] decides it on its own flag. Shown once the player has started to get away.
const COOLDOWN: [u32; 1] = [0x84a2_26ec];
/// Parts of the package with no state bound yet, held hidden whatever the feature: the heat meter (its arcs and
/// `x` label are not driven), the bounty row (no bounty in the pursuit feed), and the cool-down bar's "HIDING"
/// group and car icon (the hiding timer is not in the feed).
const HELD_HIDDEN: [u32; 4] = [0xc46a_80a9, 0x8674_e6d4, 0x60b4_5523, 0x3807_4901];

/// What the layout shows this frame, one flag per feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shown {
    pub nitrous: bool,
    pub turbo: bool,
    pub speedbreaker: bool,
    pub minimap: bool,
    pub laser: bool,
    pub mirror: bool,
    pub pursuit: bool,
    pub cooldown: bool,
}

/// The elements and where they sit in the package.
pub struct Layout {
    package: PackageId,
    /// The parent of each object, as an index.
    parent: Vec<Option<usize>>,
    speedometer: Vec<usize>,
    tachometer: Vec<usize>,
    speedbreaker: Vec<usize>,
    nitrous: Vec<usize>,
    turbo: Vec<usize>,
    /// The group of the minimap's pieces, its backing disc and the player's arrow.
    minimap: Vec<usize>,
    mirror: Vec<usize>,
    radar: Vec<usize>,
    /// The green laser over the radar, shown while a cop is nearby.
    laser: Vec<usize>,
    pursuit: Vec<usize>,
    pursuit_cops: Vec<usize>,
    cooldown: Vec<usize>,
    held: Vec<usize>,
    applied: Option<Shown>,
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
            speedbreaker: find(&[
                fe_hash_upper(super::speedbreaker::GROUP),
                fe_hash_upper(super::speedbreaker::ICON),
                fe_hash_upper(super::speedbreaker::MASK),
                super::speedbreaker::LINEAR_GROUP,
            ]),
            nitrous: find(&NITROUS),
            turbo: find(&[fe_hash_upper("TURBO_GROUP")]),
            minimap: find(&[fe_hash_upper(GROUP), fe_hash_upper(BACKING), fe_hash_upper(ARROW)]),
            mirror: find(&MIRROR),
            radar: find(&RADAR),
            laser: find(&LASER),
            pursuit: find(&PURSUIT_BOARD),
            pursuit_cops: find(&PURSUIT_COPS),
            cooldown: find(&COOLDOWN),
            held: find(&HELD_HIDDEN),
            applied: None,
        }
    }

    /// Shows the speedometer, the tachometer and the features asked for, and hides everything else in the package.
    /// Nothing happens while the request is the one already applied.
    pub fn update(&mut self, rt: &mut Runtime, want: Shown) {
        if self.applied == Some(want) {
            return;
        }
        self.applied = Some(want);
        let mut shown = vec![false; self.parent.len()];
        let mut roots: Vec<usize> = self.speedometer.iter().chain(&self.tachometer).copied().collect();
        roots.extend(&self.speedbreaker);
        if want.mirror {
            roots.extend(&self.mirror);
            roots.extend(&self.radar);
        }
        if want.nitrous {
            roots.extend(&self.nitrous);
        }
        if want.turbo {
            roots.extend(&self.turbo);
        }
        if want.speedbreaker {
            roots.extend(&self.speedbreaker);
        }
        if want.minimap {
            roots.extend(&self.minimap);
        }
        if want.laser {
            roots.extend(&self.laser);
        }
        if want.pursuit {
            roots.extend(&self.pursuit);
            roots.extend(&self.pursuit_cops);
        }
        if want.cooldown {
            roots.extend(&self.cooldown);
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
        // The cool-down bar sits under the radar's group, which shows it with the mirror. Its own flag decides it,
        // and the parts held hidden stay hidden under whatever shows around them.
        for (i, slot) in shown.iter_mut().enumerate() {
            if self.is_under_any(i, &self.cooldown) {
                *slot = want.cooldown;
            }
            if self.is_under_any(i, &self.held) {
                *slot = false;
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

    fn is_under_any(&self, i: usize, roots: &[usize]) -> bool {
        roots.iter().any(|root| self.is_under(i, *root))
    }
}
