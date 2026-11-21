//! The NIS player's camera: the track's sample at the scene's time, placed in render space (car-space keys on the
//! scene's camera car, `player_cars.rs`), the orbit for a scene with no usable sample, and the letterbox.

use blackbox_chunk::ice::CameraSample;

use super::{NisPlayer, camera};
use crate::scenes::world::scene_space::lens_on_car;
use crate::viewer::camera::Lens;

impl NisPlayer {
    /// The camera sample of the scene now, from its track. `None` with no track, or when the track has no sample.
    pub(super) fn sample(&self) -> Option<CameraSample> {
        let playing = self.play.as_ref()?;
        camera::track_sample(playing.camera.as_ref()?, playing.run.scene_time())
    }

    /// The camera of the scene now, `aspect` being width over height. A scene whose track gives no camera (no track, or
    /// a point in a space that is not placed) plays the slow orbit instead.
    pub fn lens(&self, aspect: f32) -> Option<Lens> {
        let playing = self.play.as_ref()?;
        let car = self.camera_car().map(|spot| (spot.position, spot.rotation));
        let track = self.sample().and_then(|sample| lens_on_car(&playing.anchor, &sample, car, aspect));
        let fraction = playing.run.elapsed() / playing.run.clock().duration();
        Some(track.unwrap_or_else(|| camera::orbit(&playing.focus, fraction, aspect)))
    }

    /// The letterbox bar height as a fraction of the screen at the scene's time; 0 when the scene has no bars.
    pub fn letterbox(&self) -> f32 {
        self.sample().map_or(0.0, |sample| sample.letterbox)
    }
}
