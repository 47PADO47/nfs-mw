//! The console commands of the computer-driven cars: `traffic` and `pursuit`.

use blackbox_gfx::RenderBackend;
use blackbox_vehicle::performance::Performance;
use nfsmw_data::pursuit::{ground_cops, wave_names};

use super::models::{load_cop_models, load_helicopter_model};
use super::traffic_events::{Sighting, starts_pursuit};
use crate::modding::cop_rows::{with_heat_overrides, without_models};
use crate::scenes::world::WorldScene;

pub(in crate::scenes::world) fn traffic_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    const USAGE: &str =
        "usage: traffic <count>|off|status|cars|spawn <car> [metres] [oncoming]|sirens|warmup <seconds>";
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
        ["cars"] => {
            let traffic = scene.traffic.as_ref().ok_or(USAGE)?;
            Ok(traffic.mod_traffic_status())
        }
        ["spawn", car, rest @ ..] => spawn_car(scene, focus, car, rest),
        [count] => {
            let count: usize = count.parse().map_err(|_| format!("{count:?} is not a number ({USAGE})"))?;
            // Until the traffic setting changes, this count replaces it.
            scene.traffic_manual = Some(count);
            Ok(format!("keeping {count} cars on the road around you"))
        }
        _ => Err(USAGE.into()),
    }
}

/// `traffic spawn <car> [metres] [oncoming]`: asks for the traffic car `car` (a mod's traffic car, or the `pvehicle` of one of the
/// game's) that many metres ahead of the player, 30 by default, heading the way the player faces or, with `oncoming`, towards
/// the player. It is placed on the nearest lane that can take a car as soon as its model is loaded (a few seconds the first
/// time), whatever the traffic density is.
fn spawn_car(scene: &mut WorldScene, focus: super::Focus, car: &str, rest: &[&str]) -> Result<String, String> {
    let mut metres = 30.0_f32;
    let mut oncoming = false;
    for word in rest {
        if *word == "oncoming" {
            oncoming = true;
            continue;
        }
        metres = word.parse().map_err(|_| format!("{word:?} is not a distance in metres, or `oncoming`"))?;
    }
    if !metres.is_finite() || metres.abs() > 1000.0 {
        return Err("the distance is from -1000 to 1000 metres".into());
    }
    let cars = nfsmw_data::car::list(&scene.dir);
    let name = nfsmw_data::car::pick_folder(&cars, car).unwrap_or(car).to_owned();
    let at = [focus.position[0] + focus.heading.cos() * metres, focus.position[1] + focus.heading.sin() * metres];
    let turn = match oncoming {
        true => std::f32::consts::PI,
        false => 0.0,
    };
    if !crate::traffic_requests::push(&name, at, focus.heading + turn) {
        return Err("too many cars are waiting to be placed".into());
    }
    Ok(format!("{name} is asked for {metres:.0} m ahead; it is placed when its model is loaded"))
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
    const USAGE: &str = "usage: pursuit <heat 1-10>|off|status|911|heli|cop <car>|cops|breakers|hiding";
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
        ["cops"] => Ok(cops_listing(scene)),
        ["heli"] => {
            let body = scene.traffic_focus().body.ok_or("not driving (use the drive command first)")?;
            scene.traffic.as_mut().ok_or("this track has no road network")?.dev_send_helicopter(&body)
        }
        [heat] => {
            let heat: u32 = heat.parse().ok().filter(|h| (1..=10).contains(h)).ok_or(USAGE)?;
            let player = scene.players.lead().ok_or("not driving (use the drive command first)")?.ai_performance();
            log::info!(
                "the player's car: top speed {:.0} m/s, grip {:.2} to {:.2}, acceleration {:.1?}",
                player.top_speed,
                player.start_grip,
                player.end_grip,
                player.acceleration
            );
            begin_pursuit(scene, renderer, heat, player, true)
        }
        _ => Err(USAGE.into()),
    }
}

