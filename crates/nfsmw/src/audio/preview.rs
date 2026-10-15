//! The sound viewer's transport: the scene asks for a bank sound to play, pause, seek or stop, and the audio plugin
//! plays it and reports its position back. Both ends share one [`Preview`], as the front end and its viewer share a
//! `frontend::Browse`. The plugin's half is `preview_play`.

use std::sync::{Arc, Mutex, PoisonError};

/// A request from the scene to the audio plugin. Requests are taken in the order they were made.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// Play sound `index` of the bank `bank` (a path under `SOUND/`, such as `FE/FE_MB.abk`), replacing the
    /// sound playing now.
    Play { bank: String, index: usize },
    /// Pause a playing sound, or resume a paused one.
    TogglePause,
    /// Move the playhead to this many seconds into the sound.
    Seek(f64),
}

/// What the plugin reports back each frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// The bank and index of the sound last asked for, if any.
    pub current: Option<(String, usize)>,
    pub playing: bool,
    pub paused: bool,
    /// The playhead, in seconds.
    pub position: f64,
    /// The length of the current sound, in seconds; 0 when none is loaded.
    pub duration: f64,
    /// The sound ended on its own (its playhead reached the end), until the next request.
    pub finished: bool,
    /// Why the last request failed (no sound device, a bad bank, an unreadable sound).
    pub error: Option<String>,
}

#[derive(Default)]
struct Shared {
    requests: Mutex<Vec<Request>>,
    status: Mutex<Status>,
}

/// The two ends of the transport. Clones share the same state.
#[derive(Clone, Default)]
pub struct Preview(Arc<Shared>);

impl Preview {
    /// Queues a request for the audio plugin.
    pub fn request(&self, request: Request) {
        self.0.requests.lock().unwrap_or_else(PoisonError::into_inner).push(request);
    }

    /// Takes every request queued since the last call.
    pub fn take_requests(&self) -> Vec<Request> {
        std::mem::take(&mut *self.0.requests.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The latest status the plugin reported.
    pub fn status(&self) -> Status {
        self.0.status.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Set by the plugin each frame.
    pub fn set_status(&self, status: Status) {
        *self.0.status.lock().unwrap_or_else(PoisonError::into_inner) = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_taken_once_in_order() {
        let preview = Preview::default();
        assert!(preview.take_requests().is_empty());
        preview.request(Request::Play { bank: "FE/FE_MB.abk".into(), index: 2 });
        preview.request(Request::TogglePause);
        assert_eq!(
            preview.take_requests(),
            [Request::Play { bank: "FE/FE_MB.abk".into(), index: 2 }, Request::TogglePause]
        );
        assert!(preview.take_requests().is_empty(), "a request is taken once");
    }

    #[test]
    fn the_clones_share_the_requests_and_the_status() {
        let scene_end = Preview::default();
        let plugin_end = scene_end.clone();
        scene_end.request(Request::Seek(1.5));
        assert_eq!(plugin_end.take_requests(), [Request::Seek(1.5)]);
        plugin_end.set_status(Status { playing: true, duration: 3.0, ..Status::default() });
        assert!(scene_end.status().playing);
        assert_eq!(scene_end.status().duration, 3.0);
    }
}
