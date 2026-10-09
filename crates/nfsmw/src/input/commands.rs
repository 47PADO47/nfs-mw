//! Live reassignment through the console; persistence is explicit.

use super::{Action, Bindings, source::Family};
use crate::settings::Settings;

pub fn run(bindings: &mut Bindings, settings: &Settings, name: &str, args: &[String]) -> Result<String, String> {
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    match (name, words.as_slice()) {
        ("bind" | "addbind", [action, input]) => {
            let action = Action::parse(action)?;
            bindings.bind(action, input, name == "addbind")?;
            Ok(format!("{} = {input} (session only; bind-save persists)", action.name()))
        }
        ("unbind", [action]) => {
            bindings.unbind(Action::parse(action)?, None);
            Ok(format!("{action} unbound (session only)"))
        }
        ("unbind", [action, family]) => {
            let family = match *family {
                "keyboard" => Some(Family::Keyboard),
                "mouse" => Some(Family::Mouse),
                "gamepad" => Some(Family::Gamepad),
                "all" => None,
                _ => return Err("device family is keyboard, mouse, gamepad or all".into()),
            };
            bindings.unbind(Action::parse(action)?, family);
            Ok(format!("{action} assignments removed (session only)"))
        }
        ("bind-reset", []) => {
            bindings.reset(None, settings);
            Ok("default input bindings restored (session only)".into())
        }
        ("bind-reset", [action]) => {
            bindings.reset(Some(Action::parse(action)?), settings);
            Ok(format!("{action} default assignments restored (session only)"))
        }
        ("bind-save", []) => {
            let path = Settings::config_path().ok_or("there is no per-user config folder")?;
            bindings.save_to(&path).map_err(|e| format!("{e:#}"))?;
            Ok(format!("input bindings saved to {}", path.display()))
        }
        _ => {
            Err("usage: bind/addbind <action> <input>, unbind <action> [device], bind-reset [action], bind-save".into())
        }
    }
}
