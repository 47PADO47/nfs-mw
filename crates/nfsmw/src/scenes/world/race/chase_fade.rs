//! The fade to black of the prologue's breakdown: the screen goes dark while the car dies, stays black through the cut to
//! the next scene, and comes back when the next race or movie takes the screen (`docs/specs/story-flow.md` section 2.4,
//! the `fade` timer).

/// Seconds the screen takes to go black when the chase's `fade` timer fires.
pub const FADE_SECS: f32 = 3.0;

/// How black the screen is, from 0 (clear) to 1 (black), and how fast it is going there.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScreenFade {
    alpha: f32,
    /// Alpha per second; 0 when the fade is at rest.
    rate: f32,
}

impl ScreenFade {
    /// Starts going black over `secs` seconds. A fade already under way keeps its earlier end if that is sooner.
    pub fn fade_out(&mut self, secs: f32) {
        let rate = 1.0 / secs.max(0.01);
        self.rate = self.rate.max(rate);
    }

    /// The screen is clear again at once.
    pub fn clear(&mut self) {
        *self = ScreenFade::default();
    }

    /// Moves the fade on by `dt` seconds.
    pub fn advance(&mut self, dt: f32) {
        self.alpha = (self.alpha + self.rate * dt).clamp(0.0, 1.0);
        if self.alpha >= 1.0 {
            self.rate = 0.0;
        }
    }

    /// The black's opacity from 0 to 1.
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Whether the fade draws anything.
    pub fn is_up(&self) -> bool {
        self.alpha > 0.0
    }
}

impl crate::scenes::world::WorldScene {
    /// Draws the fade over the frame. A story movie or a cutscene that plays on top of it takes the screen instead.
    pub(in crate::scenes::world) fn show_screen_fade(&self, ctx: &egui::Context) {
        if !self.screen_fade_up() {
            return;
        }
        let alpha = (self.screen_fade.alpha() * 255.0).round() as u8;
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("screen_fade")));
        painter.rect_filled(ctx.content_rect(), 0.0, egui::Color32::from_black_alpha(alpha));
    }

    /// Whether the fade has something to draw now.
    pub(in crate::scenes::world) fn screen_fade_up(&self) -> bool {
        self.screen_fade.is_up() && !self.story_movie.is_playing() && !self.nis.is_playing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fade_reaches_black_in_its_time_and_stays() {
        let mut fade = ScreenFade::default();
        assert!(!fade.is_up());
        fade.fade_out(2.0);
        fade.advance(1.0);
        assert!((fade.alpha() - 0.5).abs() < 1e-6);
        fade.advance(5.0);
        assert_eq!(fade.alpha(), 1.0);
        fade.advance(1.0);
        assert_eq!(fade.alpha(), 1.0, "black holds");
    }

    #[test]
    fn a_quicker_fade_wins_over_a_slower_one_under_way() {
        let mut fade = ScreenFade::default();
        fade.fade_out(3.0);
        fade.fade_out(0.5);
        fade.advance(0.5);
        assert_eq!(fade.alpha(), 1.0);
    }

    #[test]
    fn clearing_puts_the_screen_back() {
        let mut fade = ScreenFade::default();
        fade.fade_out(1.0);
        fade.advance(1.0);
        fade.clear();
        assert!(!fade.is_up());
        fade.advance(1.0);
        assert!(!fade.is_up(), "a cleared fade does not go on");
    }
}
