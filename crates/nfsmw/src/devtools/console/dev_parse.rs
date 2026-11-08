//! The `vehicle`, `gear` and `devtools` console commands as plain data: text in, [`DevCommand`] out. `dev_cmd.rs` runs them.

use crate::devtools::pad_nav::Button;
use crate::devtools::panels::Panel;
use crate::garage::rider_gear::Part;

/// A `vehicle`, `gear` or `devtools` command.
#[derive(Debug, Clone, PartialEq)]
pub enum DevCommand {
    Vehicle(VehicleCommand),
    Gear(GearCommand),
    Devtools(DevtoolsCommand),
}

/// `vehicle ...`: what the vehicle on show can change (`garage::options`).
#[derive(Debug, Clone, PartialEq)]
pub enum VehicleCommand {
    Slots,
    Set {
        slot: String,
        choice: String,
    },
    Reset,
    Groups,
    /// `Some(true)`: on, `Some(false)`: off, `None`: back to stock.
    Group {
        name: String,
        on: Option<bool>,
    },
    Color(Option<[u8; 3]>),
    /// Open the vehicle panel.
    Panel,
}

/// `gear ...`: the rider's helmet, suit, gloves, boots and body.
#[derive(Debug, Clone, PartialEq)]
pub enum GearCommand {
    List,
    Set { part: Part, choice: String },
    Stock,
}

/// `devtools ...`: the panels, without the keyboard.
#[derive(Debug, Clone, PartialEq)]
pub enum DevtoolsCommand {
    Open(Panel),
    Close,
    /// A controller button pressed once, for tests and screenshots.
    Press(Button),
}

/// The commands for `help` and Tab: usage and description.
pub const DEV_COMMANDS: [(&str, &str); 9] = [
    (
        "vehicle [slots]",
        "the slots of the vehicle on show with their numbered choices, from its own data (components or categories)",
    ),
    (
        "vehicle set <slot> <number|id|name|stock|none>",
        "fit a choice of a slot at once: a number of `vehicle slots`, a component id, a part of its name",
    ),
    ("vehicle reset", "every slot back to stock and the body paint to the vehicle's own"),
    ("vehicle groups", "the groups of slots, which of them can be switched, and what the vehicle's data lacks"),
    ("vehicle group <name> on|off|stock", "switch a group of optional equipment, or put the group back to stock"),
    ("vehicle color <RRGGBB|stock>", "the body colour of a vehicle that can be repainted"),
    ("vehicle panel", "open the vehicle panel (F11)"),
    (
        "gear [list] | set <part> <number|id|name|stock> | stock",
        "the rider's gear: list it, change one part (body, helmet, suit, gloves, boots), take it all off; kept in the profile",
    ),
    (
        "devtools [menu|vehicle|console|lab|extra|garage|close|press <button>]",
        "open a developer panel or the menu (L3 + R3 on a controller), close them, or press a controller button (up down left right a b lb rb)",
    ),
];

/// Parses `name` (lower case) and its arguments when `name` is one of these commands; `None` for any other name.
pub fn parse(name: &str, args: &[&str]) -> Option<Result<DevCommand, String>> {
    match name {
        "vehicle" => Some(vehicle(args).map(DevCommand::Vehicle)),
        "gear" => Some(gear(args).map(DevCommand::Gear)),
        "devtools" => Some(devtools(args).map(DevCommand::Devtools)),
        _ => None,
    }
}

const VEHICLE_USAGE: &str = "usage: vehicle [slots | set <slot> <choice> | reset | groups | group <name> on|off|stock | color <RRGGBB|stock> | panel]";

fn vehicle(args: &[&str]) -> Result<VehicleCommand, String> {
    match args {
        [] | ["slots"] => Ok(VehicleCommand::Slots),
        ["set", slot, choice @ ..] if !choice.is_empty() => {
            Ok(VehicleCommand::Set { slot: (*slot).to_owned(), choice: choice.join(" ") })
        }
        ["set", ..] => Err("usage: vehicle set <slot> <number|id|name|stock|none>".into()),
        ["reset"] => Ok(VehicleCommand::Reset),
        ["groups"] => Ok(VehicleCommand::Groups),
        ["group", name, state] => {
            let on = match *state {
                "on" => Some(true),
                "off" => Some(false),
                "stock" => None,
                _ => return Err("usage: vehicle group <name> on|off|stock".into()),
            };
            Ok(VehicleCommand::Group { name: (*name).to_owned(), on })
        }
        ["color" | "colour", value] => color(value).map(VehicleCommand::Color),
        ["panel"] => Ok(VehicleCommand::Panel),
        _ => Err(VEHICLE_USAGE.into()),
    }
}

