//! The console commands of the computer-driven cars: `traffic` and `pursuit`.

use blackbox_render::Renderer;
use nfsmw_data::pursuit::heat_row;

use super::models::load_cop_models;
use crate::scenes::world::WorldScene;

pub(in crate::scenes::world) fn traffic_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    const USAGE: &str = "usage: traffic <count>|off|status|warmup <seconds>";
    let focus = scene.traffic_focus();
    if scene.traffic.is_none() {
        return Err("this track has no road network".into());
    }
    match args {
        ["off"] => {
            scene.traffic_manual = Some(0);
            if let Some(traffic) = scene.traffic.as_mut() {
                traffic.clear_cars();
            }
            Ok("traffic removed".into())
        }
        ["warmup", seconds] => {
            let seconds: f32 = seconds.parse().map_err(|_| format!("{seconds:?} is not a number ({USAGE})"))?;
            scene.traffic.as_mut().ok_or(USAGE)?.set_warmup(seconds);
            Ok(format!("a screenshot run simulates {seconds} s of traffic first"))
        }
        ["status"] => {
            let traffic = scene.traffic.as_ref().ok_or(USAGE)?;
            Ok(format!("models: {}\n{}", traffic.model_names().join(" "), traffic.status(focus)))
        }
        [count] => {
            let count: usize = count.parse().map_err(|_| format!("{count:?} is not a number ({USAGE})"))?;
            // Until the traffic setting changes, this count replaces it.
            scene.traffic_manual = Some(count);
            Ok(format!("keeping {count} cars on the road around you"))
        }
        _ => Err(USAGE.into()),
    }
}

/// `pursuit <heat>|off|status`: cops chase the player with the wave of that heat level.
pub(in crate::scenes::world) fn pursuit_command(
    scene: &mut WorldScene,
    renderer: &mut Renderer,
    args: &[&str],
) -> Result<String, String> {
    const USAGE: &str = "usage: pursuit <heat 1-10>|off|status";
    if scene.traffic.is_none() {
        return Err("this track has no road network".into());
    }
    match args {
        ["off"] => {
            if let Some(traffic) = scene.traffic.as_mut() {
                traffic.stop_pursuit();
            }
            Ok("the cops give up".into())
        }
        ["status"] => Ok(scene.traffic.as_ref().map(|t| t.pursuit_status()).unwrap_or_default()),
        [heat] => {
            let heat: u32 = heat.parse().ok().filter(|h| (1..=10).contains(h)).ok_or(USAGE)?;
            let player = scene.drive.as_ref().ok_or("not driving (use the drive command first)")?.performance();
            log::info!(
                "the player's car: top speed {:.0} m/s, grip {:.2} to {:.2}, acceleration {:.1?}",
                player.top_speed,
                player.start_grip,
                player.end_grip,
                player.acceleration
            );
            let row = heat_row(scene.physics.database(), heat).ok_or("no cop table for that heat")?;
            let names: Vec<String> = row.cops.iter().filter(|c| c.name != "copheli").map(|c| c.name.clone()).collect();
            load_cop_models(scene, renderer, &names)?;
            let traffic = scene.traffic.as_mut().ok_or(USAGE)?;
            traffic.start_pursuit(heat, row, player);
            Ok(format!("pursuit at heat {heat}: {}", names.join(", ")))
        }
        _ => Err(USAGE.into()),
    }
}
