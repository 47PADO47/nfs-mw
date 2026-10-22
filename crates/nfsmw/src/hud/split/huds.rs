//! The split-screen HUDs the app holds: the race pair and the drag pair, whichever the install has.

use bevy_ecs::prelude::Resource;
use blackbox_feng::UiTree;
use game_install::GameDir;

use super::Kind;
use super::pair::Pair;
use crate::hud::state::HudState;
use crate::settings::HudLayout;
use crate::ui::UiAssets;
use crate::ui::present::Screen;

#[derive(Resource)]
pub struct SplitHuds {
    pairs: Vec<Pair>,
}

impl SplitHuds {
    /// Loads both pairs; `None` when the install has neither.
    pub fn load(dir: &GameDir, assets: &UiAssets) -> Option<Self> {
        let pairs: Vec<Pair> = Kind::BOTH.into_iter().filter_map(|kind| Pair::load(dir, assets, kind)).collect();
        if pairs.is_empty() {
            log::warn!("the split-screen HUDs are off: the install has no {}", super::SPLIT_FILE);
            return None;
        }
        Some(Self { pairs })
    }

    /// One frame: feeds the pair the race needs (a drag race its own, the other pair rests) and returns the two trees to
    /// draw. A player whose state is not `visible` has no tree. `None` when the install has no pair for the race.
    pub fn frame(
        &mut self,
        states: &[HudState; 2],
        dt: f32,
        assets: &UiAssets,
        screen: Screen,
        layout: HudLayout,
    ) -> Option<[Option<UiTree>; 2]> {
        let kind = Kind::of(states[0].drag);
        let pair = self.pairs.iter_mut().find(|p| p.kind == kind)?;
        pair.step(states, dt, assets);
        let [first, second] = pair.trees(screen, layout);
        Some([states[0].visible.then_some(first), states[1].visible.then_some(second)])
    }
}
