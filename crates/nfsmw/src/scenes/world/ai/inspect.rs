//! `pursuit breakers` and `pursuit hiding`: what the installed data put in the resident world, for checking that a
//! breaker or a hiding spot exists near the player. Specs: `docs/specs/pursuit-breakers.md`,
//! `docs/specs/pursuit-hiding.md`.

use glam::{Vec2, Vec3};

use crate::scenes::world::{WorldScene, space};

/// How many entries a listing prints before it summarises the rest.
const LISTED: usize = 6;

/// The player (or the camera) in physics space.
fn viewer(scene: &WorldScene) -> Vec3 {
    let [x, y] = scene.focus();
    let height = scene.drive.as_ref().map_or(0.0, |d| d.physics_pose().position.z);
    Vec3::from_array(space::to_physics(Vec3::new(x, y, height)))
}

/// A compass word for the direction from the viewer to a point, in the map's frame (x east, y north).
fn compass(from: Vec3, to: Vec3) -> &'static str {
    let render = space::to_render((to - from).to_array());
    let angle = render.y.atan2(render.x).to_degrees();
    const NAMES: [&str; 8] = ["E", "NE", "N", "NW", "W", "SW", "S", "SE"];
    NAMES[(((angle + 22.5).rem_euclid(360.0)) / 45.0) as usize % 8]
}

/// `pursuit breakers`: the breaker structures in the resident tiles, the nearest first, the zones that are open and the
/// last events.
pub(in crate::scenes::world) fn breakers(scene: &WorldScene) -> Result<String, String> {
    let at = viewer(scene);
    let props = scene.residency.props();
    let sites = props.breaker_sites(&scene.physics, Vec2::new(at.x, at.z));
    let fired = sites.iter().filter(|s| s.fired).count();
    let mut lines = vec![format!("{} pursuit breakers in the resident tiles, {fired} fired", sites.len())];
    for site in sites.iter().take(LISTED) {
        let metres = Vec2::new(site.position.x - at.x, site.position.z - at.z).length();
        let moments = match site.moments.is_empty() {
            true => "no scene".to_owned(),
            false => site.moments.join("/"),
        };
        lines.push(format!(
            "  {} at map ({:.0}, {:.0}), {metres:.0} m {}: zone {:.0} m for {:.0} s, {} supports, {moments}{}",
            site.engine,
            site.position.z,
            -site.position.x,
            compass(at, site.position),
            site.zone.0,
            site.zone.1,
            site.supports,
            if site.fired { ", FIRED" } else { "" },
        ));
    }
    if sites.len() > LISTED {
        lines.push(format!("  and {} more", sites.len() - LISTED));
    }
    let zones = scene.traffic.as_ref().map(|t| t.breaker_zone_list()).unwrap_or_default();
    for (x, z, radius, left) in &zones {
        lines.push(format!("  zone open at map ({z:.0}, {:.0}): {radius:.0} m, {left:.1} s left", -x));
    }
    if zones.is_empty() {
        lines.push("  no zone is open".to_owned());
    }
    for event in props.breaker_history() {
        lines.push(format!("  event: {event}"));
    }
    lines.push(format!("  {}", scene.moment_status()));
    Ok(lines.join("\n"))
}

/// `pursuit hiding`: the hiding spots of the track, the nearest first, and whether the player is in one now.
pub(in crate::scenes::world) fn hiding(scene: &WorldScene) -> Result<String, String> {
    let traffic = scene.traffic.as_ref().ok_or("this track has no road network")?;
    let at = viewer(scene);
    Ok(traffic.hiding_report(at, LISTED))
}
