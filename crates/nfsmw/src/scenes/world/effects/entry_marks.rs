//! Installed world-icon geometry and particle groups; see world-activity-icons.md.
use blackbox_attrib::vlt_hash;
use blackbox_gfx::{Instance, InstanceKey};
use blackbox_hash::bstring_hash;
use glam::{Mat4, Vec3};
use nfsmw_data::gameplay::RaceType;
use nfsmw_data::world::emitter_placements::Placement;

use super::super::entry_zones::{EntryZones, ZoneKind};
use super::super::resident::SectionResources;

pub(in crate::scenes::world) const DRAW_RANGE: f32 = 250.0;
const ID_NAMESPACE: usize = usize::MAX - 1;

#[derive(Clone, Copy)]
struct Icon {
    model: &'static str,
    group: &'static str,
}

fn icon(kind: ZoneKind, race: Option<RaceType>) -> Option<Icon> {
    let (model, group) = match kind {
        ZoneKind::AutoShop => ("CHECKPOINT_GEOMETRY_CHOPSHOP_00", "fxgame_icongrp_chop"),
        ZoneKind::SafeHouse => ("CHECKPOINT_GEOMETRY_SAFEHOUSE_00", "fxgame_icongrp_safe"),
        ZoneKind::CarLot => ("CHECKPOINT_GEOMETRY_CARLOT_00", "fxgame_icongrp_lot"),
        ZoneKind::Race => match race? {
            RaceType::Sprint => ("CHECKPOINT_GEOMETRY_SPRINT_00", "fxgame_icongrp_sprint"),
            RaceType::Circuit => ("CHECKPOINT_GEOMETRY_CIRCUIT_00", "fxgame_icongrp_circuit"),
            RaceType::Drag => ("CHECKPOINT_GEOMETRY_DRAG_00", "fxgame_icongrp_drag"),
            RaceType::Knockout => ("CHECKPOINT_GEOMETRY_KNOCKOUT_00", "fxgame_icongrp_lapk"),
            RaceType::Tollbooth => ("CHECKPOINT_GEOMETRY_TOLLBOOTH_00", "fxgame_icongrp_tollb"),
            RaceType::SpeedTrap => ("CHECKPOINT_GEOMETRY_SPEEDTRAP_00", "fxgame_icongrp_speedt"),
            _ => return None,
        },
    };
    Some(Icon { model, group })
}

const HIDDEN: Icon = Icon { model: "CHECKPOINT_GEOMETRY_HIDECAR_00", group: "fxgame_icongrp_hidecar" };

impl super::super::WorldScene {
    pub(in crate::scenes::world) fn activity_icons(&self, camera: Vec3) -> Vec<VisibleIcon> {
        let car = self.players.lead().map_or(camera, |drive| drive.position());
        let pursuing = self.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing());
        let hiding = self.traffic.iter().filter(|_| pursuing).flat_map(|traffic| traffic.hiding_icon_positions());
        let entries = self.race.is_none() && !self.cutscene_playing() && !pursuing;
        let mut icons = collect(&self.entry.zones, car, &self.shut_race_zones(), entries, hiding);
        for item in &mut icons {
            let p = item.position;
            if let Some(height) =
                super::super::space::ground_below(self.residency.collision(), p.x, p.y, p.z + 5.0, p.z - 10.0)
            {
                item.position.z = height;
            }
        }
        icons
    }
}

/// A stable icon identity and its location, shared by geometry and particles.
#[derive(Clone)]
pub(in crate::scenes::world) struct VisibleIcon {
    id: usize,
    position: Vec3,
    icon: Icon,
}

pub(in crate::scenes::world) fn collect(
    zones: &EntryZones,
    car: Vec3,
    shut: &[String],
    show_entries: bool,
    hiding: impl Iterator<Item = Vec3>,
) -> Vec<VisibleIcon> {
    let mut result = Vec::new();
    if show_entries {
        for (id, zone) in zones.zones.iter().enumerate() {
            if zone.centre.distance_squared(car) > DRAW_RANGE * DRAW_RANGE {
                continue;
            }
            if zone.kind == ZoneKind::Race && shut.contains(&zone.name) {
                continue;
            }
            if let Some(icon) = icon(zone.kind, zone.race_type) {
                result.push(VisibleIcon { id, position: zone.centre, icon });
            }
        }
    }
    let first_hiding = zones.zones.len();
    for (i, position) in hiding.enumerate() {
        if position.distance_squared(car) <= DRAW_RANGE * DRAW_RANGE {
            result.push(VisibleIcon { id: first_hiding + i, position, icon: HIDDEN });
        }
    }
    result
}

pub(in crate::scenes::world) fn placements(icons: &[VisibleIcon]) -> Vec<((usize, usize), Placement)> {
    icons
        .iter()
        .map(|item| {
            (
                (ID_NAMESPACE, item.id),
                Placement {
                    group: vlt_hash(item.icon.group),
                    section: 1,
                    matrix: Mat4::from_translation(item.position).to_cols_array(),
                    triggers: Vec::new(),
                },
            )
        })
        .collect()
}

