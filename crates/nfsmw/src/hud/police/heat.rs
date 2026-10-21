//! Fractional heat and authored scripts. Spec: `hud-pursuit-state.md`.

use blackbox_feng::{ObjectRef, PackageId, Runtime, runtime::INIT_SCRIPT};

pub(super) struct Binding {
    masks: [Option<ObjectRef>; 2],
    icon: Option<ObjectRef>,
    multiplier: Option<ObjectRef>,
    labels: Vec<ObjectRef>,
}

fn script(rt: &mut Runtime, object: ObjectRef, id: u32) {
    if rt.script_of(object) != Some(id) {
        rt.run_script(object, id);
    }
}

fn rotations(fraction: f32) -> [f32; 2] {
    [175.0 - fraction.min(0.5) * 350.0, 175.0 - (fraction - 0.5).max(0.0) * 350.0]
}

impl Binding {
    pub(super) fn new(rt: &Runtime, package: PackageId) -> Self {
        let labels = rt
            .package(package)
            .into_iter()
            .flat_map(|p| p.objects.iter().enumerate())
            .filter(|(_, o)| {
                o.kind == blackbox_feng::package::ObjectKind::String && matches!(o.name_hash, 0xA81A_2028 | 0x7F91_DA62)
            })
            .map(|(index, _)| ObjectRef { package, index })
            .collect();
        Self {
            masks: [0x8628_24C9, 0x4B2C_BE1B].map(|hash| rt.find(package, hash)),
            icon: rt.find(package, 0x6F85_ED55),
            multiplier: rt.find(package, 0x7F91_DA62),
            labels,
        }
    }

    pub(super) fn apply(&self, rt: &mut Runtime, heat: f32) {
        let heat = match heat.is_finite() {
            true => heat.clamp(0.0, 10.0),
            false => 0.0,
        };
        let fraction = heat.fract();
        for (mask, degrees) in self.masks.into_iter().zip(rotations(fraction)) {
            if let Some(mask) = mask {
                rt.set_mask_rotation(mask, degrees);
            }
        }
        for &label in &self.labels {
            rt.set_text(label, format!("x{}", heat as u32));
            rt.set_alpha(label, 255);
        }
        if let Some(multiplier) = self.multiplier {
            let id = match fraction < 0.5 {
                true => 0x41E1_FEDC,
                false => INIT_SCRIPT,
            };
            script(rt, multiplier, id);
            rt.set_hidden(multiplier, heat < 1.0);
        }
        let Some(icon) = self.icon else { return };
        if heat <= 0.0 {
            script(rt, icon, INIT_SCRIPT);
            return;
        }
        let id = match fraction > 0.75 {
            true => 0xDA60_0155,
            false => 0x7703_1C70,
        };
        script(rt, icon, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mask_fills_its_own_half_and_both_reset_at_the_next_level() {
        assert_eq!(rotations(1.0_f32.fract()), [175.0, 175.0]);
        assert_eq!(rotations(1.25_f32.fract()), [87.5, 175.0]);
        assert_eq!(rotations(1.5_f32.fract()), [0.0, 175.0]);
        assert_eq!(rotations(1.75_f32.fract()), [0.0, 87.5]);
        assert_eq!(rotations(2.0_f32.fract()), [175.0, 175.0]);
    }
}
