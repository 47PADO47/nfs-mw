//! The `story` console command: the story flow's state, and the entries that test it without playing to them. `story
//! status` shows the stage, the prologue's resume point, the current bin, the rival beaten flag, the final race on a
//! resume and the career completion. `story race <event id>` starts any career race with its story intro. `story
//! arrival` marks the current rival beaten and asks for the safe house arrival, and `story firstvisit` asks for the
//! stage-16 safe house intro (`docs/specs/story-flow.md` sections 2.5 and 2.7; `docs/specs/story-flow-2.md` sections 4.6
//! and 5).

use nfsmw_career::settings::{Flags, NodeStates};
use nfsmw_career::story::final_resume::resume_final_race;
use nfsmw_career::story::prologue::{PrologueResume, resume_point};

use crate::career::CareerRequest;
use crate::scenes::world::WorldScene;
use crate::scenes::world::race::RaceOptions;

const USAGE: &str = "usage: story status | story race <event id> | story arrival | story firstvisit | story carlot | story cash <amount>";

/// The safe house zone the console's requests name: the same one a safe house entry names.
const SAFE_HOUSE_ZONE: &str = "menu_gate_safehouse/zone_safehouse_1";

/// One `story` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StoryCommand {
    Status,
    /// A career race by its event id (`16.1.0`, `15.1.2`, `1.8.1`).
    Race(String),
    Arrival,
    FirstVisit,
    /// The first car's lot, as the end of the scripted chase opens it.
    CarLot,
    /// Sets the career's cash, to test a purchase without playing for the money.
    Cash(u32),
}

/// Reads the arguments of `story ...`.
pub(super) fn parse(args: &[&str]) -> Result<StoryCommand, String> {
    match args {
        ["status"] => Ok(StoryCommand::Status),
        ["race", event] => Ok(StoryCommand::Race((*event).to_owned())),
        ["arrival"] => Ok(StoryCommand::Arrival),
        ["firstvisit"] => Ok(StoryCommand::FirstVisit),
        ["carlot"] => Ok(StoryCommand::CarLot),
        ["cash", amount] => amount.parse().map(StoryCommand::Cash).map_err(|_| USAGE.to_owned()),
        _ => Err(USAGE.into()),
    }
}

/// Runs `story ...`.
pub(in crate::scenes::world) fn story_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    match parse(args)? {
        StoryCommand::Status => status(scene),
        StoryCommand::Race(event) => scene.start_race(&event, RaceOptions { career: true, ..RaceOptions::default() }),
        StoryCommand::Arrival => arrival(scene),
        StoryCommand::FirstVisit => first_visit(scene),
        StoryCommand::CarLot => car_lot(scene),
        StoryCommand::Cash(amount) => cash(scene, amount),
    }
}

/// The state lines of the career being played.
fn status(scene: &WorldScene) -> Result<String, String> {
    scene
        .career
        .handle
        .with(|career| {
            status_lines(career.stage(), &career.settings.nodes(), career.rival_beaten, career.settings.flags)
        })
        .ok_or_else(|| "no career is played".to_owned())
}

/// The status lines for a career at `stage` (its current bin), with its node states, the rival beaten (if any) and its
/// settings flags.
pub(super) fn status_lines(stage: u8, nodes: &NodeStates, rival_beaten: Option<u8>, flags: Flags) -> String {
    let prologue = match resume_point(stage, nodes, false) {
        PrologueResume::StartAt(event) => event,
        PrologueResume::Done => "none",
    };
    let rival = match rival_beaten {
        Some(bin) => format!("yes, bin {bin}"),
        None => "no".to_owned(),
    };
    [
        format!("stage: {stage} (current bin)"),
        format!("prologue resumes at: {prologue}"),
        format!("rival beaten: {rival}"),
        format!("final race on resume: {}", yes_no(resume_final_race(nodes, flags))),
        format!("career complete: {}", yes_no(flags.contains(Flags::CAREER_COMPLETE))),
    ]
    .join("\n")
}

fn yes_no(value: bool) -> &'static str {
    match value {
        true => "yes",
        false => "no",
    }
}

