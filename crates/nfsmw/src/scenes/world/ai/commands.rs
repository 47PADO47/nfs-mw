//! The console commands of the computer-driven cars: `traffic` and `pursuit`.

use blackbox_gfx::RenderBackend;
use blackbox_vehicle::performance::Performance;
use nfsmw_data::pursuit::{ground_cops, heat_row, heat_rows, wave_names};

use super::models::{load_cop_models, load_helicopter_model};
use super::traffic_events::{Sighting, starts_pursuit};
use crate::modding::cop_rows::{with_heat_overrides, without_models};
use crate::scenes::world::WorldScene;

pub(in crate::scenes::world) fn traffic_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    const USAGE: &str = "usage: traffic <count>|off|status|sirens|warmup <seconds>";
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
        ["sirens"] => {
            let traffic = scene.traffic.as_ref().ok_or(USAGE)?;
            Ok(traffic.siren_status())
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

/// `pursuit <heat>|off|status|911|heli`: cops chase the player with the wave of that heat level; `heli` sends the helicopter
/// into the running chase; `911` makes a 911 call now
/// (the cops are let in and a patrol that sees the player chases it). `pursuit breakers` and
/// `pursuit hiding` list the pursuit breakers and the hiding spots of the resident world (`inspect.rs`).
pub(in crate::scenes::world) fn pursuit_command(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
    const USAGE: &str = "usage: pursuit <heat 1-10>|off|status|911|heli|cop <car>|breakers|hiding";
    match args {
        ["breakers"] => return super::inspect::breakers(scene),
        ["hiding"] => return super::inspect::hiding(scene),
        _ => {}
    }
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
        ["911"] => Ok(scene.traffic.as_mut().map(|t| t.dev_911_call()).unwrap_or_default()),
        ["cop", name] => queue_cop(scene, renderer, name),
        ["heli"] => {
            let body = scene.traffic_focus().body.ok_or("not driving (use the drive command first)")?;
            scene.traffic.as_mut().ok_or("this track has no road network")?.dev_send_helicopter(&body)
        }
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
            begin_pursuit(scene, renderer, heat, player)
        }
        _ => Err(USAGE.into()),
    }
}

/// `pursuit cop <car>`: asks the cop manager for a cop car of that `pvehicle` name 40 m ahead of the player, as a script's spawn
/// request does. It joins the chase when one runs, else it patrols. It is placed on a later run, when the cops are not locked
/// out and nothing overlaps the spot; when the cop budget is full the lowest-priority cop is taken off the road first.
fn queue_cop(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, name: &str) -> Result<String, String> {
    use super::cop_manager::requests::Role;
    load_cop_models(scene, renderer, &[name.to_owned()])?;
    let focus = scene.traffic_focus();
    let body = focus.body.ok_or("not driving (use the drive command first)")?;
    let forward = glam::Vec3::new(body.forward.x, 0.0, body.forward.y);
    let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
    let role = match traffic.is_pursuing() {
        true => Role::Chase,
        false => Role::Patrol,
    };
    traffic.queue_cop_spawn(name, body.position + forward * 40.0, forward, role);
    Ok(format!("a {name} is asked for as {role:?}: {}", traffic.fleet_status()))
}

/// Starts a chase at `heat` against `player`: loads the cop models of its wave and gives the wave to the traffic.
fn begin_pursuit(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    heat: u32,
    player: Performance,
) -> Result<String, String> {
    let db = scene.physics.database();
    heat_row(db, heat).ok_or("no cop table for that heat")?;
    // The chase re-reads its wave at every heat level it climbs to, so the cop models of all levels load now. The waves
    // the mods set take the install's rows here, so the cars they name load too.
    let rows = with_heat_overrides(heat_rows(db));
    let names = wave_names(&rows);
    let missing = load_wave_models(scene, renderer, &names);
    let rows = without_models(rows, &missing);
    let start_names: Vec<String> =
        rows.get((heat.max(1) - 1) as usize).into_iter().flat_map(ground_cops).map(|c| c.name.clone()).collect();
    // The helicopter is not a car of the wave: it is only drawn, so it loads a rig and no physics. A model that fails
    // to load leaves the helicopter out of the picture.
    if let Err(e) = load_helicopter_model(scene, renderer) {
        log::warn!("the police helicopter has no model: {e}");
    }
    let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
    if !traffic.police.can_begin() {
        return Err("finish the arrest and infractions screen before starting another pursuit".into());
    }
    traffic.start_pursuit(heat, rows, player);
    Ok(format!("pursuit at heat {heat}: {}", start_names.join(", ")))
}

/// Loads the cop model of each name in `names` on its own, so one that fails does not stop the others. Returns the names
/// that did not load: the wave leaves their entries out (`cop_rows::without_models`), with a warning for each.
fn load_wave_models(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, names: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    for name in names {
        let Err(e) = load_cop_models(scene, renderer, std::slice::from_ref(name)) else { continue };
        log::warn!("pursuit: the cop car {name} did not load, its wave entries are skipped: {e}");
        missing.push(name.clone());
    }
    missing
}

/// The speeding trigger (`docs/specs/traffic-sound-events.md` §3.3): while no chase is on, a patrol cop that sees the
/// player starts one when the player is over the speeding line, has hit traffic lately, or the heat is high enough.
pub(in crate::scenes::world) fn speeding_pursuit(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) {
    let Some(traffic) = scene.traffic.as_ref() else { return };
    if traffic.is_pursuing() {
        return;
    }
    if !traffic.police.can_begin() {
        return;
    }
    let Some(drive) = scene.drive.as_ref() else { return };
    // The speed comes from the drive now; the traffic supplies the rest (the sight and the recent hit).
    let heat = traffic.police.patrol_heat();
    let sight = Sighting { player_speed: drive.speed().abs(), ..traffic.sighting(heat) };
    if !starts_pursuit(&traffic.events(), &sight) {
        return;
    }
    let player = drive.performance();
    match begin_pursuit(scene, renderer, heat, player) {
        Ok(line) => log::info!("speeding: {line}"),
        Err(e) => log::warn!("speeding: no chase: {e}"),
    }
}

/// `signals [props]`: the traffic lights of the signalled approaches, matched to the real traffic-light props of the map
/// or not. An approach with no matched prop shows no light.
pub(in crate::scenes::world) fn signals_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    const USAGE: &str = "usage: signals [props]";
    let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
    match args {
        [] | ["props"] => Ok(traffic.signal_props_status()),
        _ => Err(USAGE.into()),
    }
}

/// `roadblock`: lays a roadblock across the road 250 m ahead now, as the request lottery would (a manual setup; in a chase
/// the roadblocks come by themselves, `docs/specs/ai-pursuit-roadblocks.md` section 1). The outcome also goes to the log
/// file, so a headless run shows it (the console keeps its own buffer).
pub(in crate::scenes::world) fn roadblock_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    let result = roadblock_outcome(scene, args);
    match &result {
        Ok(text) => log::info!("{text}"),
        Err(e) => log::warn!("roadblock: {e}"),
    }
    result
}

fn roadblock_outcome(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    if !args.is_empty() {
        return Err("usage: roadblock".into());
    }
    let drive = scene.drive.as_ref().ok_or("not driving (use the drive command first)")?;
    if drive.hud_state().is_none() {
        return Err("the car is not on the road yet; type the command once the drive has started".into());
    }
    let view = drive.player_view();
    let (collision, props) = scene.residency.world_parts();
    let surfaces = &scene.physics.surfaces;
    let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
    traffic
        .place_roadblock_with_props(view, collision, props, surfaces)
        .map(|n| format!("roadblock: {n} cop cars placed"))
}
