//! A jump to a marker in the running world, under a fade (`docs/specs/vehicle-reset.md`, "Placements"): the original fades
//! the screen to black, puts the car on the marker once the area has loaded and fades back in, so the move itself is never
//! seen. The rewrite fades out, moves the car when the screen is black (`move_to_point`), and fades in once the car stands
//! on its marker. The fade times are [ours].

use crate::scenes::world::WorldScene;

/// Seconds to go black, and to come back.
const FADE_OUT: f32 = 0.3;
const FADE_IN: f32 = 0.5;

/// A jump under way: its point, and whether the car has been sent there.
#[derive(Debug, Clone, Copy)]
pub(in crate::scenes::world) struct JumpFade {
    point: crate::career::JumpPoint,
    moved: bool,
}

impl WorldScene {
    /// Starts a jump to `point` in the running world: the screen goes black first.
    pub(super) fn begin_jump_fade(&mut self, point: crate::career::JumpPoint) {
        self.career.jump_fade = Some(JumpFade { point, moved: false });
        self.screen_fade.fade_out(FADE_OUT);
    }

    /// Once a frame: moves the car when the screen is black, and fades back in once it stands on its marker.
    pub(super) fn career_jump_fade(&mut self) {
        let Some(mut jump) = self.career.jump_fade else { return };
        if !jump.moved {
            if self.screen_fade.alpha() < 1.0 {
                return;
            }
            self.move_to_point(jump.point);
            jump.moved = true;
            self.career.jump_fade = Some(jump);
            return;
        }
        if self.players.lead().is_some_and(|drive| drive.waiting_for_road().is_some()) {
            return;
        }
        self.screen_fade.fade_in(FADE_IN);
        self.career.jump_fade = None;
    }
}
