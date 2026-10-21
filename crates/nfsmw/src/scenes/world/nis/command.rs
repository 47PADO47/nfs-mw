//! The `nis` console command: list the NIS scenes of the install, play one from the player's car, stop it and show its
//! state.

use nfsmw_data::nis::NisBundle;

use crate::scenes::world::WorldScene;

const USAGE: &str = "usage: nis list | nis play <scene> [track] | nis stop | nis status";

/// One parsed `nis` command. The scene name is borrowed from the arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NisCommand<'a> {
    List,
    Play { scene: &'a str, track: u32 },
    Stop,
    Status,
}

/// Parses the arguments after `nis`. A play with no track asks for track 0.
pub fn parse<'a>(args: &[&'a str]) -> Result<NisCommand<'a>, String> {
    match args {
        ["list"] => Ok(NisCommand::List),
        ["play", scene] => Ok(NisCommand::Play { scene, track: 0 }),
        ["play", scene, track] => {
            let track = track.parse::<u32>().map_err(|_| format!("{track:?} is not a track number ({USAGE})"))?;
            Ok(NisCommand::Play { scene, track })
        }
        ["stop"] => Ok(NisCommand::Stop),
        ["status"] => Ok(NisCommand::Status),
        _ => Err(USAGE.into()),
    }
}

/// Runs `nis ...` in the world.
pub(in crate::scenes::world) fn nis_command(world: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    match parse(args)? {
        NisCommand::List => Ok(list(world)),
        NisCommand::Play { scene, track } => {
            world.play_nis(scene, track)?;
            Ok(format!("playing NIS {scene}, camera track {track}"))
        }
        NisCommand::Stop => match world.stop_nis() {
            true => Ok("the NIS scene is stopped".into()),
            false => Err("no NIS scene is playing".into()),
        },
        NisCommand::Status => Ok(world.nis_status()),
    }
}

fn list(world: &WorldScene) -> String {
    let names = NisBundle::list(&world.dir);
    format!("{} NIS scenes: {}", names.len(), names.join(" "))
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
