//! The NIS player's state machine: which scene plays, how far it has got, and how it ended. It advances on the caller's
//! real frame time and knows nothing of the world. The camera is in `camera.rs`, the console in `command.rs`, and the
//! world's side (the freeze, the skip, the bars) in `world.rs`. The scene's event sequence is in `events.rs`, and what its
//! events do in `handlers.rs`. Spec: `docs/specs/nis-player.md`, `nis-player-2.md`.

use std::rc::{Rc, Weak};
use std::sync::Arc;

use blackbox_chunk::ice::{CameraSample, Track};
use blackbox_gfx::{Instance, RenderBackend};
use blackbox_nis::{Playback, PlaybackState, Scene, SceneClock};
use blackbox_sequence::Package;
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::nis::{CameraLibrary, NisBundle};

use super::camera;
use super::cars::{self, CarSpot, CarTrack, Target};
use super::events::{NisEvent, NisEvents};
use super::handlers::NisScreen;
use crate::audio::nis::NisAudioRequest;
use super::characters::{Characters, Materials};
use crate::scenes::world::scene_space::{Anchor, lens_of, placed};
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
    /// Where the orbit circles: the marker the scene would stand at, which a render-space scene does not use.
    pub focus: Anchor,
    /// The camera track, or `None` for a scene with no camera set (a slow orbit plays instead).
    pub camera: Option<Track>,
}

impl Plan {
    /// The plan of the scene `name` with the header `scene` (its frame window), its camera, and the `marker` it would
    /// stand at. The scene's anchor follows its type ([`placed`]). `None` when the scene has neither a window nor a
    /// camera track to give it a length.
    pub fn new(name: &str, scene: &Scene, camera: Option<Track>, marker: Anchor, kind: NisKind) -> Option<Self> {
        let clock = clock_of(scene, camera.as_ref())?;
        let anchor = placed(scene.scene_type, marker);
        Some(Self { name: name.to_owned(), kind, clock, anchor, focus: marker, camera })
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
    focus: Anchor,
    camera: Option<Track>,
    run: Playback,
    /// The scene's car tracks (`cars.rs`), one per car slot that has both its clips.
    cars: Vec<CarTrack>,
    /// Which real car each car slot drives, once the world has linked them; `None` until then.
    links: Option<Vec<(usize, Target)>>,
    /// The scene's event sequence, when its bundle has one. Dropping the scene stops it.
    events: Option<NisEvents>,
    /// The owner of the effects the scene plays: they stop once it is dropped with the scene.
    owner: Rc<()>,
    /// The scene's audio request, which the audio plugin follows while the scene plays.
    audio: NisAudioRequest,
    /// Whether the scene's voice is playing, as the audio plugin last reported it.
    voice: bool,
    /// The scene's characters (`characters.rs`), uploaded once the world prepares them.
    characters: Characters,
}

impl Playing {
    /// The linked cars' poses now, in render space and before the ground lift, each with the target it drives.
    fn spots(&self) -> Vec<(Target, CarSpot)> {
        let Some(links) = self.links.as_ref() else { return Vec::new() };
        let time = self.run.scene_time();
        links
            .iter()
            .filter_map(|&(slot, target)| {
                let car = self.cars.iter().find(|car| car.slot == slot)?;
                Some((target, car.spot(&self.anchor, time)))
            })
            .collect()
    }
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
    /// The linked cars where they stood when the last scene ended, until the world takes them (`take_final_cars`).
    final_cars: Option<Vec<(Target, CarSpot)>>,
    /// The flash and the overlay the playing scene shows; cleared when it ends.
    screen: NisScreen,
    /// The characters of the scenes that ended, freed by the next [`NisPlayer::prepare_characters`].
    retired: Vec<Characters>,
}

impl NisPlayer {
    /// Loads the scene `scene` from the install and starts it, standing at `marker` when its type places it there (types
    /// 0, 2 and 3; the others play in render space). Its camera is track `track`, or track 0 when the scene's set has no
    /// track `track`. The error names what failed.
    pub fn play(
        &mut self,
        dir: &GameDir,
        scene: &str,
        marker: Anchor,
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
        let plan = Plan::new(&bundle.name, &bundle.scene, camera, marker, kind)
            .ok_or_else(|| format!("{scene} has neither a frame window nor a camera track"))?;
        let cars = cars::read_tracks(&bundle.anim.banks);
        let characters = Characters::from_bundle(&bundle);
        self.start_with_cars(plan, cars, bundle.sequence.clone(), characters)
    }

