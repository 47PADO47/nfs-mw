//! The assets and the presenter as Bevy resources, so the HUD and the menus load the fonts and textures once and
//! upload each texture once.

use std::sync::Arc;

use bevy_app::App;
use bevy_ecs::prelude::*;
use game_install::GameDir;

use super::UiAssets;
use super::fonts::{CustomFontSet, scan};
use super::present::BlackboxPresenter;

/// The fonts, textures and strings, loaded once.
#[derive(Resource, Clone)]
pub struct SharedAssets(pub Arc<UiAssets>);

/// The presenter and the textures it has uploaded.
#[derive(Resource, Default)]
pub struct Presenter(pub BlackboxPresenter);

/// Loads the assets and adds the presenter unless an earlier plugin did. `None` when the install cannot supply them.
pub fn ensure(app: &mut App, dir: &GameDir) -> Option<Arc<UiAssets>> {
    if let Some(shared) = app.world().get_resource::<SharedAssets>() {
        return Some(shared.0.clone());
    }
    // The custom fonts are read whether or not they are on, so the console can switch them on without a restart.
    let custom =
        crate::settings::custom_fonts::folder().map(|folder| CustomFontSet::load(&scan(&folder))).unwrap_or_default();
    let assets = match UiAssets::load_with(dir, custom) {
        Ok(a) => Arc::new(a),
        Err(e) => {
            log::warn!("the UI is off: {e:#}");
            return None;
        }
    };
    app.insert_resource(SharedAssets(assets.clone())).init_resource::<Presenter>();
    Some(assets)
}