pub(in crate::scenes::world) fn geometry(icons: &[VisibleIcon], resources: &SectionResources, out: &mut Vec<Instance>) {
    for item in icons {
        let Some((mesh, _, _)) = resources.meshes.get(&bstring_hash(item.icon.model)) else { continue };
        out.push(Instance::keyed(
            *mesh,
            Mat4::from_translation(item.position),
            InstanceKey::new(0x7fff_fffc, item.id as u32),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::entry_zones::Zone;
    use super::*;

    #[test]
    fn icon_table_selects_matching_stock_assets() {
        let sprint = icon(ZoneKind::Race, Some(RaceType::Sprint)).unwrap();
        assert_eq!(sprint.model, "CHECKPOINT_GEOMETRY_SPRINT_00");
        assert_eq!(sprint.group, "fxgame_icongrp_sprint");
        assert!(icon(ZoneKind::Race, None).is_none());
        assert_eq!(icon(ZoneKind::SafeHouse, None).unwrap().group, "fxgame_icongrp_safe");
    }

    #[test]
    fn locked_events_and_distant_icons_are_excluded() {
        let zone = |name: &str, position| Zone {
            kind: ZoneKind::Race,
            race_type: Some(RaceType::Circuit),
            name: name.into(),
            centre: position,
            radius: 6.0,
            respawn: None,
        };
        let zones =
            EntryZones { zones: vec![zone("locked", Vec3::ZERO), zone("far", Vec3::X * 300.0)], ..Default::default() };
        assert!(collect(&zones, Vec3::ZERO, &["locked".into()], true, std::iter::empty()).is_empty());
    }

    #[test]
    fn particles_do_not_require_the_model() {
        let zones = EntryZones {
            zones: vec![Zone {
                kind: ZoneKind::SafeHouse,
                race_type: None,
                name: "safe".into(),
                centre: Vec3::X,
                radius: 6.0,
                respawn: None,
            }],
            ..Default::default()
        };
        let icons = collect(&zones, Vec3::ZERO, &[], true, std::iter::empty());
        assert_eq!(placements(&icons).len(), 1);
        let mut meshes = Vec::new();
        geometry(&icons, &SectionResources::default(), &mut meshes);
        assert!(meshes.is_empty());
    }

    #[test]
    fn model_is_drawn_without_a_particle_asset() {
        let zones = EntryZones {
            zones: vec![Zone {
                kind: ZoneKind::SafeHouse,
                race_type: None,
                name: "safe".into(),
                centre: Vec3::X,
                radius: 6.0,
                respawn: None,
            }],
            ..Default::default()
        };
        let icons = collect(&zones, Vec3::ZERO, &[], true, std::iter::empty());
        let mut resources = SectionResources::default();
        let mesh = blackbox_gfx::MeshHandle::from_raw(7);
        resources.meshes.insert(
            bstring_hash("CHECKPOINT_GEOMETRY_SAFEHOUSE_00"),
            (mesh, blackbox_scenery::LodModel { num_polys: 1, density: 1.0 }, Default::default()),
        );
        let mut out = Vec::new();
        geometry(&icons, &resources, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].mesh, mesh);
        assert_eq!(out[0].transform.w_axis.truncate(), Vec3::X);
    }

    #[test]
    #[ignore = "needs installed InGameA and attributes"]
    fn installed_activity_icon_models_and_groups_resolve() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let assets = nfsmw_data::world::load_global_textures(&dir).unwrap();
        let db = nfsmw_data::shared::attributes(&dir).unwrap();
        let icons = [
            icon(ZoneKind::AutoShop, None).unwrap(),
            icon(ZoneKind::SafeHouse, None).unwrap(),
            icon(ZoneKind::CarLot, None).unwrap(),
            icon(ZoneKind::Race, Some(RaceType::Sprint)).unwrap(),
            icon(ZoneKind::Race, Some(RaceType::Circuit)).unwrap(),
            icon(ZoneKind::Race, Some(RaceType::Drag)).unwrap(),
            icon(ZoneKind::Race, Some(RaceType::Knockout)).unwrap(),
            icon(ZoneKind::Race, Some(RaceType::Tollbooth)).unwrap(),
            icon(ZoneKind::Race, Some(RaceType::SpeedTrap)).unwrap(),
            HIDDEN,
        ];
        for icon in icons {
            assert!(assets.solids.iter().any(|solid| solid.name_hash == bstring_hash(icon.model)), "{}", icon.model);
            let group = db.collection_by_key(vlt_hash("emittergroup"), vlt_hash(icon.group)).expect(icon.group);
            assert!(group.get_f32("FarClip").is_some(), "{} requires a far clip", icon.group);
            assert!(!nfsmw_data::car::exhaust::group_emitters(&db, group).is_empty(), "{}", icon.group);
        }
    }
}