    /// Starts a prepared scene with no cars and no events. Refused while another plays, or when the scene has no time to
    /// play.
    #[cfg(test)]
    pub fn start(&mut self, plan: Plan) -> Result<(), String> {
        self.start_with_cars(plan, Vec::new(), None, Characters::default())
    }

    /// Starts a prepared scene with its car tracks, its event sequence (if it has one) and its characters. Refused while
    /// another plays, or when the scene has no time to play. The events start on the scene's `START` stimulus, and fire
    /// as the scene runs.
    fn start_with_cars(
        &mut self,
        plan: Plan,
        cars: Vec<CarTrack>,
        sequence: Option<Arc<Package>>,
        characters: Characters,
    ) -> Result<(), String> {
        self.refuse_while_playing()?;
        let run = Playback::new(plan.clock, plan.kind.skippable());
        if run.is_finished() {
            return Err(format!("{} has no time to play", plan.name));
        }
        let events = sequence.and_then(|package| NisEvents::start(package, &plan.name));
        self.outcome = None;
        self.cut = true;
        self.screen.clear();
        self.last = Some(format!("{}: playing", plan.name));
        self.play = Some(Playing {
            audio: NisAudioRequest { scene: Some(plan.name.clone()) },
            name: plan.name,
            kind: plan.kind,
            anchor: plan.anchor,
            focus: plan.focus,
            camera: plan.camera,
            run,
            cars,
            links: None,
            events,
            owner: Rc::new(()),
            voice: false,
            characters,
        });
        Ok(())
    }

    fn refuse_while_playing(&self) -> Result<(), String> {
        match self.play.as_ref() {
            Some(playing) => Err(format!("NIS {} is playing; stop it first", playing.name)),
            None => Ok(()),
        }
    }

    /// Plays `dt` seconds of real time: the clock, the events that fall in it (returned, each once, in order) and the
    /// screen. The scene ends when its clock runs out.
    pub fn advance(&mut self, dt: f32) -> Vec<NisEvent> {
        let Some(playing) = self.play.as_mut() else { return Vec::new() };
        let before = playing.run.elapsed();
        let finished = playing.run.advance(dt) == PlaybackState::Finished;
        let step = playing.run.elapsed() - before;
        let events = playing.events.as_mut().map_or_else(Vec::new, |events| events.advance(step));
        self.screen.advance(dt);
        if finished {
            self.end(NisOutcome::Completed);
        }
        events
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
        self.final_cars = Some(playing.spots());
        self.last = Some(format!("{}: {outcome:?}", playing.name));
        self.outcome = Some(outcome);
        self.screen.clear();
        self.cut = true;
        self.retired.push(playing.characters);
    }

    /// Uploads the playing scene's characters, once, with the world's shared textures `shared`, and frees the characters
    /// of the scenes that ended. The world calls it every frame before it draws.
    pub fn prepare_characters(&mut self, renderer: &mut dyn RenderBackend, shared: &Materials) {
        for characters in self.retired.drain(..) {
            characters.release(renderer);
        }
        if let Some(playing) = self.play.as_mut() {
            playing.characters.prepare(renderer, shared);
        }
    }

