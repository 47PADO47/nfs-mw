//! The world's side of a text message in free roam (`docs/specs/story-flow-2.md` section 3.7): the console command that
//! sends a message the way `EAddSMS` does, for checking the message screen. The flush of [`super::phone`] delivers it on
//! the next free frame and asks the front end to open it.

use nfsmw_career::phone::{AddOutcome, is_known};

use crate::scenes::world::WorldScene;

/// `career sms <id>`: queues text message `id` (`EAddSMS`). Replies with what happened to it.
pub(in crate::scenes::world) fn command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    const USAGE: &str = "usage: career sms <id>";
    let [sub, id] = args else { return Err(USAGE.into()) };
    if *sub != "sms" {
        return Err(USAGE.into());
    }
    let id: u32 = id.parse().map_err(|_| USAGE.to_string())?;
    if !is_known(id) {
        return Err(format!("there is no text message {id}"));
    }
    let phone = &mut scene.career.phone;
    let outcome = scene.career.handle.with(|career| phone.add_sms(&mut career.settings.sms, id));
    let Some(outcome) = outcome else { return Err("no career is being played".into()) };
    Ok(match outcome {
        AddOutcome::Queued => format!("message {id} is queued and opens on the next free frame"),
        AddOutcome::AlreadyPending => format!("message {id} is already queued"),
        AddOutcome::Ignored => format!("message {id} is already in the inbox"),
        AddOutcome::Delivered { .. } => format!("message {id} is in the inbox"),
    })
}
