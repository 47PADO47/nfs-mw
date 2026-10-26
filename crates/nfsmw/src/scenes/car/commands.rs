//! The car viewer's console commands: `car`, `garage` and `freecam`.

use blackbox_gfx::RenderBackend;

use super::CarScene;
use crate::viewer::camera::FlyCamera;

pub(super) const LIST: &[(&str, &str)] = &[
    ("car <folder>", "show another car (garage lists the folders)"),
    ("garage", "list the cars of the install by category (racing, police, mod player, mod police); use car <folder> to show one"),
    ("paint <part-name> [RRGGBB]", "preview an installed paint at the current orbit angle"),
    ("freecam", "switch between the orbit and the free camera"),
];

/// Free-camera speed, in metres per second: cars are a few metres long.
const FREE_SPEED: f32 = 4.0;

pub(super) fn run(
    scene: &mut CarScene,
    renderer: &mut dyn RenderBackend,
    name: &str,
    args: &[&str],
) -> Option<Result<String, String>> {
    Some(match (name, args) {
        ("garage", []) => list(scene),
        ("car", [folder]) => switch(scene, renderer, folder),
        ("car", _) => Err("usage: car <folder>".into()),
        ("paint", _) => paint(scene, renderer, args),
        ("freecam", []) => Ok(toggle_free(scene)),
        _ => return None,
    })
}

fn list(scene: &CarScene) -> Result<String, String> {
    let source = scene.source.as_ref().ok_or("this viewer cannot load other cars")?;
    Ok(crate::devtools::car_list::garage_list(&source.dir))
}

fn switch(scene: &mut CarScene, renderer: &mut dyn RenderBackend, folder: &str) -> Result<String, String> {
    let source = scene.source.as_ref().ok_or("this viewer cannot load other cars")?;
    let cars = nfsmw_data::car::list(&source.dir);
    let found = nfsmw_data::car::pick_folder(&cars, folder)
        .ok_or_else(|| format!("no car {folder:?}, or several start with it (garage lists them)"))?;
    let model = nfsmw_data::car::load(&source.dir, found, &source.options).map_err(|e| format!("{e:#}"))?;
    scene.replace_model(renderer, model);
    Ok(format!("showing {found}"))
}

/// Orbit to free: start where the orbit camera is, looking at the car.
fn toggle_free(scene: &mut CarScene) -> String {
    scene.cut.arm();
    if scene.free.take().is_some() {
        return "orbit camera".into();
    }
    let eye = scene.camera.eye();
    let look = (scene.camera.target - eye).normalize_or_zero();
    scene.free = Some(FlyCamera {
        position: eye,
        yaw: look.y.atan2(look.x),
        pitch: look.z.clamp(-1.0, 1.0).asin(),
        speed: FREE_SPEED,
    });
    "free camera: WASD, Space/C, mouse (click to capture), scroll for speed".into()
}

fn paint(scene: &mut CarScene, renderer: &mut dyn RenderBackend, args: &[&str]) -> Result<String, String> {
    let (name, color) = match args {
        [name] => (*name, None),
        [name, hex] if hex.len() == 6 && hex.is_ascii() => {
            let value = u32::from_str_radix(hex, 16).map_err(|_| "color must be RRGGBB")?;
            (*name, Some([(value >> 16) as u8, (value >> 8) as u8, value as u8]))
        }
        _ => return Err("usage: paint <installed part-name> [RRGGBB]".into()),
    };
    if !scene.model.paintable {
        return Err("this car has a fixed livery".into());
    }
    let source = scene.source.as_ref().ok_or("this viewer has no installed paint catalog")?;
    let tables = nfsmw_data::car::CarTables::load(&source.dir).map_err(|e| e.to_string())?;
    let hash = blackbox_hash::bstring_hash(&name.to_ascii_uppercase());
    let part = tables.parts.by_name_hash(hash).ok_or("installed paint not found")?;
    let car = tables.car_type_for_folder(&scene.model.name).ok_or("car type unavailable")?;
    let slot = nfsmw_data::car::slot::BASE_PAINT;
    if part.part_id != nfsmw_data::car::LAYOUT.slots[slot].part_id
        || !tables.slot_types.search_types(slot, car.type_name_hash).any(|t| t == tables.parts.type_name_hash(part))
    {
        return Err("paint is unavailable for this car".into());
    }
    let selected = nfsmw_data::car::Paint::of(&tables, part);
    let color = color.unwrap_or(selected.rgb);
    let old = scene.model.paint.replace(selected);
    if let Err(error) = scene.model.set_body_color(color) {
        scene.model.paint = old;
        return Err(error.to_string());
    }
    scene.release(renderer);
    scene.upload(renderer);
    scene.cut.arm();
    Ok(format!("preview paint {name}, RGB {color:?}"))
}
