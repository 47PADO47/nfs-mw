//! Installed lens-flare profiles. Behavior: `docs/specs/player-car-lights.md`.

mod damage;
pub use damage::damage_mask;

use blackbox_attrib::{CollectionRef, Database, Value};
use blackbox_hash::bstring_hash;

pub const FLARE_TEXTURES: [&str; 3] = ["HEADLIGHTFLAREINNER", "HEADLIGHTFLAREOUTER", "HEADLIGHTGLOW"];

#[derive(Debug, Clone, PartialEq)]
pub struct FlareProfile {
    pub size: [f32; 2],
    pub scale: [f32; 2],
    pub power: f32,
    pub z_bias: f32,
    pub color: [f32; 4],
    pub texture: u32,
}

impl FlareProfile {
    pub(crate) fn read(c: CollectionRef<'_>) -> Option<Self> {
        let Value::Vector4(color) = c.get("colour")? else { return None };
        let slot = usize::try_from(c.get_u32("flare_texture")?).ok()?;
        let profile = Self {
            size: [c.get_f32("MinSize")?, c.get_f32("MaxSize")?],
            scale: [c.get_f32("MinScale")?, c.get_f32("Maxscale")?],
            power: c.get_f32("Power")?,
            z_bias: c.get_f32("ZBias")?,
            color: *color,
            texture: bstring_hash(FLARE_TEXTURES.get(slot)?),
        };
        let valid = profile
            .size
            .into_iter()
            .chain(profile.scale)
            .chain([profile.power, profile.z_bias])
            .chain(profile.color)
            .all(|v| v.is_finite() && v >= 0.0);
        valid.then_some(profile)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlareProfiles {
    pub head: Vec<FlareProfile>,
    pub brake: Vec<FlareProfile>,
    pub traffic_brake: Vec<FlareProfile>,
    pub reverse: Vec<FlareProfile>,
    pub cop_red: Vec<FlareProfile>,
    pub cop_blue: Vec<FlareProfile>,
    pub cop_white: Vec<FlareProfile>,
    pub cop_bright_red: Vec<FlareProfile>,
    pub cop_bright_blue: Vec<FlareProfile>,
    pub cop_orange: Vec<FlareProfile>,
}

impl FlareProfiles {
    pub fn read(db: &Database) -> Self {
        let read = |names: &[&str]| {
            names.iter().filter_map(|name| FlareProfile::read(db.collection("light_flares_cg", name)?)).collect()
        };
        Self {
            head: read(&["headlight_inner", "headlight_outer"]),
            brake: read(&["brakelight_inner"]),
            traffic_brake: read(&["brakelight_traffic_inner"]),
            reverse: read(&["reverselight_inner"]),
            cop_red: read(&["coplightred_inner"]),
            cop_blue: read(&["coplightblue_inner"]),
            cop_white: read(&["coplightwhite_inner"]),
            cop_bright_red: read(&["coplightbrightred"]),
            cop_bright_blue: read(&["coplightbrightblue"]),
            cop_orange: read(&["coplightorange"]),
        }
    }

    pub fn textures(&self) -> impl Iterator<Item = u32> + '_ {
        self.head
            .iter()
            .chain(&self.brake)
            .chain(&self.traffic_brake)
            .chain(&self.reverse)
            .chain(&self.cop_red)
            .chain(&self.cop_blue)
            .chain(&self.cop_white)
            .chain(&self.cop_bright_red)
            .chain(&self.cop_bright_blue)
            .chain(&self.cop_orange)
            .map(|p| p.texture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an installed game"]
    fn installed_player_lights_use_racer_profiles_and_authored_textures() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = crate::shared::attributes(&dir).unwrap();
        let p = FlareProfiles::read(&db);
        assert_eq!(p.head.len(), 2);
        assert_eq!(p.brake.len(), 1);
        assert_eq!(p.traffic_brake.len(), 1);
        assert_eq!(p.reverse.len(), 1);
        let traffic =
            FlareProfile::read(db.collection("light_flares_cg", "brakelight_traffic_inner").unwrap()).unwrap();
        assert_ne!(p.brake[0].color, traffic.color);
        assert_eq!(p.traffic_brake[0], traffic);
        let model = super::super::load(
            &dir,
            "BMWM3GTR",
            &super::super::LoadOptions { lod: 'A', all_parts: false, preset: None },
        )
        .unwrap();
        for hash in p.textures() {
            assert!(model.textures.contains_key(&hash));
        }
        for state in [false, true] {
            let swaps = model.swaps.clone().with_lamp_state(state, state, &model.textures);
            for name in ["HEADLIGHT_LEFT", "HEADLIGHT_GLASS_LEFT", "BRAKELIGHT_LEFT", "BRAKELIGHT_GLASS_LEFT"] {
                let kind = name.strip_suffix("_LEFT").unwrap();
                let enabled = state || kind == "HEADLIGHT_GLASS";
                let wanted = bstring_hash(&format!("BMWM3GTR_KIT00_{kind}_{}", if enabled { "ON" } else { "OFF" }));
                let expected = match model.textures.contains_key(&wanted) {
                    true => wanted,
                    false => model.swaps.resolve(bstring_hash(name)),
                };
                assert_eq!(swaps.resolve(bstring_hash(name)), expected);
                let used = model.solids.values().any(|solid| {
                    solid.groups.iter().any(|group| group.diffuse_texture(solid) == Some(bstring_hash(name)))
                });
                if used {
                    assert!(model.textures.contains_key(&expected), "used lamp texture {name}");
                }
            }
        }
    }
}
