//! The detector's two authored layouts. Contract: `docs/specs/hud-pursuit-state.md`.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::HudState;

pub(super) const INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;
pub(super) const HIDE: u32 = 0x0016_A259;
pub(super) const APPEAR: u32 = 0x5079_C8F8;
pub(super) const LEAVE: u32 = 0x0331_13AC;

pub(super) fn script(rt: &mut Runtime, object: Option<ObjectRef>, id: u32) {
    let Some(object) = object else { return };
    if rt.script_of(object) != Some(id) {
        rt.run_script(object, id);
    }
}

fn leave(rt: &mut Runtime, object: Option<ObjectRef>) {
    let Some(object) = object else { return };
    match rt.script_of(object) {
        Some(INIT) | None => script(rt, Some(object), HIDE),
        Some(HIDE | LEAVE) => {}
        _ => script(rt, Some(object), LEAVE),
    }
}

pub(super) struct Binding {
    group: Option<ObjectRef>,
    backing: Option<ObjectRef>,
    mirror_backing: Option<ObjectRef>,
    direction: Option<ObjectRef>,
    icon: Option<ObjectRef>,
    leds: [Option<ObjectRef>; 2],
    sweep: Sweep,
    dt: f32,
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let find = |hash| rt.find(package, hash);
        Self {
            group: find(0x0627_43F5),
            backing: find(0x839E_7D77),
            mirror_backing: find(0x9EE0_6631),
            direction: find(0x2419_6AD4),
            icon: find(0x6CCB_BBF1),
            leds: [0x69AA_01E7, 0x9F59_065A].map(find),
            sweep: Sweep::default(),
            dt: 0.0,
        }
    }

    pub fn advance(&mut self, dt: f32) {
        self.dt += dt.max(0.0);
    }

    pub fn apply(&mut self, rt: &mut Runtime, state: &HudState, mirror: bool) {
        let detecting = state.pursuit.is_none_or(|p| p.cooldown_shown());
        script(rt, self.mirror_backing, if mirror { APPEAR } else { HIDE });
        match (mirror, detecting) {
            (true, true) => {
                script(rt, self.backing, HIDE);
                script(rt, self.group, INIT);
            }
            (true, false) => {
                script(rt, self.backing, HIDE);
                script(rt, self.group, HIDE);
            }
            (false, true) => {
                script(rt, self.backing, APPEAR);
                script(rt, self.group, APPEAR);
            }
            (false, false) => {
                leave(rt, self.backing);
                leave(rt, self.group);
            }
        }
        let target = state.radar.filter(|_| detecting);
        let dt = std::mem::take(&mut self.dt);
        let reveal = self.sweep.update(target.map(|radar| radar.normalized_range()), dt);
        for &led in self.leds.iter().flatten() {
            rt.set_mask_uv(led, [0.0, 0.0, reveal, 1.0]);
            script(rt, Some(led), INIT);
        }
        script(rt, self.icon, INIT);
        script(rt, self.direction, if target.is_some() { INIT } else { HIDE });
        if let Some(arrow) = self.direction {
            // Also supports a minimal HUD fixture without authored scripts.
            rt.set_hidden(arrow, target.is_none());
            if let Some(radar) = target {
                rt.set_rotation_z(arrow, -radar.angle);
            }
        }
    }
}

/// The original LED ramp advances per HUD update, while its cycle uses world time.
struct Sweep {
    elapsed: Option<f32>,
    amount: f32,
}

impl Default for Sweep {
    fn default() -> Self {
        Self { elapsed: None, amount: 0.3 }
    }
}

impl Sweep {
    fn update(&mut self, range: Option<f32>, dt: f32) -> f32 {
        let Some(range) = range.filter(|range| *range > 0.0) else {
            self.elapsed = None;
            return 0.0;
        };
        let elapsed = self.elapsed.map_or(0.0, |elapsed| elapsed + dt);
        self.amount = (self.amount + 0.1).min(1.0);
        self.elapsed = Some(elapsed);
        if elapsed > range.max(0.1) * 1.5 {
            self.elapsed = Some(0.0);
            self.amount = 0.3;
        }
        self.amount
    }
}

#[cfg(test)]
mod tests {
    use super::Sweep;

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
    }

    #[test]
    fn led_ramp_advances_per_update_and_restarts_only_after_the_cycle_period() {
        let mut sweep = Sweep::default();
        close(sweep.update(Some(0.01), 20.0), 0.4);
        close(sweep.update(Some(0.01), 0.0), 0.5);
        let period = 0.1 * 1.5;
        close(sweep.update(Some(0.01), period), 0.6);
        close(sweep.update(Some(0.01), 0.001), 0.3);
        close(sweep.update(Some(0.01), 20.0), 0.3);
        close(sweep.elapsed.unwrap(), 0.0);
        close(sweep.update(None, 5.0), 0.0);
        assert!(sweep.elapsed.is_none());
        close(sweep.update(Some(1.0), 5.0), 0.4);
    }

    #[test]
    fn led_reveal_retains_the_original_hud_rate_dependence() {
        let mut slow = Sweep::default();
        let mut fast = Sweep::default();
        slow.update(Some(1.0), 0.0);
        fast.update(Some(1.0), 0.0);
        close(slow.update(Some(1.0), 0.05), 0.5);
        for _ in 0..3 {
            fast.update(Some(1.0), 1.0 / 60.0);
        }
        close(fast.amount, 0.7);
        for _ in 0..10 {
            fast.update(Some(1.0), 0.0);
        }
        close(fast.amount, 1.0);
        close(fast.update(None, 1.0), 0.0);
        close(fast.update(Some(1.0), 0.0), 1.0);
    }
}
