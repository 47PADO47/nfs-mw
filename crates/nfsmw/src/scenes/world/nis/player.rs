//! The NIS player's state machine: which scene plays, how far it has got, and how it ended. It advances on the caller's
//! real frame time and knows nothing of the world. The camera is in `camera.rs`, the console in `command.rs`, and the
//! world's side (the freeze, the skip, the bars) in `world.rs`. Spec: `docs/specs/nis-player.md`, `nis-player-2.md`.

use blackbox_chunk::ice::{CameraSample, Track};
use blackbox_nis::{Playback, PlaybackState, Scene, SceneClock};
use game_install::GameDir;
use nfsmw_data::nis::{CameraLibrary, NisBundle};

use super::camera;
use crate::scenes::world::scene_space::{Anchor, lens_of};
use crate::viewer::camera::Lens;

/// Frames per second of a scene's clock (`docs/specs/nis-scenes.md`, section 0).
pub const SCENE_FPS: f32 = 30.0;

/// The kind of a scene. It decides whether the player may skip the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NisKind {
    /// A race start or an intro.
    Intro,
    /// A police arrest. The player cannot skip it.
    Arrest,
    /// Any other scene: a replay, a finish, a free-roam scene.
    Outro,
    /// A cinematic moment: a camera on a collapse.
    Moment,
    /// An ending. The player cannot skip it.
    Ending,
}

impl NisKind {
    /// Whether the player may skip a scene of this kind: arrests and endings never are.
    pub fn skippable(self) -> bool {
        !matches!(self, Self::Arrest | Self::Ending)
    }

    /// The kind a scene name implies, for the console: by its prefix (Arrest, Ending, Intro, WM for a moment), without
    /// regard to case. Any other name is an [`NisKind::Outro`].
    pub fn of_scene(name: &str) -> Self {
        const PREFIXES: [(&str, NisKind); 4] = [
            ("arrest", NisKind::Arrest),
            ("ending", NisKind::Ending),
            ("intro", NisKind::Intro),
            ("wm", NisKind::Moment),
        ];
        let lower = name.to_ascii_lowercase();
        PREFIXES.iter().find(|(prefix, _)| lower.starts_with(prefix)).map_or(NisKind::Outro, |&(_, kind)| kind)
    }
}

/// How a scene ended. A scene that is stopped from the console counts as skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NisOutcome {
    /// The scene ran to its end.
    Completed,
    /// The scene ended early: the player skipped it, or it was stopped.
    Skipped,
}

/// What a scene needs before it can play: its clock, its camera, where it stands and its kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub name: String,
    pub kind: NisKind,
    pub clock: SceneClock,
    pub anchor: Anchor,
    /// The camera track, or `None` for a scene with no camera set (a slow orbit plays instead).
    pub camera: Option<Track>,
}

impl Plan {
    /// The plan of the scene `name` with the header `scene` (its frame window) and its camera. `None` when the scene has
    /// neither a window nor a camera track to give it a length.
    pub fn new(name: &str, scene: &Scene, camera: Option<Track>, anchor: Anchor, kind: NisKind) -> Option<Self> {
        let clock = clock_of(scene, camera.as_ref())?;
        Some(Self { name: name.to_owned(), kind, clock, anchor, camera })
    }
}

/// The clock of a scene: its header's frame window at 30 frames per second. A window with no length takes the length
/// of the camera track instead, counted from frame zero.
fn clock_of(scene: &Scene, camera: Option<&Track>) -> Option<SceneClock> {
    if let Some(clock) = SceneClock::from_scene(scene, SCENE_FPS).filter(|clock| clock.duration() > 0.0) {
        return Some(clock);
    }
    let frames = (camera?.length * SCENE_FPS).round() as i32;
    SceneClock::new(0, frames, SCENE_FPS)
}

/// The scene being played.
struct Playing {
    name: String,
    kind: NisKind,
    anchor: Anchor,
    camera: Option<Track>,
    run: Playback,
}

/// The NIS player: at most one scene plays at a time. The world asks it for the camera, whether the world is frozen,
/// and the letterbox; the console starts and stops scenes.
#[derive(Default)]
pub struct NisPlayer {
    play: Option<Playing>,
    /// How the last scene ended, until the world takes it.
    outcome: Option<NisOutcome>,
    /// The camera jumped since the world last asked (a scene began or ended).
    cut: bool,
    /// The last scene's end, for the console.
    last: Option<String>,
}

impl NisPlayer {
    /// Loads the scene `scene` from the install and starts it at `anchor`. Its camera is track `track`, or track 0 when
    /// the scene's set has no track `track`. The error names what failed.
    pub fn play(
        &mut self,
        dir: &GameDir,
        scene: &str,
        anchor: Anchor,
        kind: NisKind,
        track: u32,
    ) -> Result<(), String> {
        self.refuse_while_playing()?;
        let bundle = NisBundle::load(dir, scene).map_err(|e| format!("{e:#}"))?;
        let library = CameraLibrary::load(dir).map_err(|e| format!("{e:#}"))?;
        let camera = camera_track(&library, &bundle.name, track);
        if camera.is_none() {
            log::info!("NIS {}: no camera set, so a slow orbit plays for the scene", bundle.name);
        }
        let plan = Plan::new(&bundle.name, &bundle.scene, camera, anchor, kind)
            .ok_or_else(|| format!("{scene} has neither a frame window nor a camera track"))?;
        self.start(plan)
    }

