//! Reading the front end's screens and assets, on the front end's loader thread (`frontend/pending.rs`).

use game_install::GameDir;

use super::Screens;
use crate::ui::{AssetsLoad, Catalog, SCREEN_FILES};

impl Screens {
    /// Reads the screens' packages, then waits for the UI assets that load beside them on their own thread
    /// (`ui::ensure`). They read overlapping files (`FrontB.lzc` through the session memo), so the second reader of a file
    /// waits for the first instead of reading it again. `None` when the assets could not be read. It waits, so it never
    /// runs on the main thread.
    pub fn load(dir: &GameDir, assets: &AssetsLoad) -> Option<Self> {
        let catalog = Catalog::load(dir, &SCREEN_FILES);
        Some(Self::new(catalog, assets.wait()?))
    }
}
