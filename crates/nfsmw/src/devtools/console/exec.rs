//! Running parsed commands, and applying the settings they changed.

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};

use super::parse::{self, BUILT_IN, Command};
use super::{Console, settings_cmd};
use crate::app::Host;
use crate::app::pacing::FrameLimiter;
use crate::devtools::logbuf;
use crate::settings::Settings;

/// Run the lines typed since last frame and print what they say.
pub fn execute(
    mut console: ResMut<Console>,
    mut settings: ResMut<Settings>,
    mut host: NonSendMut<Host>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    let host = &mut *host;
    console.scene_commands = host.scene.commands();
    // Scene commands need the renderer; keep the lines for when it exists.
    if host.renderer.is_none() {
        return;
    }
    for line in std::mem::take(&mut console.pending) {
        logbuf::input(&format!("> {line}"));
        let result = match parse::parse(&line) {
            Ok(None) => continue,
            Ok(Some(command)) => run(command, &mut settings, host, &mut window, &mut exit),
            Err(e) => Err(e),
        };
        match result {
            Ok(text) if text.is_empty() => {}
            Ok(text) => logbuf::output(&text),
            Err(e) => logbuf::error(&e),
        }
    }
}

fn run(
    command: Command,
    settings: &mut Settings,
    host: &mut Host,
    window: &mut Window,
    exit: &mut MessageWriter<AppExit>,
) -> Result<String, String> {
    match command {
        Command::Help => Ok(help(host.scene.commands())),
        Command::Clear => {
            logbuf::clear();
            Ok(String::new())
        }
        Command::Quit => {
            exit.write(AppExit::Success);
            Ok("bye".into())
        }
        Command::Get(None) => Ok(settings_cmd::get_all(settings)),
        Command::Get(Some(key)) => settings_cmd::get(settings, &key),
        Command::Set { key, value } => settings_cmd::set(settings, &key, &value),
        Command::Resolution { width, height } => {
            window.resolution.set_physical_resolution(width, height);
            Ok(format!("window {width}x{height}"))
        }
        Command::Scene { name, args } => {
            let renderer = host.renderer.as_mut().ok_or("the renderer is not ready")?;
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            host.scene
                .command(renderer, &name, &args)
                .unwrap_or_else(|| Err(format!("unknown command {name:?} (type help)")))
        }
    }
}

fn help(scene: &[(&str, &str)]) -> String {
    let width = BUILT_IN.iter().chain(scene).map(|(usage, _)| usage.len()).max().unwrap_or(0);
    BUILT_IN
        .iter()
        .chain(scene)
        .map(|(usage, what)| format!("  {usage:<width$}  {what}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Push changed settings into the parts that hold them: the frame limiter and the swapchain.
pub fn sync_settings(settings: Res<Settings>, mut host: NonSendMut<Host>, mut applied: Local<Option<Settings>>) {
    let Some(before) = applied.replace(*settings) else { return };
    if before == *settings {
        return;
    }
    if before.max_fps != settings.max_fps {
        host.limiter = FrameLimiter::new(settings.max_fps);
    }
    if before.vsync != settings.vsync
        && let Some(renderer) = host.renderer.as_mut()
    {
        renderer.set_vsync(settings.vsync);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_lists_built_in_and_scene_commands_aligned() {
        let text = help(&[("car <folder>", "change the car")]);
        assert!(text.contains("quit"));
        assert!(text.contains("car <folder>"));
        let scene = [("car <folder>", "change the car")];
        let columns: Vec<usize> = BUILT_IN
            .iter()
            .chain(&scene)
            .map(|(_, what)| text.lines().find(|l| l.ends_with(what)).and_then(|l| l.rfind(what)).unwrap())
            .collect();
        assert!(columns.windows(2).all(|w| w[0] == w[1]), "descriptions line up: {columns:?}");
    }
}