    /// Starts a prepared scene. Refused while another plays, or when the scene has no time to play.
    pub fn start(&mut self, plan: Plan) -> Result<(), String> {
        self.refuse_while_playing()?;
        let run = Playback::new(plan.clock, plan.kind.skippable());
        if run.is_finished() {
            return Err(format!("{} has no time to play", plan.name));
        }
        self.outcome = None;
        self.cut = true;
        self.last = Some(format!("{}: playing", plan.name));
        self.play = Some(Playing { name: plan.name, kind: plan.kind, anchor: plan.anchor, camera: plan.camera, run });
        Ok(())
    }

    fn refuse_while_playing(&self) -> Result<(), String> {
        match self.play.as_ref() {
            Some(playing) => Err(format!("NIS {} is playing; stop it first", playing.name)),
            None => Ok(()),
        }
    }

    /// Plays `dt` seconds of real time. The scene ends when its clock runs out.
    pub fn advance(&mut self, dt: f32) {
        let Some(playing) = self.play.as_mut() else { return };
        let finished = playing.run.advance(dt) == PlaybackState::Finished;
        if finished {
            self.end(NisOutcome::Completed);
        }
    }

    /// The player's skip. Returns whether the scene was skipped: a scene that cannot be skipped (an arrest or an
    /// ending) plays on, and nothing is skipped when no scene plays.
    pub fn skip(&mut self) -> bool {
        let Some(playing) = self.play.as_mut() else { return false };
        if !playing.run.skip() {
            return false;
        }
        self.end(NisOutcome::Skipped);
        true
    }

    /// Ends the scene now, whatever its kind. Returns whether a scene was playing.
    pub fn stop(&mut self) -> bool {
        if self.play.is_none() {
            return false;
        }
        self.end(NisOutcome::Skipped);
        true
    }

    fn end(&mut self, outcome: NisOutcome) {
        let Some(playing) = self.play.take() else { return };
        self.last = Some(format!("{}: {outcome:?}", playing.name));
        self.outcome = Some(outcome);
        self.cut = true;
    }

    /// How the last scene ended, once: the next call gives `None` until another scene ends.
    pub fn take_outcome(&mut self) -> Option<NisOutcome> {
        self.outcome.take()
    }

    /// Whether the camera jumped since this was last called: a scene began or ended.
    pub fn take_cut(&mut self) -> bool {
        std::mem::take(&mut self.cut)
    }

    /// Whether a scene plays. The world freezes, hides its HUD and gives the camera to the scene while it does.
    pub fn is_playing(&self) -> bool {
        self.play.is_some()
    }

    /// The camera sample of the scene now, from its track. `None` with no track, or when the track has no sample.
    fn sample(&self) -> Option<CameraSample> {
        let playing = self.play.as_ref()?;
        camera::track_sample(playing.camera.as_ref()?, playing.run.scene_time())
    }

    /// The camera of the scene now, `aspect` being width over height. A scene whose track gives no camera (no track, or
    /// a point in a space that is not placed) plays the slow orbit instead.
    pub fn lens(&self, aspect: f32) -> Option<Lens> {
        let playing = self.play.as_ref()?;
        let track = self.sample().and_then(|sample| lens_of(&playing.anchor, &sample, aspect));
        let fraction = playing.run.elapsed() / playing.run.clock().duration();
        Some(track.unwrap_or_else(|| camera::orbit(&playing.anchor, fraction, aspect)))
    }

    /// The letterbox bar height as a fraction of the screen at the scene's time; 0 when the scene has no bars.
    pub fn letterbox(&self) -> f32 {
        self.sample().map_or(0.0, |sample| sample.letterbox)
    }

    /// One line for the console.
    pub fn status(&self) -> String {
        let now = match &self.play {
            Some(playing) => format!(
                "playing {} ({:?}, {} camera) {:.1} of {:.1} s",
                playing.name,
                playing.kind,
                if playing.camera.is_some() { "track" } else { "orbit" },
                playing.run.elapsed(),
                playing.run.clock().duration()
            ),
            None => "none playing".to_owned(),
        };
        format!("NIS: {now}; last: {}", self.last.as_deref().unwrap_or("none"))
    }
}

/// The camera of a scene: its track `track`, or track 0 when its set has no such track. `None` when the scene has no
/// camera set at all.
fn camera_track(library: &CameraLibrary, scene: &str, track: u32) -> Option<Track> {
    match library.track(scene, track as usize) {
        Some(found) => Some(found.clone()),
        None if track == 0 => None,
        None => {
            log::warn!("NIS {scene}: no camera track {track}; using track 0");
            library.track(scene, 0).cloned()
        }
    }
}

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
