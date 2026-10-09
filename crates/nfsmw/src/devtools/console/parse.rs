//! Turning a typed line into a [`Command`]. Pure text in, plain data out.

/// A console command. Scene-specific ones (`car`, `freecam`…) are passed on by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Clear,
    Quit,
    /// Actual window mode, size, position and focus (as opposed to requested preferences).
    Window,
    Monitors,
    /// Every key, button and stick binding.
    Keys,
    Bindings {
        name: String,
        args: Vec<String>,
    },
    /// Show one setting, or all of them.
    Get(Option<String>),
    Set {
        key: String,
        value: String,
    },
    /// Resize the window to `width` × `height` pixels.
    Resolution {
        width: u32,
        height: u32,
    },
    /// Not built in: offered to the scene.
    Scene {
        name: String,
        args: Vec<String>,
    },
}

/// Names of the built-in commands, for `help` and tab completion.
pub const BUILT_IN: [(&str, &str); 34] = [
    ("bind <action> <input>", "replace one device family's action assignments; keys lists action names"),
    ("addbind <action> <input>", "add another physical input to an action"),
    ("unbind <action> [keyboard|mouse|gamepad|all]", "remove action assignments"),
    ("bind-reset [action]", "restore default assignments for one action or all"),
    ("bind-save", "save the live assignments to the config file"),
    ("help", "list the commands"),
    ("clear", "empty the console"),
    ("quit", "close the game"),
    ("get [setting]", "show a setting, or all"),
    ("set <setting> <value>", "change a setting (get lists the keys)"),
    ("fps <number|unlocked>", "frame-rate cap (same as set fps)"),
    ("resolution <width> <height>|native", "window size or exclusive video mode (also WIDTHxHEIGHT)"),
    ("window_mode <windowed|borderless|exclusive>", "change the window mode (Alt+Enter toggles fullscreen)"),
    ("monitor <current|primary|index>", "select a monitor, indices start at zero"),
    ("window", "show the actual window size, mode, DPI and focus"),
    ("monitors", "list available monitors and their indices"),
    ("keys", "list every key, button and stick binding"),
    ("graphics_preset <custom|low|medium|high>", "set the graphics settings a preset stands for"),
    ("car_shading <simple|glossy>", "car shading: glossy or the cheap single-light look"),
    ("smoke_quality <standard|high>", "change tire smoke presentation quality"),
    ("hud-layout <pc|classic|xbox360>", "change HUD placement and scale"),
    ("post_tonemap <off|aces>", "filmic tone mapping (darkens the image)"),
    ("post_bloom <off|low|medium|high>", "glow around bright areas"),
    ("post_aa <off|fxaa>", "anti-aliasing of the 3D scene"),
    ("collision_sparks <on|off>", "change collision sparks"),
    ("spark_style <original-pc|restored-experimental>", "select stock or experimental collision particles"),
    ("speed_trails <on|off>", "change experimental high-speed wind trails"),
    ("exhaust_flames <on|off>", "change the tail-pipe flames (shift blow-off, lift-off backfire)"),
    ("volume <0-100>", "master volume (same as set volume)"),
    ("sound [bank [index]]", "list the sounds of a bank (IG_GLOBAL/Siren_MB.abk) or play one"),
    ("engine <car> [percent] | off", "hold a car's engine sound at a share of its RPM range"),
    ("radio [list|next|play <n>|on|off|shuffle|ordered]", "the radio: show what plays, skip, switch it"),
    (
        "speech [list|say <event>|play <event>|bank <n> [take]|stop]",
        "the police dispatch's lines: queue an event, play one now",
    ),
    ("music [pursuit <1-4> [0-100]|intensity <0-100>|clear]", "the pursuit music: show it, force a chase to test it"),
];

/// Further shorthands for `set`: `vsync off` is `set vsync off`.
const SET_SHORTHANDS: [&str; 23] = [
    "fps",
    "vsync",
    "metrics",
    "volume",
    "readout",
    "window_mode",
    "monitor",
    "tire_smoke",
    "skid_marks",
    "collision_sparks",
    "spark_style",
    "speed_trails",
    "exhaust_flames",
    "smoke_quality",
    "car_shading",
    "graphics_preset",
    "hud_layout",
    "hud-layout",
    "radio_hud",
    "radio-hud",
    "post_tonemap",
    "post_bloom",
    "post_aa",
];

