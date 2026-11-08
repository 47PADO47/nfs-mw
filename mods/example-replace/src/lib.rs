//! `example-replace`: replaces a street lamp model and a road texture of the 2005 city through replacement sets
//! (`docs/modding/api-replace.md`). The mod has no game code at run time: when it loads it writes its files and the two manifests
//! into its own folder (`ctx.mod_dir()`), and the game reads them after every mod has loaded and adds the options "Street lamps"
//! and "Road textures" to the gameplay settings. The files are made here from code: the repository holds no model or picture.

pub mod generate;

use blackbox_modding::sdk::{Ctx, Level, Mod, ModResult, declare_mod};

/// The example mod.
pub struct ExampleReplace;

impl Mod for ExampleReplace {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let folder = ctx.mod_dir()?;
        for file in generate::files() {
            let path = folder.join(file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|_| blackbox_modding::sdk::Error::Host)?;
            }
            std::fs::write(&path, &file.bytes).map_err(|_| blackbox_modding::sdk::Error::Host)?;
        }
        ctx.log(Level::Info, "example.replace: wrote the lamp and road sets");
        Ok(ExampleReplace)
    }
}

declare_mod!(ExampleReplace);
