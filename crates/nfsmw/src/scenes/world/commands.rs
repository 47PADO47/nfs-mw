//! The world viewer's console commands: `drive`, `reset`, `tp`, `goto`, `freecam` and `pos`.

use blackbox_render::Renderer;
use glam::Vec3;

use super::{View, WorldScene, load_car};

pub(super) const LIST: &[(&str, &str)] = &[
    ("drive [car]", "get into a car (a car folder or unique prefix); no name: the last one"),
    ("reset", "put the car back on the nearest road, facing the same way"),
    ("tp <x> <y>", "put the car on the road nearest to the map position"),
    ("goto <x> <y> [height]", "fly the free camera to a map position"),
    ("garage", "list your cars (for now every car in the install; use drive <car> to get into one)"),
    ("freecam", "switch between the chase camera and the free camera"),
    ("pos", "show where the camera or the car is"),
    ("props [radius]", "list the props with collision near the car or camera (default 30 m)"),
    ("tire-effects [status|clear|smoke on/off|marks on/off]", "tire visual controls and bounded resource counts"),
];

pub(super) fn run(
    scene: &mut WorldScene,
    renderer: &mut Renderer,
    name: &str,
    args: &[&str],
) -> Option<Result<String, String>> {
    Some(match name {
        "drive" => drive(scene, renderer, args),
        "reset" if args.is_empty() => reset(scene),
        "tp" => tp(scene, args),
        "goto" => goto(scene, args),
        "freecam" if args.is_empty() => Ok(scene.toggle_view().to_owned()),
        "garage" if args.is_empty() => Ok(nfsmw_data::car::list(&scene.dir).join("  ")),
        "pos" if args.is_empty() => Ok(pos(scene)),
        "props" => props(scene, args),
        "tire-effects" => tire_effects(scene, renderer, args),
        "reset" | "freecam" | "pos" | "garage" => Err(format!("usage: {name}")),
        _ => return None,
    })
}

fn tire_effects(scene: &mut WorldScene, renderer: &mut Renderer, args: &[&str]) -> Result<String, String> {
    let drive = scene.drive.as_mut().ok_or("not driving (use the drive command)")?;
    let result = drive.effects.command(args)?;
    if let [effect @ ("smoke" | "marks"), value @ ("on" | "off")] = args {
        scene.tire_effects[usize::from(*effect == "marks")] = *value == "on";
    }
    scene.upload_effects(renderer);
    let capacity = renderer.effect_capacities();
    Ok(format!("{result}; GPU capacity {} surface / {} particle vertices", capacity[0], capacity[1]))
}

fn drive(scene: &mut WorldScene, renderer: &mut Renderer, args: &[&str]) -> Result<String, String> {
    let wanted = match args {
        [] => scene.last_car.clone(),
        [name] => (*name).to_owned(),
        _ => return Err("usage: drive [car]".into()),
    };
    let (folder, model) = load_car(&scene.dir, &wanted).map_err(|e| format!("{e:#}"))?;
    scene.start_driving(renderer, folder.clone(), model, None).map_err(|e| format!("{e:#}"))?;
    Ok(format!("driving {folder}"))
}

fn reset(scene: &mut WorldScene) -> Result<String, String> {
    let drive = scene.drive.as_mut().ok_or("not driving (use the drive command)")?;
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
    let drive = scene.drive.as_mut().ok_or("not driving (use the drive command, or goto for the camera)")?;
    drive.respawn_near([x, y], None);
    scene.view = View::Chase;
    Ok(format!("looking for a road near ({x:.0}, {y:.0})"))
}

fn goto(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    let usage = "goto <x> <y> [height]";
    let [x, y] = numbers::<2>(args, usage)?;
    let height = match args.get(2) {
        Some(h) => h.parse::<f32>().map_err(|_| format!("{h:?} is not a number (usage: {usage})"))?,
        None => scene.start_height,
    };
    if scene.drive.is_some() && scene.view == View::Chase {
        scene.toggle_view();
    }
    scene.camera.position = Vec3::new(x, y, scene.camera.position.z);
    // The height is settled once the new area has loaded (above its ground, as at the start).
    scene.start = [x, y];
    scene.start_height = height;
    scene.grounded = false;
    Ok(format!("flying to ({x:.0}, {y:.0}), {height:.0} m above the ground"))
}

fn pos(scene: &WorldScene) -> String {
    let (what, p) = match (&scene.drive, scene.view) {
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
                nfsmw_data::world::PropKind::Light { mass } => format!("light, {mass:.0} kg"),
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
