//! The front end queues EA Trax requests; the audio system owns playback and unexpected-screen-close cleanup.

use std::sync::{Mutex, OnceLock};

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

use super::Audio;
use crate::app::FrameSet;
use crate::frontend::Frontend;
use crate::settings::eatrax::Jukebox;

enum Request {
    Preview(Option<usize>),
    Changed(Jukebox),
}

fn requests() -> &'static Mutex<Vec<Request>> {
    static REQUESTS: OnceLock<Mutex<Vec<Request>>> = OnceLock::new();
    REQUESTS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn request(song: Option<usize>) {
    if let Ok(mut queue) = requests().lock() {
        queue.push(Request::Preview(song));
    }
}

pub fn changed(jukebox: Jukebox) {
    if let Ok(mut queue) = requests().lock() {
        queue.push(Request::Changed(jukebox));
    }
}

pub(super) fn add_systems(app: &mut App) {
    app.add_systems(Update, service.in_set(FrameSet::Ui).before(super::plugin::drive_car));
}

fn service(mut audio: NonSendMut<Audio>, frontend: Option<Res<Frontend>>) {
    let active = frontend.is_some_and(|frontend| frontend.jukebox_active());
    let queue = requests().lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default();
    for request in queue {
        match request {
            Request::Changed(jukebox) => audio.apply_jukebox(&jukebox),
            Request::Preview(song) if active || song.is_none() => {
                if let Err(error) = audio.preview_eatrax(song) {
                    log::warn!("EA Trax preview: {error}");
                }
            }
            Request::Preview(_) => {}
        }
    }
    if !active {
        let _ = audio.preview_eatrax(None);
    }
}
