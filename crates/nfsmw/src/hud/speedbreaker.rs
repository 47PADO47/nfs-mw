//! Original charge gauge and scripts. Spec: `speedbreaker-audio.md`.
use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

pub(super) const ICON: &str = "SPEEDBREAKER_METER_ICON_GROUP";
pub(super) const GROUP: &str = "Speedbreaker_Meter_Group";
pub(super) const MASK: &str = "Speedbreaker_Meter_Multi_Image";
pub(super) const LINEAR_GROUP: u32 = 0x82D6_0021;
pub(super) const LINEAR_BAR: u32 = 0x1FDA_F669;
const LINEAR_ACTIVE: u32 = 0x5B0D_9106;
const LOW: u32 = 0x026D_ED57;
const NORMAL: u32 = 0x620E_4851;
const CHARGED: u32 = 0x7703_1C70;

pub(super) struct Binding {
    package: PackageId,
    active: bool,
    icon: Option<ObjectRef>,
    mask: Option<ObjectRef>,
    group: Option<ObjectRef>,
    bar: Option<Bar>,
    empty_degrees: f32,
}

struct Bar {
    object: ObjectRef,
    left: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn script(rt: &mut Runtime, object: ObjectRef, id: u32) {
    if rt.script_of(object) != Some(id) {
        rt.run_script(object, id);
    }
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let find = |name| rt.find(package, fe_hash_upper(name));
        let bar = rt.find(package, LINEAR_BAR).and_then(|object| {
            let size = rt.size(object)?;
            let at = rt.position(object)?;
            Some(Bar { object, left: at.x - size.x / 2.0, y: at.y, width: size.x, height: size.y })
        });
        let drag = rt.package(package).is_some_and(|p| p.name.eq_ignore_ascii_case("HUD_Drag.fng"));
        Self {
            package,
            active: false,
            icon: find(ICON),
            mask: find(MASK),
            group: rt.find(package, LINEAR_GROUP),
            bar,
            empty_degrees: match drag {
                true => -48.0,
                false => 175.0,
            },
        }
    }

    pub fn update(&mut self, rt: &mut Runtime, charge: f32, active: bool) {
        if active != self.active {
            self.active = active;
            let (fade, mode) = match active {
                true => (0x54C2_0A66, "ACTIVATE"),
                false => (0xBCC0_0F05, "DEACTIVATE"),
            };
            rt.post_to_package(self.package, fade);
            rt.post_to_package(self.package, fe_hash_upper(mode));
        }
        let charge = charge.clamp(0.0, 1.0);
        if let Some(mask) = self.mask {
            rt.set_mask_rotation(mask, self.empty_degrees * (1.0 - charge));
        }
        if let Some(icon) = self.icon {
            script(
                rt,
                icon,
                match charge > 0.0 {
                    true => CHARGED,
                    false => blackbox_feng::runtime::INIT_SCRIPT,
                },
            );
        }
        if !self.group.is_some_and(|group| rt.script_of(group) == Some(LINEAR_ACTIVE)) {
            return;
        }
        let Some(bar) = &self.bar else { return };
        let width = bar.width * charge;
        rt.set_size_xy(bar.object, width, bar.height);
        rt.set_position_xy(bar.object, bar.left + width / 2.0, bar.y);
        script(
            rt,
            bar.object,
            match charge <= 0.3 {
                true => LOW,
                false => NORMAL,
            },
        );
    }
}
