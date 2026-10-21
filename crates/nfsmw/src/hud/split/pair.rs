//! The two HUDs of one kind of race (race or drag) in one FEng runtime: loading, feeding both from their players' states,
//! and the trees to draw.

use blackbox_feng::{PackageId, Runtime, UiTree};
use game_install::GameDir;

use super::map::SharedMap;
use super::place::Placement;
use super::seat::SeatHud;
use super::{Kind, Seat, package};
use crate::hud::state::HudState;
use crate::settings::HudLayout;
use crate::ui::UiAssets;
use crate::ui::present::Screen;

pub struct Pair {
    pub kind: Kind,
    runtime: Runtime,
    seats: [SeatHud; 2],
    placements: [Placement; 2],
    map: Option<SharedMap>,
}

impl Pair {
    /// Loads the two packages of `kind`; `None` when the install lacks them.
    pub fn load(dir: &GameDir, assets: &UiAssets, kind: Kind) -> Option<Self> {
        let [first, second] = package::load(dir, kind)?;
        let missing: Vec<&str> =
            assets.missing_resources(&first).into_iter().chain(assets.missing_resources(&second)).collect();
        log::info!("split HUD {kind:?}: {} resources without a texture ({missing:?})", missing.len());
        let mut runtime = crate::ui::runtime();
        if let Some(strings) = assets.strings.clone() {
            runtime.set_string_resolver(move |label| strings.get(label));
        }
        let ids = [runtime.load(first), runtime.load(second)];
        let map = match kind {
            Kind::Race => SharedMap::open(&mut runtime, ids[0], dir, assets),
            Kind::Drag => None,
        };
        Some(Self::assemble(runtime, ids, kind, map))
    }

    /// The pair for two packages already loaded into `runtime`.
    pub(super) fn assemble(runtime: Runtime, ids: [PackageId; 2], kind: Kind, map: Option<SharedMap>) -> Self {
        let icons = if map.is_some() { SharedMap::icon_hashes() } else { Vec::new() };
        let seats = Seat::BOTH.map(|seat| SeatHud::new(&runtime, ids[seat.index()], kind, seat));
        let placements = [Placement::new(&runtime, ids[0], &icons), Placement::new(&runtime, ids[1], &[])];
        Self { kind, runtime, seats, placements, map }
    }

    /// Feeds both HUDs for a frame of `dt` seconds. `states` are the players' (the first player's picks the map).
    pub fn step(&mut self, states: &[HudState; 2], dt: f32, assets: &UiAssets) {
        let map_shown = self.map.is_some() && states[0].minimap.is_some();
        if let Some(map) = self.map.as_mut() {
            map.advance(dt);
            map.follow(&states[0], assets);
        }
        for seat in &mut self.seats {
            seat.advance(dt);
            let own_map = map_shown && seat.seat == Seat::First;
            seat.apply(&mut self.runtime, &states[seat.seat.index()], own_map);
        }
        // After the layers' visibility: the second arrow hides and shows itself.
        if let (true, Some(map)) = (map_shown, self.map.as_mut()) {
            map.apply(&mut self.runtime, &states[0], &states[1]);
        }
        self.runtime.update(dt);
        let _ = self.runtime.take_outgoing();
    }

    /// The trees to draw, placed in the window: the first player's, then the second's.
    pub fn trees(&self, screen: Screen, layout: HudLayout) -> [UiTree; 2] {
        std::array::from_fn(|i| {
            let mut tree = self.runtime.tree(self.seats[i].package);
            self.placements[i].apply(&mut tree, screen, layout);
            tree
        })
    }
}
