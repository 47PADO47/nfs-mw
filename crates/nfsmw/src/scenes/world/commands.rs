//! The world viewer's console commands: `drive`, `reset`, `repair`, `tp`, `goto`, `freecam`, `pos`, `props`, `debug`,
//! `race`, `nis`, `movie`, `story`, `session`, `coop`, `plate` and `plate-design`.

use blackbox_gfx::RenderBackend;
use glam::Vec3;

use super::ai;
use super::drive::{self, MarkerMeshes};
use super::{View, WorldScene, load_car};

pub(super) const LIST: &[(&str, &str)] = &[
    (
        "extra camera|testing [key value] | center|freecam|refill|clear|recover|repair|pursuit|end-pursuit|marker|return",
        "F5 preferences and session overrides",
    ),
    (
        "environment clouds <0..1|location> | hour <0..24>",
        "set the same session weather/time controls as F8 Environment",
    ),
    ("drive [car]", "get into a car (a car folder or unique prefix); no name: the last one"),
    ("reset", "put the car back on the nearest road, facing the same way"),
    ("tp <x> <y>", "put the car on the road nearest to the map position"),
    (
        "goto <x> <y> [height] [heading pitch]",
        "fly the free camera to a map position, optionally looking along a heading and pitch (degrees)",
    ),
    (
        "garage",
        "list the cars of the install by category (racing, police, mod player, mod police with their heat waves); use drive <car> to get into one",
    ),
    ("parts preset <name>", "load a saved F6 car setup through the normal garage validation"),
    (
        "parts [stock|<body|spoiler|hood|roof|rims> <stock|index> ...]",
        "list or select the active car's installed model parts; indices start at 1",
    ),
    (
        "plate [TEXT|reset]",
        "the licence plate text of a car that has a plate (a mod car); for this session. No argument: show it; reset: the car's own",
    ),
    (
        "plate-design [ID|name|none|list [filter]|reset]",
        "the design behind the glyphs of a plate that offers designs; for this session. none: the plain plate; list: the designs",
    ),
    ("repair", "clear the car's visible damage and punctured tires"),
    (
        "damage <zone> <severity 0-6> [glass]",
        "hit the car in a zone (0-9 or DZ_FRONT, DZ_LFRONT, ...) to test the damage visuals",
    ),
    ("freecam", "switch between the chase camera and the free camera"),
    ("pos", "show where the camera or the car is"),
    ("props [radius]", "list the props with collision near the car or camera (default 30 m)"),
    (
        "traffic <count>|off|status|cars|spawn <car> [metres] [oncoming]|sirens|warmup <s>",
        "keep <count> computer-driven cars on the road around you; off removes them; cars lists the traffic cars mods add; spawn puts one of them <metres> ahead; sirens lists the police cars' siren states; warmup: a screenshot run simulates <s> seconds first",
    ),
    ("tire-effects [status|clear|smoke on/off|marks on/off]", "tire visual controls and bounded resource counts"),
    ("vehicle-effects [status|clear]", "collision spark and speed-trail resource counts"),
    (
        "exhaust-flames [status|engine <level>|pop]",
        "the tail-pipe flames (setting exhaust_flames): status, the engine upgrade level, fake a sputter backfire",
    ),
    ("debug collisions [on|off]", "draw the car's contact points: wall hits, tyre rays, props (no argument: toggle)"),
    (
        "race list [filter] | start <race> [laps=N] [opponents=N] [difficulty=N] [traffic=N] | status | stop",
        "list the races of the install, start one (a name, part of it or an event id like 1.5.2), show the race, end it",
    ),
    ("gps <x> <y> | gps off", "set the GPS route to a point on the road (x y as the map shows them), or clear it"),
    (
        "nis list | play <scene> [track] | stop | status",
        "the NIS cutscenes of the install: list them, play a scene's camera from the car (track 0 unless given), stop it, show it",
    ),
    ("movie <name> | movie stop", "play a movie full screen over the world (a name or part of one), or stop it"),
    (
        "coop on <CAR[:PRESET]> [arrows|pad] | coop off | coop",
        "co-op without the menus: player 2 joins behind the lead on the arrow block (default) or the first pad; off ends it; no argument: who plays",
    ),
    (
        "session [on [seed] | off]",
        "the multiplayer session rules: on runs the world as a session would (no time scaling, the players' own views, seeded draws, blocking model loads); off is single player; no argument: the rules and the world's state hash",
    ),
    (
        "story status | race <event id> | arrival | firstvisit",
        "the story flow: its state (stage, prologue resume, rival beaten, final race on resume), a career race with its intro (16.1.0, 1.8.1), the safe house arrival, the first visit",
    ),
];

