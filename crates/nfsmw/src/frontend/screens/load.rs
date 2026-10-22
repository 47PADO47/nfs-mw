//! Reading the front end's screens and assets at startup.

use bevy_app::App;
use game_install::GameDir;

use super::Screens;
use crate::ui::{Catalog, SCREEN_FILES};

impl Screens {
    /// Reads the screens' packages and the UI assets (`ui::ensure`). They read overlapping files (`FrontB.lzc` through
    /// the session memo), so they load side by side: the second reader of a file waits for the first instead of reading
    /// it again. `None` when the assets could not be read.
    pub fn load(app: &mut App, dir: &GameDir) -> Option<Self> {
        let (assets, catalog) = std::thread::scope(|scope| {
            let catalog = scope.spawn(|| Catalog::load(dir, &SCREEN_FILES));
            let assets = crate::ui::ensure(app, dir);
            (assets, catalog.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
        });
        Some(Self::new(catalog, assets?))
    }
}
