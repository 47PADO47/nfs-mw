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

use super::camera;
use super::car_memory::CarMemory;
use super::cars::{CarSpot, CarTrack, Target};
use super::characters::{Characters, Materials};
use super::cop_wait::CopWait;
use super::events::{NisEvent, NisEvents};
use super::handlers::NisScreen;
use super::loading::Loading;
use crate::audio::nis::NisAudioRequest;
use crate::scenes::world::scene_space::{Anchor, lens_of, placed};
use crate::viewer::camera::Lens;

pub use super::kind::{NisKind, NisOutcome};

/// Frames per second of a scene's clock (`docs/specs/nis-scenes.md`, section 0).
pub const SCENE_FPS: f32 = 30.0;

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
    /// The cop channels still waiting for a police car (`cop_wait.rs`); empty when none waits.
    cop_wait: CopWait,
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
    /// What the world remembers of the linked cars between frames (`car_memory.rs`).
    memory: CarMemory,
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
    /// The scene being read on a thread; it starts playing when the read is done.
    loading: Option<Loading>,
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
    /// Starts loading the scene `scene` from the install, standing at `marker` when its type places it there (types 0, 2
    /// and 3; the others play in render space). Its camera is track `track`, or track 0 when the scene's set has no
    /// track `track`. The data is read on a thread, so the scene begins on a later frame (see [`NisPlayer::advance`]);
    /// until then the player counts as playing, and a scene that fails to load ends as skipped. The error names what
    /// failed to start.
    pub fn play(
        &mut self,
        dir: &GameDir,
        scene: &str,
        marker: Anchor,
        kind: NisKind,
        track: u32,
    ) -> Result<(), String> {
        let _trace = blackbox_trace::scope("NIS: start loading the scene");
        self.refuse_while_playing()?;
        let loading = Loading::start(dir, scene, marker, kind, track)?;
        // The scene's voice decodes while the scene loads.
        crate::audio::nis::prefetch(dir, scene);
        self.last = Some(format!("{scene}: loading"));
        self.loading = Some(loading);
        Ok(())
    }

    /// Starts the loaded scene when its thread is done. A scene that cannot start ends as skipped.
    fn finish_loading(&mut self) {
        let Some(result) = self.loading.as_mut().and_then(Loading::poll) else { return };
        let Some(loading) = self.loading.take() else { return };
        let _trace = blackbox_trace::scope("NIS: prepare the loaded scene");
        let started = result.and_then(|loaded| {
            let ready = loading.prepare(loaded)?;
            self.start_with_cars(ready.plan, ready.cars, ready.sequence, ready.characters)
        });
        let Err(error) = started else { return };
        log::warn!("NIS {}: the scene is not played: {error}", loading.name);
        self.give_up(&loading.name, &format!("failed ({error})"));
    }

    /// Ends the load that is under way as skipped. Returns whether one was.
    fn cancel_loading(&mut self) -> bool {
        let Some(loading) = self.loading.take() else { return false };
        self.give_up(&loading.name, "stopped");
        true
    }

    /// The scene `name` will not play: it ends as skipped, so the world carries on.
    fn give_up(&mut self, name: &str, why: &str) {
        self.last = Some(format!("{name}: {why}"));
        self.outcome = Some(NisOutcome::Skipped);
        self.cut = true;
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
            cop_wait: CopWait::default(),
            events,
            owner: Rc::new(()),
            voice: false,
            characters,
            memory: CarMemory::default(),
        });
        Ok(())
    }

    fn refuse_while_playing(&self) -> Result<(), String> {
        let loading = self.loading.as_ref().map(|loading| loading.name.as_str());
        match self.play.as_ref().map(|playing| playing.name.as_str()).or(loading) {
            Some(name) => Err(format!("NIS {name} is playing; stop it first")),
            None => Ok(()),
        }
    }

    /// Plays `dt` seconds of real time: the clock, the events that fall in it (returned, each once, in order) and the
    /// screen. The scene ends when its clock runs out.
    pub fn advance(&mut self, dt: f32) -> Vec<NisEvent> {
        self.finish_loading();
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
        if self.cancel_loading() {
            return true;
        }
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

    /// Whether the camera jumped since this was last called: a scene began or ended.
    pub fn take_cut(&mut self) -> bool {
        std::mem::take(&mut self.cut)
    }

    /// Whether a scene plays. The world freezes, hides its HUD and gives the camera to the scene while it does.
    pub fn is_playing(&self) -> bool {
        self.play.is_some() || self.loading.is_some()
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
            None if self.loading.is_some() => "loading the scene".to_owned(),
            None => "none playing".to_owned(),
        };
        format!("NIS: {now}; last: {}", self.last.as_deref().unwrap_or("none"))
    }
}

#[path = "player_cars.rs"]
mod cars_api;
#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
