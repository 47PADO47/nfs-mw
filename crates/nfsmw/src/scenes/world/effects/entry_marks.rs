//! The ground markers of the entry zones: a coloured disc on the road at each auto shop, safe house and race start near
//! the car. The disc is brightest at its centre, fades to nothing at its edge, and breathes slowly. The look is
//! `[unconfirmed]` (`docs/specs/world-markers.md` section 1); the size is the zone's radius.

use std::f32::consts::TAU;

use blackbox_gfx::EffectVertex;
use glam::Vec3;

use super::super::entry_zones::{EntryZones, Zone, ZoneKind};

/// Zones whose centre is further than this from the car are not drawn, metres.
pub const DRAW_RANGE: f32 = 250.0;
/// Segments round each disc.
const SEGMENTS: usize = 24;
/// How far above the zone's centre the disc is laid, metres, so it does not fight the road for depth.
const LIFT: f32 = 0.15;
/// The opacity at the centre of a disc, 0..1, and how much the pulse lowers it.
const PEAK: f32 = 0.45;
const PULSE_DEPTH: f32 = 0.3;
/// Seconds for one breath of the pulse.
const PULSE_PERIOD: f32 = 2.0;

/// The disc's colour by zone kind **[unconfirmed]**: amber for the auto shops, cyan for the safe houses, white for
/// the races.
pub fn colour(kind: ZoneKind) -> [u8; 3] {
    match kind {
        ZoneKind::AutoShop => [255, 176, 0],
        ZoneKind::SafeHouse => [64, 200, 255],
        ZoneKind::CarLot => [120, 255, 120],
        ZoneKind::Race => [255, 255, 255],
    }
}

/// The opacity of the discs at `clock` seconds: `PEAK` lowered by the pulse, 0..1.
pub fn peak_opacity(clock: f32) -> f32 {
    let breath = 0.5 - 0.5 * (clock * TAU / PULSE_PERIOD).cos();
    PEAK * (1.0 - PULSE_DEPTH * breath)
}

/// Append the discs of the zones within [`DRAW_RANGE`] of `car` to `out` (surface overlays, alpha blended).
pub fn push_discs(zones: &EntryZones, car: Vec3, clock: f32, out: &mut Vec<EffectVertex>) {
    let peak = peak_opacity(clock);
    for zone in zones.near(car, DRAW_RANGE) {
        push_disc(zone, peak, out);
    }
}

/// One disc: a fan of triangles from the centre (at `peak` opacity) to a rim that is clear.
fn push_disc(zone: &Zone, peak: f32, out: &mut Vec<EffectVertex>) {
    let [r, g, b] = colour(zone.kind);
    // Render space is z up: the disc lies in the x, y plane.
    let centre = zone.centre + Vec3::Z * LIFT;
    let centre_vertex = EffectVertex {
        position: centre.to_array(),
        color: [r, g, b, (peak * 255.0) as u8],
        uv: [0.5, 0.5],
        detail: [0.0; 2],
    };
    let rim = |i: usize| {
        let angle = TAU * i as f32 / SEGMENTS as f32;
        let (sin, cos) = angle.sin_cos();
        let at = centre + Vec3::new(cos * zone.radius, sin * zone.radius, 0.0);
        EffectVertex {
            position: at.to_array(),
            color: [r, g, b, 0],
            uv: [0.5 + 0.5 * cos, 0.5 + 0.5 * sin],
            detail: [0.0; 2],
        }
    };
    for i in 0..SEGMENTS {
        out.push(centre_vertex);
        out.push(rim(i));
        out.push(rim(i + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disc_is_brightest_at_its_centre_and_clear_at_its_rim() {
        let zone = Zone { kind: ZoneKind::SafeHouse, name: "zone".to_string(), centre: Vec3::ZERO, radius: 6.0 };
        let mut out = Vec::new();
        push_disc(&zone, 0.5, &mut out);
        assert_eq!(out.len(), SEGMENTS * 3, "one triangle per segment");
        let alpha_at = |v: &EffectVertex| v.color[3];
        assert!(out.iter().step_by(3).all(|v| alpha_at(v) == 127), "the centre is the peak");
        for rim in out.chunks(3).flat_map(|t| [t[1], t[2]]) {
            assert_eq!(alpha_at(&rim), 0, "the rim is clear");
            let distance = Vec3::from(rim.position).with_z(0.0).length();
            assert!((distance - 6.0).abs() < 1e-4, "the rim is the zone's radius: {distance}");
        }
    }

    #[test]
    fn the_disc_is_laid_just_above_the_ground_at_the_zone_centre() {
        let zone = Zone {
            kind: ZoneKind::AutoShop,
            name: "zone".to_string(),
            centre: Vec3::new(3.0, -2.0, 10.0),
            radius: 7.0,
        };
        let mut out = Vec::new();
        push_disc(&zone, 0.6, &mut out);
        assert_eq!(out[0].position, [3.0, -2.0, 10.0 + LIFT], "render space: z is up");
        assert_eq!(&out[0].color[..3], &[255, 176, 0]);
    }

    #[test]
    fn the_pulse_breathes_between_the_full_peak_and_the_lower_one() {
        let high = peak_opacity(0.0);
        let low = peak_opacity(PULSE_PERIOD / 2.0);
        assert!((high - PEAK).abs() < 1e-6, "{high}");
        assert!((low - PEAK * (1.0 - PULSE_DEPTH)).abs() < 1e-6, "{low}");
    }

    #[test]
    fn only_the_zones_in_range_are_drawn() {
        let zones = EntryZones::default();
        let mut out = Vec::new();
        push_discs(&zones, Vec3::ZERO, 0.0, &mut out);
        assert!(out.is_empty(), "no zones, nothing drawn");
    }
}
