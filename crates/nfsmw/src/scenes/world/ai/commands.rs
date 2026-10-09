//! The console commands of the computer-driven cars: `traffic` and `pursuit`.

use blackbox_render::Renderer;
use nfsmw_data::pursuit::{ai_vehicle, heat_row};

use super::{CopCar, CopModel, TrafficModel};
use crate::scenes::world::drive::CarRig;
use crate::scenes::world::{WorldScene, load_car};

/// The cars traffic is made of, by folder name; the ones missing from the install are skipped.
const TRAFFIC_CARS: &[&str] =
    &["TRAFFICCOUP", "TRAF4DSEDA", "TRAF4DSEDC", "TRAFHA", "TRAFCOURT", "TRAFPICKUPA", "TRAFMINIVAN", "TRAFNEWS"];

fn load_traffic_models(scene: &mut WorldScene, renderer: &mut Renderer) -> Result<usize, String> {
    let mut loaded = 0;
    for name in TRAFFIC_CARS {
        let (folder, model) = match load_car(&scene.dir, name) {
            Ok(car) => car,
            Err(e) => {
                log::warn!("traffic car {name}: {e:#}");
                continue;
            }
        };
        let physics = match scene.physics_of(&model) {
            Ok(physics) => physics,
            Err(e) => {
                log::warn!("traffic car {folder} has no physics: {e:#}");
                continue;
            }
        };
        let rig = std::rc::Rc::new(CarRig::upload(renderer, model));
        let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
        traffic.add_model(TrafficModel { name: folder, rig, physics });
        loaded += 1;
    }
    match loaded {
        0 => Err("no traffic car could be loaded (see the log)".into()),
        n => Ok(n),
    }
}

pub(in crate::scenes::world) fn traffic_command(
    scene: &mut WorldScene,
    renderer: &mut Renderer,
    args: &[&str],
) -> Result<String, String> {
    const USAGE: &str = "usage: traffic <count>|off|status|warmup <seconds>";
    let focus = scene.traffic_focus();
    if scene.traffic.is_none() {
        return Err("this track has no road network".into());
    }
    match args {
        ["off"] => {
            if let Some(traffic) = scene.traffic.as_mut() {
                traffic.clear(renderer);
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
            if !scene.traffic.as_ref().is_some_and(|t| t.has_models()) {
                load_traffic_models(scene, renderer)?;
            }
            let traffic = scene.traffic.as_mut().ok_or(USAGE)?;
            traffic.set_target(count);
            Ok(format!("keeping {count} cars on the road around you ({} models)", traffic.model_names().len()))
        }
        _ => Err(USAGE.into()),
    }
}

/// Loads the cop cars `names` that are not loaded yet.
fn load_cop_models(scene: &mut WorldScene, renderer: &mut Renderer, names: &[String]) -> Result<(), String> {
    for name in names {
        if scene.traffic.as_ref().is_some_and(|t| t.has_cop_model(name)) {
            continue;
        }
        let (folder, model) = load_car(&scene.dir, &name.to_ascii_uppercase()).map_err(|e| format!("{name}: {e:#}"))?;
        let car_type = model.car_type.clone().unwrap_or_else(|| name.clone());
        let physics = scene.physics_of(&model).map_err(|e| format!("{name}: {e:#}"))?;
        let ai = ai_vehicle(scene.physics.database(), &car_type);
        let performance = blackbox_vehicle::performance::measure(&physics.spec);
        log::info!(
            "cop car {folder}: top speed {:.0} m/s, grip {:.2} to {:.2}, AI limit {:.0} km/h x{} x{}",
            performance.top_speed,
            performance.start_grip,
            performance.end_grip,
            ai.max_speed_kmh,
            ai.top_speed_multiplier,
            ai.acceleration_multiplier
        );
        let rig = std::rc::Rc::new(CarRig::upload(renderer, model));
        let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
        traffic.add_cop_model(CopModel {
            model: TrafficModel { name: folder, rig, physics },
            car: CopCar { performance, ai },
        });
    }
    Ok(())
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
