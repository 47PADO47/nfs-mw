//! Command dispatch.

mod install;

use anyhow::Result;
use nfsmw_data::game::open_install;

use crate::cli::{Cli, Command};
use crate::scenes::{car::CarScene, world::WorldScene};
use crate::settings::Settings;

pub fn run(cli: Cli) -> Result<()> {
    let game_dir = cli.game_dir.as_deref();
    match cli.command {
        Command::CheckInstall => install::check(game_dir),
        Command::ListCars => {
            for car in nfsmw_data::car::list(&open_install(game_dir)?) {
                println!("{car}");
            }
            Ok(())
        }
        Command::ListMovies => {
            for movie in crate::movie::list(&open_install(game_dir)?) {
                println!("{movie}");
            }
            Ok(())
        }
        Command::ListScreens => {
            let catalog = crate::ui::Catalog::load(&open_install(game_dir)?, &crate::ui::SCREEN_FILES);
            print!("{}", crate::frontend::dump::list(&catalog));
            Ok(())
        }
        Command::Strings { filter } => {
            let dir = open_install(game_dir)?;
            let data = nfsmw_data::read_unwrapped(&dir, "LANGUAGES/English.bin")?;
            let table = blackbox_text::StringTable::from_file(&data)?;
            print!("{}", crate::frontend::dump::strings(&table, &filter));
            Ok(())
        }
        Command::DumpScreen { name } => {
            let catalog = crate::ui::Catalog::load(&open_install(game_dir)?, &crate::ui::SCREEN_FILES);
            let package =
                catalog.find(&name).ok_or_else(|| anyhow::anyhow!("no screen {name:?} (try list-screens)"))?;
            print!("{}", crate::frontend::dump::dump(package));
            Ok(())
        }
        Command::PlayMovie { name, start, view } => {
            let dir = open_install(game_dir)?;
            let scene = crate::movie::MovieScene::open(&dir, &name, f64::from(start))?;
            let run_options = view.run_options(&dir, false);
            crate::app::run(Box::new(scene), &Settings::load(view.settings_layer()), run_options)
        }
        Command::ViewCar { car, lod, all_parts, preset, yaw, view } => {
            let dir = open_install(game_dir)?;
            let options = nfsmw_data::car::LoadOptions { lod, all_parts, preset };
            let model = nfsmw_data::car::load(&dir, &car, &options)?;
            let run_options = view.run_options(&dir, false);
            crate::app::run(
                Box::new(CarScene::new(model, yaw).with_source(dir, options)),
                &Settings::load(view.settings_layer()),
                run_options,
            )
        }
        Command::ViewWorld { at, height, heading, pitch, fog_distance, wait_for_load, drive, drive_script, view } => {
            let dir = open_install(game_dir)?;
            let driving = drive.is_some();
            let drive = drive.map(|car| crate::scenes::world::DriveOptions { car, script: drive_script });
            let options =
                crate::scenes::world::Options { at, height, heading, pitch, fog_distance, wait_for_load, drive };
            crate::app::run(
                Box::new(WorldScene::open(&dir, options)?),
                &Settings::load(view.settings_layer()),
                view.run_options(&dir, driving),
            )
        }
    }
}