/// `pursuit cops`: the wave of every heat level (car, count, spawn weight), the mods' police cars marked with their unit
/// class, and each mod police car with its wave entries. Any car named here can be asked for with `pursuit cop <car>`.
fn cops_listing(scene: &WorldScene) -> String {
    use nfsmw_data::car::mods::{cop_cars, cop_kind};
    let rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    let mut lines: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let units: Vec<String> = row
                .cops
                .iter()
                .map(|cop| {
                    let tag = cop_kind(&cop.name).map_or(String::new(), |kind| format!(" (mod, {})", kind.name()));
                    format!("{} x{} @{}%{tag}", cop.name, cop.count, cop.chance)
                })
                .collect();
            format!("heat {}: {}", index + 1, units.join(", "))
        })
        .collect();
    for car in cop_cars() {
        let Some(unit) = car.cop.as_ref() else { continue };
        let role = match car.player {
            true => "player and police",
            false => "police only",
        };
        lines.push(format!("mod cop car {} ({role}): {}", car.folder, unit.describe()));
    }
    lines.join(
        "
",
    )
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
    forced: bool,
) -> Result<String, String> {
    let db = scene.physics.database();
    let context = scene.traffic.as_ref().ok_or("this track has no road network")?.police.pursuit_context();
    // The chase re-reads its wave at every heat level it climbs to, so the cop models of all levels load now. The waves
    // the mods set take the install's rows here, so the cars they name load too.
    let rows = with_heat_overrides(context.table().rows(db).ok_or("the pursuit escalation table is incomplete")?);
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
    traffic.police.set_forced_pursuit(forced);
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
    // The police watch the chase's primary: the lead outside co-op (`players/chase.rs`).
    let Some(drive) = scene.players.get(scene.chase_primary()) else { return };
    // The speed comes from the drive now; the traffic supplies the rest (the sight and the recent hit).
    let heat = traffic.police.patrol_heat();
    let sight = Sighting { player_speed: drive.speed().abs(), ..traffic.sighting(heat) };
    let scripted =
        traffic.requested_chase_ready(scene.traffic_focus(), scene.residency.collision(), &scene.physics.surfaces);
    if !scripted && !starts_pursuit(&traffic.events(), &sight) {
        return;
    }
    let player = drive.ai_performance();
    match begin_pursuit(scene, renderer, heat, player, false) {
        Ok(line) => log::info!(
            "{}: {line}",
            match scripted {
                true => "scripted cop",
                false => "patrol offence",
            }
        ),
        Err(e) => log::warn!("speeding: no chase: {e}"),
    }
}

/// Installs event rules even while no chase runs, so natural sightings use the same context as scripted starts.
pub(in crate::scenes::world) fn configure_pursuit_context(
    scene: &mut WorldScene,
    context: super::PursuitContext,
) -> Result<(), String> {
    let Some(traffic) = scene.traffic.as_mut() else { return Ok(()) };
    if traffic.police.pursuit_context() == context {
        return Ok(());
    }
    let db = scene.physics.database();
    let table = context.table();
    let rows = with_heat_overrides(table.rows(db).ok_or("the pursuit escalation table is incomplete")?);
    let backups = table.backups(db).ok_or("the pursuit backup table is incomplete")?;
    let formations = table.formations(db).ok_or("the pursuit formation table is incomplete")?;
    let tuning = table.tuning(db).ok_or("the pursuit level tuning table is incomplete")?;
    let support = table.support(db).ok_or("the pursuit support table is incomplete")?;
    traffic.police.set_pursuit_context(context);
    traffic.police.set_heat_growth(table.growth(db));
    traffic.police.set_formation_rows(formations);
    traffic.police.set_level_tuning(tuning);
    traffic.police.set_support_rows(support);
    traffic.police.set_rep_table(table.rep(db));
    traffic.backup_rows = backups.clone();
    if let Some(pursuit) = traffic.pursuit.as_mut() {
        pursuit.rows = rows;
        pursuit.set_backups(backups);
    }
    Ok(())
}

/// Event starts use the prepared context and obey its limits. Development console starts remain explicit overrides.
pub(in crate::scenes::world) fn start_event_pursuit(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    heat: u32,
) -> Result<String, String> {
    let player = scene.players.lead().ok_or("not driving")?.ai_performance();
    begin_pursuit(scene, renderer, heat, player, false)
}

/// The mid-size police car of the scripted chase of `16.2.1` (`docs/specs/story-flow.md` section 2.4, F-15).
const SCRIPTED_COP: &str = "copmidsize";
/// The heat the scripted chase runs at: the first level, as the prologue's cops are the lightest the game has.
const SCRIPTED_HEAT: u32 = 1;

/// The cops the scripted chase asked for at its spawn points: a chase starts if none runs, and a `copmidsize` is asked for at
/// each point, to join it. They are placed on a later run of the cop manager, when nothing overlaps the spot.
pub(in crate::scenes::world) fn scripted_cops(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) {
    use super::cop_manager::requests::Role;
    if scene.career.scripted_cops.is_empty() {
        return;
    }
    let points = std::mem::take(&mut scene.career.scripted_cops);
    if let Err(e) = load_cop_models(scene, renderer, &[SCRIPTED_COP.to_owned()]) {
        return log::warn!("chase: the cop car {SCRIPTED_COP} did not load, so no cops come: {e}");
    }
    if scene.traffic.as_ref().is_some_and(|traffic| !traffic.is_pursuing())
        && let Some(player) = scene.players.lead().map(|drive| drive.ai_performance())
    {
        match begin_pursuit(scene, renderer, SCRIPTED_HEAT, player, false) {
            Ok(line) => log::info!("chase: {line}"),
            Err(e) => log::warn!("chase: no pursuit for the scripted cops: {e}"),
        }
    }
    let Some(traffic) = scene.traffic.as_mut() else { return };
    let role = match traffic.is_pursuing() {
        true => Role::Chase,
        false => Role::Patrol,
    };
    for (at, forward) in points {
        traffic.queue_cop_spawn(SCRIPTED_COP, at, forward, role);
    }
    log::info!("chase: the scripted cops are asked for: {}", traffic.fleet_status());
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
    let drive = scene.players.lead().ok_or("not driving (use the drive command first)")?;
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