/// Parse one line. `Ok(None)` for an empty line.
pub fn parse(line: &str) -> Result<Option<Command>, String> {
    let mut words = line.split_whitespace();
    let Some(name) = words.next() else { return Ok(None) };
    let args: Vec<&str> = words.collect();
    let name = name.to_ascii_lowercase();
    if matches!(name.as_str(), "bind" | "addbind" | "unbind" | "bind-reset" | "bind-save") {
        return Ok(Some(Command::Bindings { name, args: args.iter().map(|a| (*a).to_owned()).collect() }));
    }
    Ok(Some(match name.as_str() {
        "help" | "?" => Command::Help,
        "clear" | "cls" => Command::Clear,
        "quit" | "exit" => Command::Quit,
        "window" => Command::Window,
        "monitors" => Command::Monitors,
        "keys" | "bindings" | "controls" => Command::Keys,
        "get" => match args.as_slice() {
            [] => Command::Get(None),
            [key] => Command::Get(Some((*key).to_owned())),
            _ => return Err("usage: get [setting]".into()),
        },
        "set" => match args.as_slice() {
            // No value: a switch flips, any other setting says what it takes (an empty value).
            [key] => Command::Set { key: key.to_ascii_lowercase(), value: String::new() },
            [key, value] => Command::Set { key: key.to_ascii_lowercase(), value: (*value).to_owned() },
            _ => return Err("usage: set <setting> [value] (get lists the settings)".into()),
        },
        "tire-effects" if matches!(args.as_slice(), ["smoke" | "marks", _]) => {
            let key = match args[0] {
                "smoke" => "tire_smoke",
                _ => "skid_marks",
            };
            Command::Set { key: key.to_owned(), value: args[1].to_owned() }
        }
        shorthand if SET_SHORTHANDS.contains(&shorthand) => {
            if args.len() > 1 {
                return Err(format!("usage: {shorthand} <value>"));
            }
            Command::Set {
                key: shorthand.to_owned(),
                value: args.first().map_or_else(String::new, |v| (*v).to_owned()),
            }
        }
        "resolution" | "res" => {
            let usage = "resolution <width> <height> (or WIDTHxHEIGHT)";
            if args.is_empty() {
                return Ok(Some(Command::Set { key: "resolution".into(), value: String::new() }));
            }
            if args.as_slice() == ["native"] {
                return Ok(Some(Command::Set { key: "resolution".into(), value: "native".into() }));
            }
            let (w, h) = match args.as_slice() {
                [both] => both.split_once(['x', 'X']).ok_or_else(|| format!("usage: {usage}"))?,
                [w, h] => (*w, *h),
                _ => return Err(format!("usage: {usage}")),
            };
            let size = |s: &str| s.parse::<u32>().ok().filter(|n| (160..=16384).contains(n));
            match (size(w), size(h)) {
                (Some(width), Some(height)) => Command::Resolution { width, height },
                _ => return Err("the width and height must be between 160 and 16384".into()),
            }
        }
        _ => Command::Scene { name, args: args.iter().map(|a| (*a).to_owned()).collect() },
    }))
}

/// The built-in and scene command names that start with `prefix`, for Tab.
pub fn complete(prefix: &str, scene_commands: &[(&str, &str)]) -> Vec<String> {
    let first = |usage: &str| usage.split_whitespace().next().unwrap_or("").to_owned();
    let mut names: Vec<String> =
        BUILT_IN.iter().map(|(u, _)| first(u)).chain(scene_commands.iter().map(|(u, _)| first(u))).collect();
    names.retain(|n| n.starts_with(&prefix.to_ascii_lowercase()));
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(line: &str) -> Command {
        parse(line).unwrap().unwrap()
    }

    #[test]
    fn built_ins() {
        assert_eq!(parse("   ").unwrap(), None);
        assert_eq!(ok("HELP"), Command::Help);
        assert_eq!(ok("exit"), Command::Quit);
        assert_eq!(ok("get"), Command::Get(None));
        assert_eq!(ok("get fps"), Command::Get(Some("fps".into())));
        assert_eq!(ok("set Vsync off"), Command::Set { key: "vsync".into(), value: "off".into() });
    }

    #[test]
    fn shorthands_are_sets() {
        assert_eq!(ok("fps 60"), Command::Set { key: "fps".into(), value: "60".into() });
        assert_eq!(ok("metrics advanced"), Command::Set { key: "metrics".into(), value: "advanced".into() });
        assert_eq!(ok("readout full"), Command::Set { key: "readout".into(), value: "full".into() });
        assert!(parse("fps 1 2").is_err());
    }

    #[test]
    fn set_without_a_value_asks_for_its_usage_or_flips() {
        let bare = |key: &str| Command::Set { key: key.into(), value: String::new() };
        assert_eq!(ok("set fps"), bare("fps"));
        assert_eq!(ok("SET Vsync"), bare("vsync"));
        assert_eq!(ok("fps"), bare("fps"));
        assert_eq!(ok("resolution"), bare("resolution"));
        assert!(parse("set").is_err() && parse("set a b c").is_err());
    }

    #[test]
    fn resolution_forms() {
        let want = Command::Resolution { width: 1920, height: 1080 };
        assert_eq!(ok("resolution 1920 1080"), want);
        assert_eq!(ok("res 1920x1080"), want);
        assert!(parse("resolution 1920").is_err());
        assert!(parse("resolution 10 10").is_err());
        assert!(parse("resolution a b").is_err());
        assert_eq!(ok("resolution native"), Command::Set { key: "resolution".into(), value: "native".into() });
        assert_eq!(
            ok("window_mode borderless"),
            Command::Set { key: "window_mode".into(), value: "borderless".into() }
        );
        assert_eq!(ok("monitor primary"), Command::Set { key: "monitor".into(), value: "primary".into() });
        assert_eq!(ok("window"), Command::Window);
        assert_eq!(ok("monitors"), Command::Monitors);
    }

    #[test]
    fn keys_has_three_names() {
        for name in ["keys", "bindings", "CONTROLS"] {
            assert_eq!(ok(name), Command::Keys);
        }
    }

    #[test]
    fn unknown_commands_go_to_the_scene() {
        assert_eq!(ok("car BMWM3GTR"), Command::Scene { name: "car".into(), args: vec!["BMWM3GTR".into()] });
        assert_eq!(ok("FreeCam"), Command::Scene { name: "freecam".into(), args: vec![] });
    }

    #[test]
    fn completion() {
        let scene = [("car <folder>", "change the car"), ("cars", "list the cars")];
        assert_eq!(complete("c", &scene), ["car", "car_shading", "cars", "clear", "collision_sparks"]);
        assert_eq!(complete("re", &scene), ["resolution"]);
        assert_eq!(complete("vo", &scene), ["volume"]);
        assert!(complete("zzz", &scene).is_empty());
    }
}
