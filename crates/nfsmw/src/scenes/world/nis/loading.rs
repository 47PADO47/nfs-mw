//! A scene's data, read off the frame thread. [`super::player::NisPlayer::play`] starts the read ([`Loading::start`])
//! and the player polls it each frame ([`Loading::poll`]); the scene starts when it is ready.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use blackbox_chunk::ice::Track;
use blackbox_sequence::Package;
use game_install::GameDir;
use nfsmw_data::nis::{CameraLibrary, NisBundle};

use super::cars::{self, CarTrack};
use super::characters::Characters;
use super::kind::NisKind;
use super::player::Plan;
use crate::scenes::world::scene_space::Anchor;

/// What the loading thread reads: the scene's bundle and the camera library.
pub struct Loaded {
    bundle: NisBundle,
    library: Arc<CameraLibrary>,
}

/// What a scene starts from.
pub struct Prepared {
    pub plan: Plan,
    pub cars: Vec<CarTrack>,
    pub sequence: Option<Arc<Package>>,
    pub characters: Characters,
}

/// A scene being read on a named thread, with what the player asked for it.
pub struct Loading {
    /// The scene's name as it was asked for.
    pub name: String,
    marker: Anchor,
    kind: NisKind,
    track: u32,
    receiver: Receiver<Result<Loaded, String>>,
}

impl Loading {
    /// Starts reading the scene `scene`. An error when the install has no such scene or no thread can start; a scene
    /// that fails to read is answered by [`Loading::poll`].
    pub fn start(dir: &GameDir, scene: &str, marker: Anchor, kind: NisKind, track: u32) -> Result<Self, String> {
        if !NisBundle::list(dir).iter().any(|name| name.eq_ignore_ascii_case(scene)) {
            return Err(format!("there is no NIS scene named {scene:?} in the install"));
        }
        let (sender, receiver) = channel();
        let (dir, name) = (dir.clone(), scene.to_owned());
        std::thread::Builder::new()
            .name("nis-load".into())
            .spawn(move || {
                // The player may have given up on the scene, so nobody listens any more.
                let _ = sender.send(read(&dir, &name));
            })
            .map_err(|e| format!("no thread to load {scene}: {e}"))?;
        Ok(Self::waiting_for(scene, marker, kind, track, receiver))
    }

    fn waiting_for(
        scene: &str,
        marker: Anchor,
        kind: NisKind,
        track: u32,
        receiver: Receiver<Result<Loaded, String>>,
    ) -> Self {
        Self { name: scene.to_owned(), marker, kind, track, receiver }
    }

    /// The result, once the thread has one: `None` while it reads.
    pub fn poll(&mut self) -> Option<Result<Loaded, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("the loading thread stopped".into())),
        }
    }

    /// The scene's plan, car tracks, events and characters from what was read. The error names what failed.
    pub fn prepare(&self, loaded: Loaded) -> Result<Prepared, String> {
        let Loaded { bundle, library } = loaded;
        let camera = camera_track(&library, &bundle.name, self.track);
        if camera.is_none() {
            log::info!("NIS {}: no camera set, so a slow orbit plays for the scene", bundle.name);
        }
        let plan = Plan::new(&bundle.name, &bundle.scene, camera, self.marker, self.kind)
            .ok_or_else(|| format!("{} has neither a frame window nor a camera track", self.name))?;
        Ok(Prepared {
            plan,
            cars: cars::read_tracks(&bundle.anim.banks),
            sequence: bundle.sequence.clone(),
            characters: Characters::from_bundle(&bundle),
        })
    }
}

/// Reads the scene `scene` and the camera library from the install.
fn read(dir: &GameDir, scene: &str) -> Result<Loaded, String> {
    let bundle = NisBundle::load(dir, scene).map_err(|e| format!("{e:#}"))?;
    let library = CameraLibrary::load(dir).map_err(|e| format!("{e:#}"))?;
    Ok(Loaded { bundle, library })
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
impl Loading {
    /// A load that the test answers through the returned sender.
    pub fn for_test(scene: &str) -> (Self, std::sync::mpsc::Sender<Result<Loaded, String>>) {
        let (sender, receiver) = channel();
        let marker = Anchor::facing(glam::Vec3::ZERO, 0);
        (Self::waiting_for(scene, marker, NisKind::Intro, 0, receiver), sender)
    }
}
