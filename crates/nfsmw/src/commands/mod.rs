//! Command dispatch.

mod install;

use anyhow::Result;
use nfsmw_data::game::open_install;

use crate::cli::{Cli, Command};
use crate::input::Bindings;
use crate::scenes::{car::CarScene, world::WorldScene};
use crate::settings::{Partial, Settings};

/// Runs the window with the front end on top of an empty backdrop.
fn run_front_end(
    dir: &game_install::GameDir,
    start: crate::frontend::Start,
    script: Option<&str>,
    view: &crate::cli::ViewArgs,
) -> Result<()> {
    let script = script.map(crate::frontend::UiScript::parse).transpose().map_err(anyhow::Error::msg)?;
    let mut options = view.run_options(dir, true);
    options.frontend = Some(crate::frontend::FrontendPlugin { dir: dir.clone(), start, script });
    crate::app::run(Box::new(crate::frontend::MenuScene), &Settings::load(view.settings_layer()), options)
}

pub fn run(cli: Cli) -> Result<()> {
    let game_dir = cli.game_dir.as_deref();
    // No command at all: the game, as `play` starts it.
    let command = cli.command.unwrap_or_else(Command::play);
    match command {
        Command::CheckInstall => install::check(game_dir),
        Command::Keys => {
            let s = Settings::load(Partial::default());
            print!("{}", Bindings::with_paddles(s.paddle_up, s.paddle_down).describe());
            Ok(())
        }
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
        Command::Play { skip_boot, drive, ui_script, view } => {
            let dir = open_install(game_dir)?;
            let start = match (drive, skip_boot || ui_script.is_some()) {
                (true, _) => crate::frontend::Start::Drive,
                (false, true) => crate::frontend::Start::Menu,
                (false, false) => crate::frontend::Start::Boot,
            };
            run_front_end(&dir, start, ui_script.as_deref(), &view)
        }
        Command::ViewScreen { name, pause, options, category, ui_script, view } => {
            let dir = open_install(game_dir)?;
            let category = match category.to_ascii_lowercase().as_str() {
                "audio" => crate::frontend::Category::Audio,
                "video" => crate::frontend::Category::Video,
                "gameplay" => crate::frontend::Category::Gameplay,
                other => anyhow::bail!("unknown category {other:?} (audio, video, gameplay)"),
            };
            if crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES).find(&name).is_none() {
                anyhow::bail!("no screen {name:?} in the install (try list-screens)");
            }
            let start = crate::frontend::Start::Screen(name, crate::frontend::Args { pause, options, category });
            let settle = view.screenshot.is_some().then_some("wait 2");
            run_front_end(&dir, start, ui_script.as_deref().or(settle), &view)
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
