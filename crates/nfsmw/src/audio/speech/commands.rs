//! The `speech` console command.

use super::{Outcome, Param, Speech, SpeechParams};
use crate::audio::Audio;

/// Usage line, for the error of a mistyped command.
const USAGE: &str = "speech [status|flow|list|say <event> [speaker] [param=n ...]|play <event> [speaker] [param=n ...]|     bank <n> [take]|stop|reset]; params are named as in the game (car_color, direction, ...), n a bit value";

/// Run `speech <args>`.
pub fn command(audio: &mut Audio, args: &[&str]) -> Result<String, String> {
    let device = if audio.available() { "" } else { " (no sound device)" };
    if let ["flow"] = args {
        return Ok(audio.chase_flow_status());
    }
    let speech = audio.speech()?;
    match args {
        [] | ["status"] => Ok(status(speech, device)),
        ["list"] => Ok(list(speech)),
        ["say", event, rest @ ..] => {
            let id = resolve(speech, event)?;
            let (speaker, params) = request_of(rest)?;
            let name = speech.dispatcher().event(id).map(|e| e.name.clone()).unwrap_or_default();
            Ok(match speech.request_with(id, speaker, params) {
                Outcome::Queued => format!("{name} is queued{device}"),
                Outcome::Refreshed => format!("{name} was waiting already: its wait starts over"),
                Outcome::Unknown => format!("no speech event {id}"),
                Outcome::Muted => "speech is off (speech_volume is 0)".to_owned(),
                Outcome::Thinned => format!("{name} was dropped at random (pursuit thinning)"),
            })
        }
        ["play", event, rest @ ..] => {
            let id = resolve(speech, event)?;
            let (speaker, params) = request_of(rest)?;
            let secs = speech.play_now(id, speaker, &params)?;
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

/// The speaker (a bare number) and parameters (`name=value`, value decimal or `0x` hex) after an event.
fn request_of(args: &[&str]) -> Result<(u16, SpeechParams), String> {
    let mut speaker = 0;
    let mut params = SpeechParams::new();
    for arg in args {
        let Some((name, value)) = arg.split_once('=') else {
            speaker = arg.parse().map_err(|_| format!("{arg:?} is not a speaker number (1 to 9) or name=value"))?;
            continue;
        };
        let param = Param::from_name(name).ok_or_else(|| format!("{name:?} is not a speech parameter"))?;
        let parsed = match value.strip_prefix("0x") {
            Some(hex) => u32::from_str_radix(hex, 16),
            None => value.parse(),
        };
        params.set(param, parsed.map_err(|_| format!("{value:?} is not a number"))?);
    }
    Ok((speaker, params))
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
            let voice = if speech.lines().says(e.id) { "voice" } else { "     " };
            let how = if e.interrupt { "interrupt" } else { "         " };
            format!("{:4} {voice} {how} p{:<3} {}", e.id, e.priority, e.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
