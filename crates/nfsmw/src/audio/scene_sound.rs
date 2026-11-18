//! The sounds a scene owns follow the scene: a scene that leaves the window takes them along, and a movie's sound
//! stops with its movie, also when the movie was skipped or stopped while the scene stays (the world's story movies).
use super::{Audio, pcm};
use crate::viewer::Scene;

impl Audio {
    /// Stops everything the last scene started when the host's scene-change count `changes` moved on since the last
    /// call. Returns whether it did.
    pub(crate) fn follow_scene_change(&mut self, changes: u32) -> bool {
        if self.scene_seen == changes {
            return false;
        }
        self.scene_seen = changes;
        self.trace.note("scene_change", &changes.to_string());
        self.drive_car(None, 0.0);
        self.drive_second_car(None, 0.0);
        self.stop_music("scene changed");
        self.stop_traffic();
        self.stop_explosion();
        self.stop_world_effect_audio();
        // The moment stream's other voices: a thunder clap or a breaker sting playing at the change.
        self.stop_thunder_voice();
        self.stop_breaker_moment_voice();
        self.stop_moment_stream();
        self.stop_nis("scene changed");
        self.stop_police_radio();
        true
    }

    /// Plays the sound the scene hands over (a movie's), and stops the sound of a movie that is over: run out, skipped
    /// or stopped while its scene stays in the window.
    pub(crate) fn follow_clip(&mut self, scene: &mut dyn Scene) {
        if let Some(clip) = scene.take_clip()
            && let Err(e) = self.play_music(pcm::sound(&clip))
        {
            log::warn!("the scene's sound: {e}");
        }
        if self.music.is_some() && !scene.clip_playing() {
            self.stop_music("movie over");
        }
    }
}

#[cfg(test)]
#[path = "scene_sound_tests.rs"]
mod tests;
