//! The interactive music inside [`Audio`]: loaded when the first pursuit asks for it, updated every frame.

use super::conductor::{Conductor, Stage};
use super::kira_stage::{KiraStage, Unavailable};
use super::pursuit::PursuitFiles;
use super::state::{MusicState, PURSUIT_SETS, Pursuit};
use crate::audio::Audio;

/// The pursuit music's files: not read yet, failed to read, or ready.
#[derive(Default)]
enum Files {
    #[default]
    Unloaded,
    Failed(String),
    Ready(PursuitFiles),
}

/// Usage line for `help`.
const USAGE: &str = "music [status|pursuit <1-4> [0-100]|intensity <0-100>|clear]";

#[derive(Default)]
pub struct Interactive {
    conductor: Conductor,
    files: Files,
    /// A state the `music` console command put in place of the game's.
    forced: Option<MusicState>,
    /// The last state looked at and whether a game was on, for `music status`.
    last: (MusicState, bool),
}

/// The music file and its map, relative to the install.
const MAP: &str = "SOUND/PFDATA/MW_Music.mpf";
const STREAMS: &str = "SOUND/PFDATA/MW_Music.mus";

impl Audio {
    /// Once per frame, before the radio: `driving` is true while a game is on. Starts, steers and ends the pursuit
    /// music for `input` (or for the state the console forced). Returns whether the licensed songs may play.
    pub fn update_interactive(&mut self, input: &MusicState, driving: bool, dt: f32) -> bool {
        let state = self.interactive.forced.unwrap_or(*input);
        if state.pursuit.is_some() && driving && matches!(self.interactive.files, Files::Unloaded) {
            self.interactive.files = self.load_pursuit_files();
        }
        self.interactive.last = (state, driving);
        let Interactive { conductor, files, .. } = &mut self.interactive;
        let mut unavailable;
        let mut kira;
        let stage: &mut dyn Stage = match (self.output.as_mut(), &*files) {
            (Some(out), Files::Ready(files)) => {
                kira = KiraStage { music: &mut out.music, files };
                &mut kira
            }
            (None, _) => {
                unavailable = Unavailable("there is no sound device".into());
                &mut unavailable
            }
            (_, Files::Failed(e)) => {
                unavailable = Unavailable(e.clone());
                &mut unavailable
            }
            (_, _) => {
                unavailable = Unavailable("the pursuit music is not loaded".into());
                &mut unavailable
            }
        };
        conductor.update(stage, &state, driving, dt)
    }

    fn load_pursuit_files(&self) -> Files {
        let load = || {
            let map = self.dir.read(MAP).map_err(|e| format!("{MAP}: {e}"))?;
            let mus = self.dir.resolve(STREAMS).ok_or_else(|| format!("{STREAMS} is not in the install"))?;
            PursuitFiles::new(&map, mus.to_path_buf())
        };
        match load() {
            Ok(files) => Files::Ready(files),
            Err(e) => {
                log::warn!("no pursuit music: {e}");
                Files::Failed(e)
            }
        }
    }
}

/// Run `music <args>`.
pub fn command(audio: &mut Audio, args: &[&str]) -> Result<String, String> {
    let state = &mut audio.interactive;
    match args {
        [] | ["status"] => {
            let (last, driving) = state.last;
            let forced =
                if state.forced.is_some() { "\n  the console holds this state (music clear releases it)" } else { "" };
            let files = match &state.files {
                Files::Failed(e) => format!("\n  the music files did not load: {e}"),
                _ => String::new(),
            };
            Ok(format!("{}{forced}{files}", state.conductor.status(&last, driving)))
        }
        ["clear"] => {
            state.forced = None;
            Ok("music: the game's state is back in charge".into())
        }
        ["pursuit", set] | ["pursuit", set, _] => {
            let set: u8 = set.parse().ok().filter(|s| (1..=PURSUIT_SETS).contains(s)).ok_or("the set is 1 to 4")?;
            let intensity = match args.get(2) {
                Some(value) => percent(value)?,
                None => 0.5,
            };
            state.forced = Some(MusicState { pursuit: Some(Pursuit::new(set, intensity)), racing: false });
            Ok(format!("music: pursuit set {set} at intensity {intensity:.2} (in a game; music clear ends it)"))
        }
        ["intensity", value] => {
            let intensity = percent(value)?;
            let Some(pursuit) = state.forced.as_mut().and_then(|s| s.pursuit.as_mut()) else {
                return Err("no forced pursuit (music pursuit <1-4> first)".into());
            };
            *pursuit = Pursuit::new(pursuit.set, intensity);
            Ok(format!("music: intensity {intensity:.2}"))
        }
        _ => Err(format!("usage: {USAGE}")),
    }
}

/// `0` to `100` as 0 to 1.
pub(super) fn percent(text: &str) -> Result<f32, String> {
    let value: f32 = text.trim_end_matches('%').parse().map_err(|_| format!("{text:?} is not a percentage"))?;
    if !(0.0..=100.0).contains(&value) {
        return Err("the intensity is 0 to 100".into());
    }
    Ok(value / 100.0)
}