pub(super) fn run(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    name: &str,
    args: &[&str],
) -> Option<Result<String, String>> {
    Some(match name {
        "extra" => super::extra_options::command(scene, renderer, args),
        "environment" => super::environment::command(scene, args),
        "drive" => drive(scene, renderer, args),
        "reset" if args.is_empty() => reset(scene),
        "repair" if args.is_empty() => repair(scene),
        "damage" => damage(scene, args),
        "tp" => tp(scene, args),
        "goto" => goto(scene, args),
        "gps" => gps(scene, args),
        "race" => super::race::race_command(scene, args),
        "nis" => super::nis::nis_command(scene, args),
        "movie" => super::story_movie::movie_command(scene, args),
        "story" => super::career::story_command::story_command(scene, args),
        "session" => super::session_rules::session_command(scene, args),
        "freecam" if args.is_empty() => Ok(scene.toggle_view().to_owned()),
        "garage" if args.is_empty() => Ok(crate::devtools::car_list::garage_list(&scene.dir)),
        "parts" => super::garage::command::run(scene, renderer, args),
        "plate" => super::garage::plate_command::plate_text(scene, renderer, args),
        "plate-design" => super::garage::plate_command::plate_design(scene, renderer, args),
        "coop" => super::players::command::coop(scene, renderer, args),
        "pos" if args.is_empty() => Ok(pos(scene)),
        "props" => props(scene, args),
        "traffic" => ai::traffic_command(scene, args),
        "pursuit" => ai::pursuit_command(scene, renderer, args),
        "roadblock" => ai::roadblock_command(scene, args),
        "signals" => ai::signals_command(scene, args),
        "tire-effects" => tire_effects(scene, renderer, args),
        "vehicle-effects" => vehicle_effects(scene, renderer, args),
        "exhaust-flames" => exhaust_flames(scene, args),
        "debug" => debug(scene, renderer, args),
        "reset" | "freecam" | "pos" | "garage" => Err(format!("usage: {name}")),
        _ => return None,
    })
}

fn vehicle_effects(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, args: &[&str]) -> Result<String, String> {
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command)")?;
    match args {
        [] | ["status"] => return Ok(format!("{}; {}", drive.vehicle_effects.status(), scene.impact_fx.status())),
        ["clear"] => {
            drive.vehicle_effects.clear();
            scene.impact_fx.clear();
        }
        _ => return Err("usage: vehicle-effects [status|clear]".into()),
    }
    let result = format!("{}; {}", drive.vehicle_effects.status(), scene.impact_fx.status());
    scene.upload_effects(renderer);
    Ok(result)
}

fn tire_effects(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, args: &[&str]) -> Result<String, String> {
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command)")?;
    let result = drive.effects.command(args)?;
    if let [effect @ ("smoke" | "marks"), value @ ("on" | "off")] = args {
        scene.tire_effects[usize::from(*effect == "marks")] = *value == "on";
    }
    scene.upload_effects(renderer);
    let capacity = renderer.stats().effect_capacities;
    Ok(format!("{result}; GPU capacity {} surface / {} particle vertices", capacity[0], capacity[1]))
}

fn exhaust_flames(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command)")?;
    drive.flames.command(args)
}

fn drive(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, args: &[&str]) -> Result<String, String> {
    let wanted = match args {
        [] => scene.last_car.clone(),
        [name] => (*name).to_owned(),
        _ => return Err("usage: drive [car]".into()),
    };
    let (folder, model) = load_car(&scene.dir, &wanted).map_err(|e| format!("{e:#}"))?;
    scene.start_driving(renderer, folder.clone(), model, None).map_err(|e| format!("{e:#}"))?;
    Ok(format!("driving {folder}"))
}

const DAMAGE_USAGE: &str = "usage: damage <zone 0-9 or DZ_NAME> <severity 0-6> [glass]";

fn damage(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    let (zone, severity, glazed) = match args {
        [zone, severity] => (*zone, *severity, false),
        [zone, severity, "glass"] => (*zone, *severity, true),
        _ => return Err(DAMAGE_USAGE.into()),
    };
    let zone = match zone.parse::<usize>() {
        Ok(index) => index,
        Err(_) => nfsmw_data::car::physics::DAMAGE_ZONES
            .iter()
            .position(|name| name.eq_ignore_ascii_case(zone))
            .ok_or(DAMAGE_USAGE)?,
    };
    let severity: u8 = severity.parse().map_err(|_| DAMAGE_USAGE.to_owned())?;
    if zone >= nfsmw_data::car::physics::DAMAGE_ZONES.len() || severity > blackbox_damage::MAX_SEVERITY {
        return Err(DAMAGE_USAGE.into());
    }
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command)")?;
    drive.inflict(zone, severity, glazed);
    Ok(format!("hit {} with severity {severity}", nfsmw_data::car::physics::DAMAGE_ZONES[zone]))
}

fn repair(scene: &mut WorldScene) -> Result<String, String> {
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command)")?;
    drive.repair();
    Ok("car repaired".into())
}

fn reset(scene: &mut WorldScene) -> Result<String, String> {
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command)")?;
    let at = drive.position();
    let heading = drive.heading();
    drive.respawn_near([at.x, at.y], Some(heading));
    scene.view = View::Chase;
    Ok("back on the road".into())
}

