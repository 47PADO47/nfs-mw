//! The `speech` console command.

use super::{Outcome, Speech};
use crate::audio::Audio;

/// Usage line, for the error of a mistyped command.
const USAGE: &str = "speech [status|list|say <event> [speaker]|play <event> [speaker]|bank <n> [take]|stop|reset]";

/// Run `speech <args>`.
pub fn command(audio: &mut Audio, args: &[&str]) -> Result<String, String> {
    let device = if audio.available() { "" } else { " (no sound device)" };
    let speech = audio.speech()?;
    match args {
        [] | ["status"] => Ok(status(speech, device)),
        ["list"] => Ok(list(speech)),
        ["say", event] | ["say", event, _] => {
            let id = resolve(speech, event)?;
            let speaker = speaker_of(args.get(2))?;
            let name = speech.dispatcher().event(id).map(|e| e.name.clone()).unwrap_or_default();
            Ok(match speech.request(id, speaker) {
                Outcome::Queued => format!("{name} is queued{device}"),
                Outcome::Refreshed => format!("{name} was waiting already: its wait starts over"),
                Outcome::Unknown => format!("no speech event {id}"),
                Outcome::Muted => "speech is off (speech_volume is 0)".to_owned(),
                Outcome::Thinned => format!("{name} was dropped at random (pursuit thinning)"),
            })
        }
        ["play", event] | ["play", event, _] => {
            let id = resolve(speech, event)?;
            let secs = speech.play_now(id, speaker_of(args.get(2))?)?;
            Ok(format!("playing {event} ({secs:.2} s){device}"))
        }
        ["bank", bank] | ["bank", bank, _] => bank_command(speech, bank, args.get(2), device),
        ["stop"] => {
            speech.silence();
            Ok("speech stopped, queue cleared".into())
        }
        ["reset"] => {
            speech.dispatcher_mut().reset();
            Ok("speech history and queue cleared".into())
        }
        _ => Err(format!("usage: {USAGE}")),
    }
}

fn speaker_of(arg: Option<&&str>) -> Result<u16, String> {
    let Some(arg) = arg else { return Ok(0) };
    arg.parse().map_err(|_| format!("{arg:?} is not a speaker number (1 to 9)"))
}

/// An event by name or number.
fn resolve(speech: &Speech, arg: &str) -> Result<u32, String> {
    if let Ok(id) = arg.parse::<u32>() {
        return Ok(id);
    }
    speech.dispatcher().find(arg).map(|e| e.id).ok_or_else(|| format!("no speech event named {arg:?} (speech list)"))
}

fn bank_command(speech: &mut Speech, bank: &str, take: Option<&&str>, device: &str) -> Result<String, String> {
    let bank: usize = bank.parse().map_err(|_| format!("{bank:?} is not a bank number"))?;
    let Some(take) = take else {
        return Ok(format!("{}\nbank <n> <take> plays a recording", speech.lines().summary()));
    };
    let take: usize = take.parse().map_err(|_| format!("{take:?} is not a take number"))?;
    let secs = speech.play_take(bank, take)?;
    Ok(format!("playing bank {bank} take {take} ({secs:.2} s){device}"))
}

fn status(speech: &Speech, device: &str) -> String {
    let d = speech.dispatcher();
    let name_of = |id: Option<u32>| id.and_then(|id| d.event(id)).map_or("nothing".to_owned(), |e| e.name.clone());
    let current = name_of(d.current());
    let heard = name_of(d.history().last_event());
    let error = speech.last_error().map(|e| format!("\n  last problem: {e}")).unwrap_or_default();
    format!(
        "speech{device}: {} events, {}\n  waiting {}, on the air: {current}, last heard: {heard}, said {} so far, \
         keeps {:.0}% of repeats{error}",
        d.events().len(),
        speech.lines().summary(),
        d.waiting(),
        speech.said(),
        d.keep_probability() * 100.0,
    )
}

fn list(speech: &Speech) -> String {
    let d = speech.dispatcher();
    d.events()
        .iter()
        .map(|e| {
            let voice = if speech.lines().has(e.id) { "voice" } else { "     " };
            let how = if e.interrupt { "interrupt" } else { "         " };
            format!("{:4} {voice} {how} p{:<3} {}", e.id, e.priority, e.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
