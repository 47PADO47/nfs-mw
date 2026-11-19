//! The audio trace's record of each front-end cue decision (`fe_cue`): which screen and message asked, whether a gate
//! suppressed it, the voice it started and the level of its bus at that moment (`docs/testing.md`).
use std::fmt::Write as _;

use super::super::Audio;
use super::super::trace::Record;

/// Where a front-end sound message was decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CueNote {
    /// The package (screen file) that sent the message.
    pub screen: String,
    pub message: u32,
    /// `authored` for a package script or response, `input` for the host's pad feedback.
    pub source: &'static str,
    /// The trigger before the screen gates.
    pub candidate: Option<i32>,
    /// The gate that suppressed the cue; `None` when it was requested.
    pub gate: Option<String>,
}

/// The front-end effects bus as the mixer sets it: master times effects, zero while a movie mutes the cues.
fn sfx_bus(audio: &Audio) -> f32 {
    if audio.movie_active {
        return 0.0;
    }
    audio.volumes.master * audio.volumes.sfx
}

fn detail(audio: &Audio, note: Option<&CueNote>, result: &str) -> String {
    let mut out = String::new();
    match note {
        Some(note) => {
            let _ =
                write!(out, "screen={} message={:08X} source={} candidate=", note.screen, note.message, note.source);
            match note.candidate {
                Some(id) => {
                    let _ = write!(out, "{id}");
                }
                None => out.push_str("none"),
            }
        }
        None => out.push_str("screen=- message=- source=direct candidate=-"),
    }
    let _ = write!(
        out,
        " {result} bus=sfx master={:.3} sfx={:.3} effective={:.3} movie_active={} world_muted={}",
        audio.volumes.master,
        audio.volumes.sfx,
        sfx_bus(audio),
        audio.movie_active,
        audio.world_muted
    );
    out
}

pub(super) fn played(audio: &Audio, id: i32, note: Option<&CueNote>, voice: &str) {
    if !audio.trace.enabled() {
        return;
    }
    let started = voice.starts_with("started");
    let detail = detail(audio, note, &format!("result=played voice={voice}"));
    audio.trace.record(Record {
        event: "fe_cue",
        trigger: Some(id),
        group: Some("Sfx"),
        success: Some(started),
        detail: &detail,
        logical_ms: audio.trace.now_ms(),
        ..Default::default()
    });
}

pub(super) fn suppressed(audio: &Audio, note: &CueNote) {
    if !audio.trace.enabled() {
        return;
    }
    let gate = note.gate.as_deref().unwrap_or("?");
    let detail = detail(audio, Some(note), &format!("result=suppressed gate={gate:?}"));
    audio.trace.record(Record {
        event: "fe_cue",
        trigger: note.candidate,
        group: Some("Sfx"),
        success: Some(false),
        detail: &detail,
        logical_ms: audio.trace.now_ms(),
        ..Default::default()
    });
}
