//! Running parsed commands, and applying the settings they changed.

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};

use super::parse::{self, BUILT_IN, Command};
use super::{Console, settings_cmd};
use crate::app::Host;
use crate::app::pacing::FrameLimiter;
use crate::app::window::WindowModes;
use crate::audio::Audio;
use crate::devtools::logbuf;
use crate::input::Bindings;
use crate::settings::Settings;

/// Run the lines typed since last frame and print what they say.
pub fn execute(
    mut console: ResMut<Console>,
    mut settings: ResMut<Settings>,
    mut host: NonSendMut<Host>,
    mut audio: Option<NonSendMut<Audio>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    (modes, bindings): (Res<WindowModes>, Res<Bindings>),
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
            Ok(Some(Command::Keys)) => Ok(bindings.describe()),
            Ok(Some(command)) => {
                run(command, &mut settings, host, audio.as_deref_mut(), &mut window, &modes, &mut exit)
            }
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
    audio: Option<&mut Audio>,
    window: &mut Window,
    modes: &WindowModes,
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
        Command::Window => Ok(WindowModes::status(window)),
        Command::Monitors => Ok(modes.monitors.clone()),
        Command::Keys => Err("keys is answered by the console before commands run".into()),
        Command::Set { key, value } => set_live(settings, host, &key, &value),
        Command::Resolution { width, height } => {
            if host.screenshot.is_some() {
                return Err("screenshot runs keep a hidden window at their fixed resolution".into());
            }
            settings_cmd::set(settings, "resolution", &format!("{width}x{height}"))
        }
        Command::Scene { name, args } if crate::audio::commands::handles(&name) => {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            crate::audio::commands::run(audio, &name, &args)
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

/// Apply console settings immediately, so later commands in the same frame see their scene effects.
pub(super) fn set_live(settings: &mut Settings, host: &mut Host, key: &str, value: &str) -> Result<String, String> {
    if host.screenshot.is_some() && matches!(key, "window_mode" | "monitor" | "resolution") {
        return Err("screenshot runs keep a hidden window at their fixed resolution".into());
    }
    let text = settings_cmd::set(settings, key, value)?;
    if matches!(key, "tire_smoke" | "skid_marks") {
        host.set_tire_effects(settings.tire_smoke, settings.skid_marks);
    }
    if key == "smoke_quality" {
        host.set_smoke_quality(settings.smoke_quality);
    }
    Ok(text)
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
    if applied.as_ref().is_none_or(|before| before.smoke_quality != settings.smoke_quality) {
        host.set_smoke_quality(settings.smoke_quality);
    }
    if applied
        .as_ref()
        .is_none_or(|before| before.tire_smoke != settings.tire_smoke || before.skid_marks != settings.skid_marks)
    {
        host.set_tire_effects(settings.tire_smoke, settings.skid_marks);
    }
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
