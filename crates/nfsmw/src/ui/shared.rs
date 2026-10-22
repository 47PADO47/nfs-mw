//! The assets and the presenter as Bevy resources, so the HUD and the menus load the fonts and textures once and
//! upload each texture once.

use std::sync::Arc;

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use game_install::GameDir;

use super::UiAssets;
use super::loading::{ASSETS, AssetsLoad, UiLoading, publish_assets, tell_host};
use super::present::BlackboxPresenter;
use crate::app::FrameSet;

/// The fonts, textures and strings, loaded once. Inserted when their background load is over (`ui/loading.rs`): the
/// systems that draw with them wait for it (`run_if(resource_exists::<SharedAssets>)`).
#[derive(Resource, Clone)]
pub struct SharedAssets(pub Arc<UiAssets>);

/// The presenter and the textures it has uploaded.
#[derive(Resource, Default)]
pub struct Presenter(pub BlackboxPresenter);

/// Starts loading the assets on a background thread and adds the presenter, unless an earlier plugin did. The handle
/// says when the assets are there; a load that fails leaves the UI off.
pub fn ensure(app: &mut App, dir: &GameDir) -> AssetsLoad {
    if let Some(load) = app.world().get_resource::<AssetsLoad>() {
        return load.clone();
    }
    let load = AssetsLoad::start(dir.clone());
    app.init_resource::<UiLoading>();
    app.world_mut().resource_mut::<UiLoading>().begin(ASSETS);
    app.insert_resource(load.clone())
        .init_resource::<Presenter>()
        .add_systems(Update, publish_assets.in_set(FrameSet::Prepare))
        .add_systems(Update, tell_host.after(FrameSet::Prepare).before(FrameSet::Draw));
    load
}