    /// Adds the playing scene's character instances to `out`, at the scene's time. Nothing is added when no scene plays.
    pub fn character_instances(&self, out: &mut Vec<Instance>) {
        let Some(playing) = self.play.as_ref() else { return };
        playing.characters.instances(playing.run.scene_time(), &playing.anchor, out);
    }

    /// Frees the GPU resources of every scene's characters. The world calls it when it leaves the window.
    pub fn release_gpu(&mut self, renderer: &mut dyn RenderBackend) {
        for characters in self.retired.drain(..) {
            characters.release(renderer);
        }
        if let Some(playing) = self.play.as_mut() {
            std::mem::take(&mut playing.characters).release(renderer);
        }
    }

    /// The flash and the overlay the playing scene shows.
    pub fn screen(&self) -> &NisScreen {
        &self.screen
    }

    /// The flash and the overlay the playing scene shows, for the events that set them.
    pub fn screen_mut(&mut self) -> &mut NisScreen {
        &mut self.screen
    }

    /// The audio request of the playing scene: its name, while it plays. `None` when no scene plays.
    pub fn audio_request(&self) -> Option<&NisAudioRequest> {
        self.play.as_ref().map(|playing| &playing.audio)
    }

    /// Records whether the playing scene's voice plays, as the audio plugin reports it each frame. A voice that stops
    /// before the scene ends is logged.
    pub fn note_voice(&mut self, playing: bool) {
        let Some(scene) = self.play.as_mut() else { return };
        if scene.voice && !playing {
            log::debug!("NIS {}: the voice stopped at {:.1} s", scene.name, scene.run.elapsed());
        }
        scene.voice = playing;
    }

    /// The playing scene's anchor: its scene space in render space. `None` when no scene plays.
    pub fn anchor(&self) -> Option<Anchor> {
        self.play.as_ref().map(|playing| playing.anchor)
    }

    /// The owner of the playing scene's effects. A weak handle: it stops working when the scene ends.
    pub fn effect_owner(&self) -> Option<Weak<()>> {
        self.play.as_ref().map(|playing| Rc::downgrade(&playing.owner))
    }

    /// How the last scene ended, once: the next call gives `None` until another scene ends.
    pub fn take_outcome(&mut self) -> Option<NisOutcome> {
        self.outcome.take()
    }

    /// The name of the scene that plays, if one does.
    pub fn playing_name(&self) -> Option<&str> {
        self.play.as_ref().map(|playing| playing.name.as_str())
    }

    /// Whether a scene plays and its car channels still wait for their real cars (see [`NisPlayer::set_links`]).
    pub fn needs_links(&self) -> bool {
        self.play.as_ref().is_some_and(|playing| playing.links.is_none())
    }

    /// Gives the playing scene's car channels their real cars. Called once per scene: a channel missing from `links`
    /// stays off. Does nothing when no scene plays.
    pub fn set_links(&mut self, links: Vec<(usize, Target)>) {
        if let Some(playing) = self.play.as_mut() {
            playing.links = Some(links);
        }
    }

    /// Each car channel of the playing scene with its slot and its position now, in render space. Empty when no scene
    /// plays.
    pub fn car_slots(&self) -> Vec<(usize, Vec3)> {
        let Some(playing) = self.play.as_ref() else { return Vec::new() };
        let time = playing.run.scene_time();
        playing.cars.iter().map(|car| (car.slot, car.spot(&playing.anchor, time).position)).collect()
    }

    /// The linked cars' poses now, in render space and before the ground lift. Empty when no scene plays or none is
    /// linked.
    pub fn car_spots(&self) -> Vec<(Target, CarSpot)> {
        self.play.as_ref().map_or_else(Vec::new, Playing::spots)
    }

    /// The linked cars' poses where the last scene ended, once: the next call gives `None` until another scene ends.
    pub fn take_final_cars(&mut self) -> Option<Vec<(Target, CarSpot)>> {
        self.final_cars.take()
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
        Some(track.unwrap_or_else(|| camera::orbit(&playing.focus, fraction, aspect)))
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
