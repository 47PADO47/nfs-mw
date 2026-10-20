//! Health-driven engine groups, independent of collision sequencers.
//! Spec: `docs/specs/cop-visible-damage.md`.
use super::{
    WheelSetup,
    exhaust::{GroupEmitter, group_emitters},
};
use blackbox_attrib::Database;
use glam::{Mat4, Vec3};

#[derive(Default)]
pub struct DamageFx {
    pub engine: Option<Mat4>,
    pub damage: Group,
    pub death: Group,
}

#[derive(Default)]
pub struct Group {
    pub key: u32,
    pub emitters: Vec<GroupEmitter>,
}

impl DamageFx {
    pub(super) fn read(db: &Database, base_model: &str, wheels: Option<&WheelSetup>) -> Self {
        let Some(ecar) = db.collection("ecar", &base_model.to_ascii_lowercase()) else { return Self::default() };
        let group = |field| {
            let Some(group) = ecar.follow(field) else { return Group::default() };
            Group { key: group.key(), emitters: group_emitters(db, group) }
        };
        Self {
            engine: wheels.map(|setup| {
                let [x, y, z, _] = setup.tire_offsets[0];
                let left = Vec3::new(x, y, z);
                let [x, y, z, _] = setup.tire_offsets[1];
                let right = Vec3::new(x, y, z);
                Mat4::from_translation((left + right) * 0.5 + Vec3::Z * ((left.y - right.y) * 0.2))
            }),
            damage: group("DamageEffect"),
            death: group("DeathEffect"),
        }
    }

    pub fn select(&self, health: f32) -> Option<&Group> {
        if !health.is_finite() || self.engine.is_none() {
            return None;
        }
        if health <= 0.0 && self.death.key != 0 {
            return Some(&self.death);
        }
        if health <= 0.5 && self.damage.key != 0 {
            return Some(&self.damage);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_thresholds_use_nonzero_authored_groups_with_death_precedence() {
        let mut fx = DamageFx {
            engine: Some(Mat4::IDENTITY),
            damage: Group { key: 1, ..Default::default() },
            death: Group { key: 2, ..Default::default() },
        };
        assert!(fx.select(0.5001).is_none());
        assert_eq!(fx.select(0.5).unwrap().key, 1);
        assert_eq!(fx.select(0.0001).unwrap().key, 1);
        assert_eq!(fx.select(0.0).unwrap().key, 2);
        fx.death.key = 0;
        assert_eq!(fx.select(0.0).unwrap().key, 1);
        assert!(fx.select(f32::NAN).is_none());
    }
}