fn color(value: &str) -> Result<Option<[u8; 3]>, String> {
    if value == "stock" {
        return Ok(None);
    }
    let hex = value.trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
        return Err("the colour is RRGGBB or stock".into());
    }
    let number = u32::from_str_radix(hex, 16).map_err(|_| "the colour is RRGGBB or stock".to_owned())?;
    Ok(Some([(number >> 16) as u8, (number >> 8) as u8, number as u8]))
}

const GEAR_USAGE: &str =
    "usage: gear [list] | gear set <body|helmet|suit|gloves|boots> <number|id|name|stock> | gear stock";

fn gear(args: &[&str]) -> Result<GearCommand, String> {
    match args {
        [] | ["list"] => Ok(GearCommand::List),
        ["stock"] => Ok(GearCommand::Stock),
        ["set", part, choice @ ..] if !choice.is_empty() => {
            let part =
                Part::of_key(part).ok_or_else(|| "the parts are body, helmet, suit, gloves and boots".to_owned())?;
            Ok(GearCommand::Set { part, choice: choice.join(" ") })
        }
        _ => Err(GEAR_USAGE.into()),
    }
}

const DEVTOOLS_USAGE: &str = "usage: devtools [menu|vehicle|console|lab|extra|garage|close|press <button>]";

fn devtools(args: &[&str]) -> Result<DevtoolsCommand, String> {
    match args {
        [] => Ok(DevtoolsCommand::Open(Panel::Menu)),
        ["close"] => Ok(DevtoolsCommand::Close),
        ["press", button] => Button::of_word(button)
            .map(DevtoolsCommand::Press)
            .ok_or_else(|| "the buttons are up, down, left, right, a, b, lb and rb".to_owned()),
        [word] => Panel::of_word(word).map(DevtoolsCommand::Open).ok_or_else(|| DEVTOOLS_USAGE.to_owned()),
        _ => Err(DEVTOOLS_USAGE.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(name: &str, args: &[&str]) -> DevCommand {
        parse(name, args).unwrap().unwrap()
    }

    #[test]
    fn vehicle_commands() {
        assert_eq!(ok("vehicle", &[]), DevCommand::Vehicle(VehicleCommand::Slots));
        assert_eq!(
            ok("vehicle", &["set", "exhaust", "akrapovic", "completo"]),
            DevCommand::Vehicle(VehicleCommand::Set { slot: "exhaust".into(), choice: "akrapovic completo".into() })
        );
        assert_eq!(ok("vehicle", &["reset"]), DevCommand::Vehicle(VehicleCommand::Reset));
        assert_eq!(
            ok("vehicle", &["group", "road", "off"]),
            DevCommand::Vehicle(VehicleCommand::Group { name: "road".into(), on: Some(false) })
        );
        assert_eq!(
            ok("vehicle", &["group", "road", "stock"]),
            DevCommand::Vehicle(VehicleCommand::Group { name: "road".into(), on: None })
        );
        assert_eq!(
            ok("vehicle", &["color", "#FF8000"]),
            DevCommand::Vehicle(VehicleCommand::Color(Some([255, 128, 0])))
        );
        assert_eq!(ok("vehicle", &["colour", "stock"]), DevCommand::Vehicle(VehicleCommand::Color(None)));
        assert!(parse("vehicle", &["set", "exhaust"]).unwrap().is_err());
        assert!(parse("vehicle", &["color", "red"]).unwrap().is_err());
        assert!(parse("vehicle", &["group", "road", "maybe"]).unwrap().is_err());
        assert!(parse("vehicle", &["nonsense"]).unwrap().is_err());
    }

    #[test]
    fn gear_commands() {
        assert_eq!(ok("gear", &[]), DevCommand::Gear(GearCommand::List));
        assert_eq!(ok("gear", &["stock"]), DevCommand::Gear(GearCommand::Stock));
        assert_eq!(
            ok("gear", &["set", "helmet", "agv", "corsa"]),
            DevCommand::Gear(GearCommand::Set { part: Part::Helmet, choice: "agv corsa".into() })
        );
        assert!(parse("gear", &["set", "hat", "1"]).unwrap().is_err());
        assert!(parse("gear", &["set", "helmet"]).unwrap().is_err());
    }

    #[test]
    fn devtools_commands() {
        assert_eq!(ok("devtools", &[]), DevCommand::Devtools(DevtoolsCommand::Open(Panel::Menu)));
        assert_eq!(ok("devtools", &["vehicle"]), DevCommand::Devtools(DevtoolsCommand::Open(Panel::Vehicle)));
        assert_eq!(ok("devtools", &["close"]), DevCommand::Devtools(DevtoolsCommand::Close));
        assert_eq!(ok("devtools", &["press", "a"]), DevCommand::Devtools(DevtoolsCommand::Press(Button::A)));
        assert!(parse("devtools", &["press", "x"]).unwrap().is_err());
        assert!(parse("devtools", &["nothing"]).unwrap().is_err());
        assert!(parse("car", &[]).is_none());
    }
}
