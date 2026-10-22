//! The `coop` console command: a co-op session without the menus, for tests, offscreen runs and power users.
//! `coop on <CAR[:PRESET]> [arrows|pad]` brings player 2 in behind the lead, `coop off` takes them out and `coop` says who
//! plays. Player 2 drives on the arrow block (the default) or on the first pad connected (`pad`).

use blackbox_gfx::RenderBackend;

use super::super::WorldScene;
use super::LEAD;
use super::coop::{CoopSetup, GUEST};
use crate::input::CoopDevice;
use crate::settings::Transmission;

const USAGE: &str = "usage: coop on <CAR[:PRESET]> [arrows|pad] | coop off | coop";

/// What the console line asks for.
#[derive(Debug, Clone, PartialEq)]
enum CoopCommand {
    /// Bring player 2 in with this car, on this device.
    On { car: String, device: CoopDevice },
    /// Take player 2 out.
    Off,
    /// Say who plays.
    Status,
}

/// The arguments after `coop`, read into a command; anything else gives the usage line.
fn parse(args: &[&str]) -> Result<CoopCommand, String> {
    match args {
        [] => Ok(CoopCommand::Status),
        ["off"] => Ok(CoopCommand::Off),
        ["on", car, rest @ ..] => {
            let device = match rest {
                [] | ["arrows"] => CoopDevice::Arrows,
                ["pad"] => CoopDevice::Pad,
                _ => return Err(USAGE.into()),
            };
            Ok(CoopCommand::On { car: (*car).to_owned(), device })
        }
        _ => Err(USAGE.into()),
    }
}

/// `coop` in the world's console: co-op is started, stopped or described.
pub(in crate::scenes::world) fn coop(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
    match parse(args)? {
        CoopCommand::Status => Ok(status(scene)),
        CoopCommand::Off => {
            scene.end_coop(renderer);
            Ok("co-op off".into())
        }
        CoopCommand::On { car, device } => {
            if !scene.players.has_lead() {
                return Err("not driving (use the drive command first)".into());
            }
            // A session already running is replaced: the guest is taken out before the new one joins.
            scene.end_coop(renderer);
            let setup = CoopSetup {
                car: car.clone(),
                transmission: Transmission::Automatic,
                alias: alias(),
                car_handle: None,
                device,
            };
            scene.queue_coop(setup);
            Ok(format!("co-op queued: {car} for player 2, on {}", device_name(device)))
        }
    }
}

/// Who plays, and on what device player 2 drives; "co-op off" outside co-op.
fn status(scene: &WorldScene) -> String {
    let Some(session) = scene.coop.as_ref() else {
        return "co-op off (coop on <CAR[:PRESET]> [arrows|pad] starts it)".into();
    };
    let lead = scene.player_name(LEAD).unwrap_or("player 1");
    let guest = scene.player_name(GUEST).unwrap_or("player 2");
    format!("co-op: {lead} and {guest}; {guest} on {}", device_name(session.device))
}

/// The alias the two players are named after: the loaded profile's, or `Player` with none loaded.
fn alias() -> String {
    crate::career::CareerHandle::shared().with_loaded(|career| career.alias.clone()).unwrap_or_else(|| "Player".into())
}

fn device_name(device: CoopDevice) -> &'static str {
    match device {
        CoopDevice::Arrows => "the arrow block",
        CoopDevice::Pad => "a pad",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coop_on_parses_car_and_device() {
        assert_eq!(
            parse(&["on", "CRUISER"]),
            Ok(CoopCommand::On { car: "CRUISER".into(), device: CoopDevice::Arrows }),
            "the arrow block is the default"
        );
        assert_eq!(
            parse(&["on", "CRUISER:Stock", "pad"]),
            Ok(CoopCommand::On { car: "CRUISER:Stock".into(), device: CoopDevice::Pad })
        );
    }

    #[test]
    fn coop_off_parses() {
        assert_eq!(parse(&["off"]), Ok(CoopCommand::Off));
        assert_eq!(parse(&[]), Ok(CoopCommand::Status));
    }

    #[test]
    fn bad_args_give_usage() {
        let bad: [&[&str]; 4] =
            [&["on"], &["on", "CRUISER", "keyboard"], &["on", "CRUISER", "pad", "extra"], &["bogus"]];
        for args in bad {
            assert_eq!(parse(args), Err(USAGE.to_owned()), "{args:?}");
        }
    }
}