fn numbers<const N: usize>(args: &[&str], usage: &str) -> Result<[f32; N], String> {
    let mut out = [0.0; N];
    if args.len() < N {
        return Err(format!("usage: {usage}"));
    }
    for (slot, arg) in out.iter_mut().zip(args) {
        *slot = arg.trim_end_matches(',').parse().map_err(|_| format!("{arg:?} is not a number (usage: {usage})"))?;
    }
    Ok(out)
}

fn tp(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    let [x, y] = numbers::<2>(args, "tp <x> <y>")?;
    let drive = scene.players.lead_mut().ok_or("not driving (use the drive command, or goto for the camera)")?;
    drive.respawn_near([x, y], None);
    scene.view = View::Chase;
    Ok(format!("looking for a road near ({x:.0}, {y:.0})"))
}

/// `gps <x> <y>` sets the route to a point on the road (render space, as the map shows it); `gps off` clears it.
fn gps(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    const USAGE: &str = "usage: gps <x> <y> | gps off";
    if args == ["off"] {
        scene.route_to(None);
        return Ok("GPS off".into());
    }
    let [x, y] = numbers::<2>(args, USAGE)?;
    match scene.route_to(Some([x, y])) {
        true => Ok(format!("GPS route to ({x:.0}, {y:.0})")),
        false => Err(format!("no GPS route to ({x:.0}, {y:.0}): not driving, or no way to it")),
    }
}

fn goto(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    let usage = "goto <x> <y> [height] [heading pitch]";
    let [x, y] = numbers::<2>(args, usage)?;
    let height = match args.get(2) {
        Some(h) => h.parse::<f32>().map_err(|_| format!("{h:?} is not a number (usage: {usage})"))?,
        None => scene.start_height,
    };
    if scene.players.has_lead() && scene.view == View::Chase {
        scene.toggle_view();
    }
    scene.camera.position = Vec3::new(x, y, scene.camera.position.z);
    if let (Some(heading), Some(pitch)) = (args.get(3), args.get(4)) {
        let angle = |text: &&str| text.parse::<f32>().map_err(|_| format!("{text:?} is not a number (usage: {usage})"));
        scene.camera.yaw = angle(heading)?.to_radians();
        scene.camera.pitch = angle(pitch)?.to_radians();
    }
    scene.cut.arm();
    // The height is settled once the new area has loaded (above its ground, as at the start).
    scene.start = [x, y];
    scene.start_height = height;
    scene.grounded = false;
    Ok(format!("flying to ({x:.0}, {y:.0}), {height:.0} m above the ground"))
}

/// `debug collisions [on|off]`: the contact points of the car drawn in the world.
fn debug(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, args: &[&str]) -> Result<String, String> {
    const USAGE: &str = "usage: debug collisions [on|off]";
    let on = match args {
        ["collisions"] => !scene.markers_on,
        ["collisions", state] => crate::settings::parse_bool(state).map_err(|e| format!("{e} ({USAGE})"))?,
        _ => return Err(USAGE.into()),
    };
    scene.markers_on = on;
    if !on {
        return Ok("collision markers off".into());
    }
    // The meshes stay on the GPU once uploaded: switching on again costs nothing.
    scene.marker_meshes.get_or_insert_with(|| MarkerMeshes::upload(renderer));
    Ok(format!("collision markers on ({})", drive::MARKER_LEGEND))
}

fn pos(scene: &WorldScene) -> String {
    let (what, p) = match (scene.players.lead(), scene.view) {
        (Some(drive), View::Chase) => ("car", drive.position()),
        _ => ("camera", scene.camera.position),
    };
    let zone = scene.residency.zone().unwrap_or_else(|| "-".into());
    format!("{what} at ({:.1}, {:.1}, {:.1}), zone {zone}", p.x, p.y, p.z)
}

fn props(scene: &WorldScene, args: &[&str]) -> Result<String, String> {
    let radius = match args {
        [] => 30.0,
        [r] => r.parse::<f32>().map_err(|_| format!("{r:?} is not a number (usage: props [radius])"))?,
        _ => return Err("usage: props [radius]".into()),
    };
    let [x, y] = scene.focus();
    let found = scene.residency.props().near(Vec3::new(x, y, 0.0), radius);
    let mut lines: Vec<String> = found
        .iter()
        .take(25)
        .map(|(name, kind, p, d)| {
            let kind = match kind {
                nfsmw_data::world::PropKind::Rigid => "rigid".to_owned(),
                nfsmw_data::world::PropKind::Movable { mass } => format!("movable, {mass:.0} kg"),
            };
            format!("{name} ({kind}) at ({:.0}, {:.0}, {:.0}), {d:.0} m", p.x, p.y, p.z)
        })
        .collect();
    lines.push(format!(
        "{} props within {radius:.0} m, {} knocked over in all",
        found.len(),
        scene.residency.props().knocked_count()
    ));
    Ok(lines.join(
        "
",
    ))
}
