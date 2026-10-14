//! Authored world environment lookup. Spec: `spatial-parameter-maps.md`.

use anyhow::Result;
use blackbox_hash::bstring_hash;
use blackbox_paramap::Layer;
use blackbox_roads::{TrackZones, to_zone_space};
use glam::Vec3;

use super::environment::EnvironmentContext;

const REVERB: u32 = bstring_hash("Reverb");
const ENCLOSURES: [u32; 5] = [3, 4, 5, 7, 10];

/// Installed metadata only; no collision/roof-derived environment guesses.
#[derive(Debug, Clone)]
pub struct EnvironmentMap {
    reverb: Option<Layer>,
    pub zones: Option<TrackZones>,
}

impl EnvironmentMap {
    pub fn read(metadata: &[u8]) -> Result<Self> {
        let reverb = Layer::read_all(metadata)?.into_iter().find(|layer| layer.name_hash == REVERB);
        let zones = TrackZones::read(metadata)?;
        Ok(Self { reverb, zones })
    }

    pub fn map_available(&self) -> bool {
        self.reverb.is_some()
    }

    pub fn zones_available(&self) -> bool {
        self.zones.is_some()
    }

    pub fn at(&self, position: Vec3) -> EnvironmentContext {
        if !position.is_finite() {
            return EnvironmentContext::default();
        }
        let point = to_zone_space(position);
        let selected = self.reverb.as_ref().and_then(|layer| {
            let set = layer.set_at(point.x, point.y)?;
            Some((layer.integer(set, 0)?, layer.integer(set, 2)?))
        });
        let (row, offset) = selected.unwrap_or((9, 0));
        let row = u8::try_from(row).ok().filter(|&row| row < 12).unwrap_or(9);
        let mut context =
            EnvironmentContext { zone: Some(row), wet_offset_db: offset, ..EnvironmentContext::default() };
        let Some(zones) = &self.zones else {
            return context;
        };
        for kind in ENCLOSURES {
            let candidate = zones.zones.iter().find(|zone| {
                zone.kind == kind && zone.contains(point) && (zone.elevation == 0.0 || zone.elevation >= position.y)
            });
            let Some(zone) = candidate else {
                continue;
            };
            if kind == 10 && zone.visit_info == 1 {
                return context;
            }
            context.tunnel = kind == 3;
            context.overpass = matches!(kind, 4 | 5);
            context.garage = kind == 7;
            context.dynamic = kind == 10;
            return context;
        }
        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_roads::TrackZone;
    use glam::Vec2;

    fn zone(kind: u32, elevation: f32, visit_info: i16) -> TrackZone {
        TrackZone {
            kind,
            position: Vec2::ZERO,
            direction: Vec2::X,
            elevation,
            visit_info,
            data: [0; 4],
            bbox_min: Vec2::splat(-2.0),
            bbox_max: Vec2::splat(2.0),
            polygon: vec![Vec2::new(-2.0, -2.0), Vec2::new(2.0, -2.0), Vec2::new(2.0, 2.0), Vec2::new(-2.0, 2.0)],
        }
    }

    fn map(zones: Vec<TrackZone>) -> EnvironmentMap {
        EnvironmentMap { reverb: None, zones: Some(TrackZones { zones }) }
    }

    #[test]
    fn enclosure_priority_and_elevation_follow_authored_zones() {
        let env = map(vec![zone(7, 0.0, 0), zone(3, 4.0, 0), zone(4, 0.0, 0)]);
        assert!(env.at(Vec3::new(0.0, 4.0, 0.0)).tunnel);
        let high = env.at(Vec3::new(0.0, 4.01, 0.0));
        assert!(high.overpass && !high.tunnel && !high.garage);
        assert_eq!(high.zone, Some(9));
        assert!(!env.at(Vec3::new(3.0, 0.0, 0.0)).overpass);
    }

    #[test]
    fn elevation_rejection_continues_in_file_order_to_an_unbounded_same_kind_zone() {
        let mut env = map(vec![zone(3, 3.0, 0), zone(3, 0.0, 0)]);
        for height in [0.0, 3.0, 3.01, 10.0] {
            assert!(env.at(Vec3::new(0.0, height, 0.0)).tunnel);
        }
        env.zones.as_mut().unwrap().zones.remove(1);
        assert!(env.at(Vec3::new(0.0, 3.0, 0.0)).tunnel);
        assert!(!env.at(Vec3::new(0.0, 3.01, 0.0)).tunnel);
    }

    #[test]
    fn rejected_first_dynamic_zone_does_not_select_a_later_one() {
        let env = map(vec![zone(10, 0.0, 1), zone(10, 0.0, 0)]);
        assert!(!env.at(Vec3::ZERO).dynamic);
        let active = map(vec![zone(10, 0.0, 0)]).at(Vec3::ZERO);
        assert!(active.dynamic && !active.garage);
        assert_eq!(active.mixer_inputs(super::super::environment::EnvironmentValues::default())[0], 32767);
    }

    #[test]
    fn absent_map_uses_documented_city_fallback_and_invalid_position_is_unavailable() {
        let env = map(vec![]);
        assert!(!env.map_available() && env.zones_available());
        assert_eq!(env.at(Vec3::ZERO).zone, Some(9));
        assert_eq!(env.at(Vec3::splat(f32::NAN)), EnvironmentContext::default());
    }

    #[test]
    #[ignore = "needs a game install; set NFSMW_GAME_DIR"]
    fn installed_spatial_environment_matches_original_lookup_vectors() {
        let install = game_install::GameDir::open(std::path::PathBuf::from(
            std::env::var_os("NFSMW_GAME_DIR").expect("installed context test requires NFSMW_GAME_DIR"),
        ))
        .unwrap();
        let metadata = nfsmw_data::read_unwrapped(&install, "TRACKS/L2RA.BUN").unwrap();
        let env = EnvironmentMap::read(&metadata).unwrap();
        assert!(env.map_available() && env.zones_available());
        let reverb = env.reverb.as_ref().unwrap();
        assert_eq!((reverb.node_count(), reverb.set_count()), (840, 19));
        // Independent original-PC lookup results, not derived through this reader.
        for (x, y, set, row) in [
            (0.0, 0.0, 0, 0),
            (-5000.0, -5000.0, 0, 0),
            (5000.0, 5000.0, 0, 0),
            (600.0, -600.0, 11, 10),
            (600.0, 0.0, 6, 5),
            (600.0, 600.0, 10, 9),
            (600.0, 1800.0, 9, 8),
            (1800.0, -600.0, 10, 9),
            (1800.0, 0.0, 11, 10),
            (1800.0, 600.0, 11, 10),
            (1800.0, 1800.0, 13, 9),
        ] {
            assert_eq!(reverb.set_at(x, y), Some(set), "{x},{y}");
            assert_eq!(reverb.integer(set, 0), Some(row));
            assert_eq!(reverb.integer(set, 2), Some(0));
        }
        let zones = env.zones.as_ref().unwrap();
        assert!(zones.zones.iter().any(|zone| zone.kind == 3));
        assert!(zones.zones.iter().any(|zone| matches!(zone.kind, 4 | 5)));
        let mut hits = 0;
        for zone in zones.zones.iter().filter(|zone| matches!(zone.kind, 3 | 4 | 5 | 7)) {
            let centre = zone.polygon.iter().copied().sum::<Vec2>() / zone.polygon.len().max(1) as f32;
            let context = env.at(blackbox_roads::from_zone_space(centre, zone.elevation));
            if context.tunnel || context.overpass || context.garage {
                hits += 1;
            }
        }
        assert!(hits > 0, "installed enclosure coverage must be nonzero");
    }
}
