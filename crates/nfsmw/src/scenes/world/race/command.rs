//! The `race` console command: list the races, start one, see the state, stop it.

use super::setup::RaceOptions;
use crate::scenes::world::WorldScene;

const USAGE: &str = "usage: race list [filter] | race start <race> [laps=N] [opponents=N] [difficulty=N] [traffic=N] [split=CAR] | race status | race stop | race mod [demo]";

/// Runs `race ...`.
pub(in crate::scenes::world) fn race_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    match args {
        ["list"] => list(scene, ""),
        ["list", filter] => list(scene, filter),
        ["status"] => Ok(scene.race_status()),
        ["mod", rest @ ..] => scene.try_mod_race(rest.contains(&"demo")),
        ["stop"] => match scene.abandon_race() {
            true => Ok("the race is over".into()),
            false => Err("no race is running".into()),
        },
        ["start", query, rest @ ..] => {
            let (split, rest): (Vec<&str>, Vec<&str>) = rest.iter().partition(|arg| arg.starts_with("split="));
            let options = parse_options(&rest)?;
            if let Some(setup) = split.first().map(|arg| split_setup(arg, scene)).transpose()? {
                scene.queued_split = Some(setup);
            }
            scene.start_race(query, options)
        }
        _ => Err(USAGE.into()),
    }
}

/// `split=CAR`: a split-screen race with the player's car and `CAR` for the second player, the keyboard and its arrow block.
/// The input seats are the front end's (`SplitInput`); the console starts the race only, for tests and screenshots.
fn split_setup(arg: &str, scene: &WorldScene) -> Result<super::SplitSetup, String> {
    let car = arg.trim_start_matches("split=").trim();
    if car.is_empty() {
        return Err("split= needs the second player's car, e.g. split=PORSCHE911".into());
    }
    let first =
        scene.drive.as_ref().map_or_else(|| crate::scenes::world::DEFAULT_CAR.to_owned(), |d| d.car_name.clone());
    Ok(super::SplitSetup {
        cars: [first, car.to_owned()],
        devices: [crate::input::PlayerDevice::Keyboard, crate::input::PlayerDevice::KeyboardArrows],
        transmissions: [crate::settings::Transmission::Automatic; 2],
    })
}

/// `laps=3 opponents=2 difficulty=66 traffic=0` and `career`.
fn parse_options(args: &[&str]) -> Result<RaceOptions, String> {
    let mut options = RaceOptions::default();
    for arg in args {
        let (key, value) = arg.split_once('=').unwrap_or((arg, ""));
        let number = |name: &str| value.parse::<i32>().map_err(|_| format!("{name} needs a number, got {value:?}"));
        match key {
            "laps" => options.laps = Some(u32::try_from(number("laps")?).map_err(|_| "laps must be positive")?),
            "opponents" => {
                options.opponents =
                    Some(usize::try_from(number("opponents")?).map_err(|_| "opponents must be 0 or more")?)
            }
            "difficulty" => options.difficulty = Some(number("difficulty")?.clamp(0, 100)),
            "traffic" => options.traffic = Some(number("traffic")?.clamp(0, 100)),
            "career" => options.career = true,
            "demo" => options.demo = true,
            _ => return Err(format!("unknown option {arg:?}; {USAGE}")),
        }
    }
    Ok(options)
}

fn list(scene: &WorldScene, filter: &str) -> Result<String, String> {
    let data = scene.gameplay.as_ref().ok_or("the race data could not be read from the install")?;
    let filter = filter.to_ascii_lowercase();
    let lines: Vec<String> = data
        .races
        .iter()
        .filter(|r| super::startable::can_start(r) && r.name.to_ascii_lowercase().contains(&filter))
        .map(|r| {
            let plural = |n: usize| if n == 1 { "" } else { "s" };
            format!(
                "{} ({}, {} lap{}, {} checkpoint{})",
                r.name,
                r.event_icon,
                r.laps,
                plural(r.laps.max(0) as usize),
                r.checkpoints.len(),
                plural(r.checkpoints.len())
            )
        })
        .collect();
    match lines.is_empty() {
        true => Err("no race matches".into()),
        false => Ok(format!("{} race(s)\n{}", lines.len(), lines.join("\n"))),
    }
}

impl WorldScene {
    /// One line about the race being run.
    pub(in crate::scenes::world) fn race_status(&self) -> String {
        let Some(session) = self.race.as_ref() else { return "no race is running".into() };
        let Some(engine) = session.engine.as_ref() else { return format!("{}: getting to the grid", session.name) };
        let places = engine.places();
        format!(
            "{}: {:?}, {:.1} s, place {}/{}, {} lap(s), phase {:?}",
            session.name,
            session.kind,
            engine.elapsed(),
            places[0],
            places.len(),
            engine.course().laps,
            engine.phase()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse_from_key_value_pairs() {
        let options = parse_options(&["laps=2", "opponents=1", "difficulty=250", "traffic=0", "career"]).unwrap();
        assert_eq!(options.laps, Some(2));
        assert_eq!(options.opponents, Some(1));
        assert_eq!(options.difficulty, Some(100), "clamped");
        assert_eq!(options.traffic, Some(0));
        assert!(options.career);
        assert!(parse_options(&["demo"]).unwrap().demo);
    }

    #[test]
    fn a_bad_option_is_an_error() {
        assert!(parse_options(&["laps=x"]).is_err());
        assert!(parse_options(&["laps=-1"]).is_err());
        assert!(parse_options(&["speed=3"]).is_err());
    }
}
