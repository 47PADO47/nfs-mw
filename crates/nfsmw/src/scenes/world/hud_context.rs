//! The world's part of the Custom HUD's context (`hud/state/context.rs`): where the player is, the race's stage and its
//! intro, the entry card, the speed-trap points, the end of the last chase and a mod's line. Each is read from the same
//! state the world's egui cards draw from (`scene_hud.rs`), so the two looks show the same thing.

use nfsmw_data::gameplay::RaceType;

use super::WorldScene;
use super::race::Stage;
use crate::hud::{EntryPrompt, HudContext, InteractPopup, RaceIntro, RaceStage, entry_action};
use crate::i18n::t;

impl WorldScene {
    /// The context of this frame.
    pub(super) fn hud_context(&self) -> HudContext {
        HudContext {
            player: self.players.lead().map(|drive| {
                let at = drive.pose().position;
                [at.x, at.y]
            }),
            race_stage: self.race.as_ref().map(|session| race_stage(&session.stage)),
            race_intro: self.hud_race_intro(),
            race_points: self.race_card().and_then(|card| card.points),
            entry: entry_prompt(&self.entry.popup),
            pursuit_end: self.pursuit_end(),
            mod_message: crate::hud::mod_message_line(),
        }
    }

    /// What the intro card of the race says: the track's name where the language has one, the type, the laps of a
    /// lapped race and the opponents' names. `None` outside a race.
    fn hud_race_intro(&self) -> Option<RaceIntro> {
        let session = self.race.as_ref()?;
        let laps = session.engine.as_ref().map(|engine| engine.course().laps);
        let lapped = matches!(session.kind, RaceType::Circuit | RaceType::Knockout) || laps.is_some_and(|n| n > 1);
        let opponents = session.plan.iter().skip(1).map(|plan| plan.name.clone()).filter(|name| !name.is_empty());
        Some(RaceIntro {
            name: track_name(&session.event),
            kind: session.kind,
            laps: laps.filter(|_| lapped).map(|n| n.max(1)),
            opponents: opponents.collect(),
        })
    }
}

/// The track's name for the career event id `event` (`10.2.1` reads `TRACKNAME_10_2_1`), as the front end names
/// tracks; `None` when the language has no such label.
fn track_name(event: &str) -> Option<String> {
    let key = format!("TRACKNAME_{}", event.replace('.', "_"));
    crate::i18n::game(blackbox_text::label_hash(&key))
}

/// The stage of a race as the HUD lays itself out for it.
fn race_stage(stage: &Stage) -> RaceStage {
    match stage {
        Stage::Loading { .. } | Stage::Intro { .. } | Stage::Staged { .. } => RaceStage::Intro,
        Stage::Running => RaceStage::Running,
        Stage::Results { .. } | Stage::PostRace { .. } | Stage::Outro { .. } => RaceStage::Results,
    }
}

/// The entry card while it is up: its text, what Interact does there, and its fade.
fn entry_prompt(popup: &InteractPopup) -> Option<EntryPrompt> {
    let prompt = popup.drawn().filter(|_| popup.visible())?;
    let action = entry_action(&prompt.line, popup.is_flashing(), &t!("hud-entry-go-in"), &t!("hud-entry-leave"));
    Some(EntryPrompt { title: prompt.title.clone(), line: prompt.line.clone(), action, opacity: popup.opacity() })
}