/// Marks the current bin's rival beaten and asks for the safe house, which runs the arrival sequence there.
fn arrival(scene: &mut WorldScene) -> Result<String, String> {
    let bin = scene
        .career
        .handle
        .with(|career| {
            let bin = career.stage();
            career.rival_beaten = Some(bin);
            bin
        })
        .ok_or("no career is played")?;
    scene.career.requests.push_back(CareerRequest::SafeHouse { zone: SAFE_HOUSE_ZONE.to_owned() });
    Ok(format!("the rival of bin {bin} is beaten; the safe house arrival is requested"))
}

/// Asks for the safe house: at stage 16 that is the first visit's intro, else the safe house menu.
fn first_visit(scene: &mut WorldScene) -> Result<String, String> {
    if !scene.career.handle.is_active() {
        return Err("no career is played".into());
    }
    scene.career.requests.push_back(CareerRequest::SafeHouse { zone: SAFE_HOUSE_ZONE.to_owned() });
    Ok("the safe house is requested: at stage 16 the first visit's intro plays first".into())
}

/// Sets the career's cash.
fn cash(scene: &mut WorldScene, amount: u32) -> Result<String, String> {
    scene
        .career
        .handle
        .with(|career| career.settings.cash = amount)
        .map(|()| format!("the career has {amount} cash"))
        .ok_or_else(|| "no career is played".to_owned())
}

/// Asks for the first car's lot, which the scripted chase's end opens (`story-flow.md` section 2.3).
fn car_lot(scene: &mut WorldScene) -> Result<String, String> {
    if !scene.career.handle.is_active() {
        return Err("no career is played".into());
    }
    scene.career.requests.push_back(CareerRequest::FirstCarLot);
    Ok("the first car's lot is requested".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_subcommands_parse() {
        assert_eq!(parse(&["status"]), Ok(StoryCommand::Status));
        assert_eq!(parse(&["race", "16.1.0"]), Ok(StoryCommand::Race("16.1.0".into())));
        assert_eq!(parse(&["arrival"]), Ok(StoryCommand::Arrival));
        assert_eq!(parse(&["firstvisit"]), Ok(StoryCommand::FirstVisit));
        assert_eq!(parse(&["carlot"]), Ok(StoryCommand::CarLot));
        assert_eq!(parse(&["cash", "30000"]), Ok(StoryCommand::Cash(30_000)));
        assert!(parse(&["cash", "lots"]).is_err());
    }

    #[test]
    fn a_bad_story_command_is_an_error() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["race"]).is_err(), "a race needs its event id");
        assert!(parse(&["race", "1.8.1", "extra"]).is_err());
        assert!(parse(&["status", "now"]).is_err());
        assert!(parse(&["bin", "3"]).is_err());
    }

    #[test]
    fn the_status_of_a_fresh_career_starts_the_prologue_at_its_first_race() {
        let text = status_lines(16, &NodeStates::default(), None, Flags::default());
        assert!(text.contains("stage: 16"), "{text}");
        assert!(text.contains("prologue resumes at: 16.1.0"), "{text}");
        assert!(text.contains("rival beaten: no"), "{text}");
        assert!(text.contains("final race on resume: no"), "{text}");
        assert!(text.contains("career complete: no"), "{text}");
    }

    #[test]
    fn the_status_of_a_finished_blacklist_resumes_the_final_race() {
        let mut nodes = NodeStates::default();
        nodes.set_won_in_career("1.2.3").expect("the list has room");
        let text = status_lines(1, &nodes, Some(2), Flags::default());
        assert!(text.contains("prologue resumes at: none"), "{text}");
        assert!(text.contains("rival beaten: yes, bin 2"), "{text}");
        assert!(text.contains("final race on resume: yes"), "{text}");
    }

    #[test]
    fn a_completed_career_says_so_and_does_not_resume_the_final_race() {
        let mut nodes = NodeStates::default();
        nodes.set_won_in_career("1.2.3").expect("the list has room");
        let text = status_lines(1, &nodes, None, Flags::CAREER_COMPLETE);
        assert!(text.contains("career complete: yes"), "{text}");
        assert!(text.contains("final race on resume: no"), "{text}");
    }
}
