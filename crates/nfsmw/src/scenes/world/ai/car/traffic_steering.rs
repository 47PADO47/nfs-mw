//! Original traffic steering refreshes the current trail on each physical driving step.

use blackbox_roads::RandomSource;
use glam::Vec2;

use super::super::traffic;
use super::{AiCar, Ctx, Role};

impl AiCar {
    pub(super) fn refresh_traffic_target<R: RandomSource>(&mut self, ctx: &Ctx<'_, R>) {
        if !matches!(self.role, Role::Traffic) || self.is_police() || self.is_wrecked() {
            return;
        }
        let body = self.body();
        if self.speed == 0.0 && Vec2::new(body.velocity.x, body.velocity.z).length() < 1.0 {
            return;
        }
        let reach = self.avoidable_radius.max(2.0 * body.velocity.length());
        let occlusion =
            traffic::steering_occlusion(&self.nav, &body, reach, traffic::others(ctx.bodies, ctx.me, ctx.player));
        self.target = occlusion.as_ref().map_or(self.nav.position, |value| value.position);
        self.blocked = occlusion.is_some_and(|value| value.avoidable != 0);
    }
}

#[cfg(test)]
mod tests;
