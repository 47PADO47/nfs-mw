//! Which parts of the HUD package are shown. The package holds every element of the in-game HUD; the game turns
//! each on or off by feature. Here an element is on when the state it shows exists in this build: the speedometer
//! and the tachometer always, the nitrous and turbo gauges when the car has the hardware, the pursuit board and its
//! cool-down bar during a chase, the bounty row in career mode. Race groups and heat arcs show their bound state;
//! the minimap shows when the scene has a map position for it.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::leaderboard::{self, Kind};
use super::leaders::ROWS;
use super::minimap::{ARROW, BACKING, GROUP};

/// The speedometer group and the group of the tachometer (needle, face, gear, shift light, redline).
pub(super) const SPEEDOMETER: [u32; 1] = [0x941f_ff09];
pub(super) const TACHOMETER: [u32; 1] = [0x5164_d4ea];
/// The drag tachometer of `HUD_Drag.fng` (the needle, the 10000 rpm lines, the gear digit). Not in the race HUD.
const DRAG_TACHOMETER: [u32; 1] = [0x157c_5b37];
/// The turbo gauge's group: its needle sits inside it. The name is the one the package uses.
pub(super) const TURBO_GROUP: &str = "TURBO_GROUP";
/// The nitrous gauge: its group (frame and icon) and the bar, a multi image next to the group.
pub(super) const NITROUS: [u32; 2] = [0x87c3_8e97, 0xedfb_6d37];
/// The rear-view mirror's backing, with the detector grille integrated under it.
const MIRROR: [u32; 1] = [0x9ee0_6631];
/// The standalone backing and detector objects. Authored scripts select the mirror/standalone poses.
const RADAR: [u32; 2] = [0x839e_7d77, 0x0627_43f5];
/// The radar's green laser (`Radar_Icon.tga`): it shows over the base radar while a cop is nearby.
const LASER: [u32; 1] = [0x6ccb_bbf1];
/// The pursuit board (`PursuitBoardGroup`): the backing, the timer and the bounty row. Shown while a chase runs.
pub(super) const PURSUIT_BOARD: [u32; 2] = [0xde89_e070, 0x7b42_2ba3];
/// The cops group (`0xe0b1430b`): the cops icon with its count, and the taken-out icon with the wrecked count. It shows
/// with the board, as the cops it counts are the chase's.
pub(super) const PURSUIT_COPS: [u32; 1] = [0xe0b1_430b];
/// The cool-down bar (`Cooldown_Meter_Group`): its track and fill. It sits inside the radar's group in the package,
/// so [`Layout::update`] decides it on its own flag. Shown once the player has started to get away.
pub(super) const COOLDOWN: [u32; 1] = [0x84a2_26ec];
/// The bounty row: the career feeds it, so it shows in career mode only and is held hidden elsewhere.
const BOUNTY_ROW: [u32; 1] = [0x8674_e6d4];
/// The cool-down bar's "HIDING" text group and its car icon. They sit under the cool-down group and rest at zero
/// alpha; `hiding_mark.rs` fades them in and out (`docs/specs/pursuit-hiding.md`). The orange bar beside them
/// (`0x5c2a4f20`) answers no message in the package and stays at rest.
pub(super) const HIDING: [u32; 2] = [0x60b4_5523, 0x3807_4901];

/// What the layout shows this frame, one flag per feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shown {
    pub radar: bool,
    pub heat: bool,
    pub nitrous: bool,
    pub turbo: bool,
    pub speedbreaker: bool,
    pub minimap: bool,
    pub laser: bool,
    pub mirror: bool,
    pub pursuit: bool,
    pub cooldown: bool,
    /// Career mode: the bounty row shows.
    pub career: bool,
    /// A race runs: the race information group (place, clock) shows.
    pub race: bool,
    /// A countdown step is up: the `321_GO_GROUP` shows.
    pub countdown: bool,
    /// A line of the generic message group is up.
    pub message: bool,
    /// The leaderboard's rows that show (0 hides the board): `LeaderBoardGroup` and its first rows (`hud/leaderboard.rs`).
    pub leaders: u8,
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
    heat: Vec<usize>,
    /// The green laser over the radar, shown while a cop is nearby.
    laser: Vec<usize>,
    pursuit: Vec<usize>,
    pursuit_cops: Vec<usize>,
    cooldown: Vec<usize>,
    /// The bounty row: shown in career mode only.
    bounty_row: Vec<usize>,
    race: Vec<usize>,
    race_lap: Vec<usize>,
    countdown: Vec<usize>,
    message: Vec<usize>,
    /// The leaderboard's group, the objects of each row, and the online headset icons (held hidden: no online game).
    leaders: Vec<usize>,
    leader_rows: Vec<Vec<usize>>,
    leader_icons: Vec<usize>,
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
            tachometer: find(&[TACHOMETER[0], DRAG_TACHOMETER[0]]),
            speedbreaker: find(&[
                fe_hash_upper(super::speedbreaker::GROUP),
                fe_hash_upper(super::speedbreaker::ICON),
                fe_hash_upper(super::speedbreaker::MASK),
                super::speedbreaker::LINEAR_GROUP,
            ]),
            nitrous: find(&NITROUS),
            turbo: find(&[fe_hash_upper(TURBO_GROUP)]),
            minimap: find(&[fe_hash_upper(GROUP), fe_hash_upper(BACKING), fe_hash_upper(ARROW)]),
            mirror: find(&MIRROR),
            radar: find(&RADAR),
            heat: find(&[0xC46A_80A9, 0xB0F6_3386]),
            laser: find(&LASER),
            pursuit: find(&PURSUIT_BOARD),
            pursuit_cops: find(&PURSUIT_COPS),
            cooldown: find(&COOLDOWN),
            bounty_row: find(&BOUNTY_ROW),
            race: find(&super::race::root_hashes()),
            race_lap: find(&[super::race::lap_hash()]),
            countdown: find(&[super::race::count_hash()]),
            message: find(&[super::race::message_hash()]),
            leaders: find(&[leaderboard::GROUP]),
            leader_rows: (0..ROWS)
                .map(|k| {
                    let row = [Kind::Data, Kind::Place, Kind::Backing].map(|kind| leaderboard::row_hashes(kind)[k]);
                    find(&row)
                })
                .collect(),
            leader_icons: find(&leaderboard::row_hashes(Kind::Icon)),
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
        }
        if want.radar {
            roots.extend(&self.radar);
        }
        if want.heat {
            roots.extend(&self.heat);
        }
        if want.career {
            roots.extend(&self.bounty_row);
        }
        if want.race {
            roots.extend(&self.race);
        }
        if want.countdown {
            roots.extend(&self.countdown);
        }
        if want.message {
            roots.extend(&self.message);
        }
        if want.leaders > 0 {
            roots.extend(&self.leaders);
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
            // Both meter roots stay eligible through their authored crossfade.
            roots.extend(&self.cooldown);
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
        // The parts without a bound feed stay hidden under whatever shows around them.
        for (i, slot) in shown.iter_mut().enumerate() {
            let held = !want.career && self.is_under_any(i, &self.bounty_row);
            // The lap group is the tollbooth milestone group (not bound); a lap count shows in the progress group.
            let race_part =
                self.is_under_any(i, &self.race_lap) || (!want.countdown && self.is_under_any(i, &self.countdown));
            // The rows past the racers are hidden, and the online headsets always (there is no online game).
            let unused_row = self
                .leader_rows
                .iter()
                .enumerate()
                .any(|(k, row)| k >= usize::from(want.leaders) && self.is_under_any(i, row));
            if held || race_part || unused_row || self.is_under_any(i, &self.leader_icons) {
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
