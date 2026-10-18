//! Signed busted/evade meter; contract: `docs/specs/hud-pursuit-state.md`.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::pursuit::PursuitHud;
use super::radar::{APPEAR, HIDE, INIT, LEAVE, script};

const OUT: u32 = 0x9297_5065;
const ENTER: u32 = 0x13F5_1124;
const FLASH: u32 = 0x026D_ED57;
const CENTER_PULSE: u32 = 0x0382_6A28;
const SHOW: u32 = 0x001C_A7C0;
const BAR_HASHES: [u32; 5] = [0xE0E0_169B, 0x8EEE_BD33, 0x47A4_E2A9, 0x8E36_53A6, 0x30F3_9A6F];

#[derive(Clone, Copy)]
struct Bar {
    object: ObjectRef,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Bar {
    fn new(rt: &Runtime, package: PackageId, hash: u32) -> Option<Self> {
        let object = rt.find(package, hash)?;
        let at = rt.position(object)?;
        let size = rt.size(object)?;
        Some(Self { object, x: at.x, y: at.y, width: size.x, height: size.y })
    }

    fn fill(self, rt: &mut Runtime, fraction: f32, right: bool) {
        let width = self.width * fraction.clamp(0.0, 1.0);
        let direction = if right { 1.0 } else { -1.0 };
        let x = self.x + direction * (self.width - width) * 0.5;
        rt.set_size_xy(self.object, width, self.height);
        rt.set_position_xy(self.object, x, self.y);
    }
}

pub(super) struct Binding {
    meter: Option<ObjectRef>,
    cooldown: Option<ObjectRef>,
    bars: [Option<Bar>; 5],
    cooldown_bar: Option<Bar>,
    phase: Option<bool>,
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            meter: rt.find(package, 0x7B42_2BA3),
            cooldown: rt.find(package, 0x84A2_26EC),
            bars: BAR_HASHES.map(|hash| Bar::new(rt, package, hash)),
            cooldown_bar: Bar::new(rt, package, 0xCF81_7638),
            phase: None,
        }
    }

    pub fn apply(&mut self, rt: &mut Runtime, state: Option<PursuitHud>) {
        let next = state.map(|p| p.cooldown_shown());
        if next != self.phase {
            match (self.phase, next) {
                (_, None) => {
                    script(rt, self.meter, LEAVE);
                    script(rt, self.cooldown, LEAVE);
                }
                (None, Some(false)) => {
                    script(rt, self.meter, APPEAR);
                    script(rt, self.cooldown, HIDE);
                }
                (None, Some(true)) => {
                    script(rt, self.meter, HIDE);
                    script(rt, self.cooldown, APPEAR);
                }
                (Some(false), Some(true)) => {
                    script(rt, self.meter, OUT);
                    script(rt, self.cooldown, ENTER);
                }
                (Some(true), Some(false)) => {
                    script(rt, self.cooldown, OUT);
                    script(rt, self.meter, ENTER);
                }
                _ => {}
            }
            self.phase = next;
        }
        let Some(pursuit) = state else { return };
        let t = pursuit.busted.clamp(-1.0, 1.0);
        for (i, active) in [(0, t > 0.5), (4, t < -0.5)] {
            script(rt, self.bars[i].map(|b| b.object), if active { FLASH } else { INIT });
        }
        script(rt, self.bars[2].map(|b| b.object), if t > -0.1 && t < 0.1 { CENTER_PULSE } else { SHOW });
        self.geometry(rt, pursuit);
    }

    /// Called after runtime scripts as well: bar tracks must not override a freshly computed fixed edge.
    pub fn finish_frame(&self, rt: &mut Runtime, state: Option<PursuitHud>) {
        let Some(pursuit) = state else { return };
        self.geometry(rt, pursuit);
    }

    fn geometry(&self, rt: &mut Runtime, pursuit: PursuitHud) {
        let widths = widths(pursuit.busted.clamp(-1.0, 1.0));
        for (i, bar) in self.bars.iter().enumerate() {
            let Some(bar) = bar else { continue };
            bar.fill(rt, widths[i], i < 2);
        }
        if let Some(bar) = self.cooldown_bar {
            bar.fill(rt, pursuit.evade, false);
        }
    }
}

fn widths(t: f32) -> [f32; 5] {
    let inner_bust = match t {
        t if t > 0.5 => 0.9,
        t if t > 0.1 => t * 2.0 - 0.1,
        _ => 0.0,
    };
    let inner_evade = match t {
        t if t < -0.5 => 1.0,
        t if t < -0.1 => -t * 2.0 - 0.1,
        _ => 0.0,
    };
    [((t - 0.5) * 2.0).max(0.0), inner_bust, 1.0, inner_evade, (-(t + 0.5) * 2.0).max(0.0)]
}

#[cfg(test)]
mod tests {
    use super::widths;

    #[test]
    fn signed_meter_matches_the_original_segment_boundaries() {
        for (t, expected) in [
            (-1.0, [0.0, 0.0, 1.0, 1.0, 1.0]),
            (-0.75, [0.0, 0.0, 1.0, 1.0, 0.5]),
            (-0.5, [0.0, 0.0, 1.0, 0.9, 0.0]),
            (-0.1, [0.0, 0.0, 1.0, 0.0, 0.0]),
            (0.0, [0.0, 0.0, 1.0, 0.0, 0.0]),
            (0.1, [0.0, 0.0, 1.0, 0.0, 0.0]),
            (0.5, [0.0, 0.9, 1.0, 0.0, 0.0]),
            (0.75, [0.5, 0.9, 1.0, 0.0, 0.0]),
            (1.0, [1.0, 0.9, 1.0, 0.0, 0.0]),
        ] {
            for (actual, expected) in widths(t).into_iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-6, "signed input {t}");
            }
        }
    }
}
