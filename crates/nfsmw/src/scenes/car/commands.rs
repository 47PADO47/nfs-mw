//! The car viewer's console commands: `car`, `cars` and `freecam`.

use blackbox_render::Renderer;

use super::CarScene;
use crate::viewer::camera::FlyCamera;

pub(super) const LIST: &[(&str, &str)] = &[
    ("car <folder>", "show another car (cars lists the folders)"),
    ("cars", "list the cars in the install"),
    ("freecam", "switch between the orbit and the free camera"),
];

/// Free-camera speed, in metres per second: cars are a few metres long.
const FREE_SPEED: f32 = 4.0;

pub(super) fn run(
    scene: &mut CarScene,
    renderer: &mut Renderer,
    name: &str,
    args: &[&str],
) -> Option<Result<String, String>> {
    Some(match (name, args) {
        ("cars", []) => list(scene),
        ("car", [folder]) => switch(scene, renderer, folder),
        ("car", _) => Err("usage: car <folder>".into()),
        ("freecam", []) => Ok(toggle_free(scene)),
        _ => return None,
    })
}

fn list(scene: &CarScene) -> Result<String, String> {
    let source = scene.source.as_ref().ok_or("this viewer cannot load other cars")?;
    Ok(nfsmw_data::car::list(&source.dir).join("  "))
}

/// The folder named `wanted`, ignoring case; failing that, the only folder that starts with it.
fn pick<'a>(cars: &'a [String], wanted: &str) -> Option<&'a str> {
    if let Some(exact) = cars.iter().find(|c| c.eq_ignore_ascii_case(wanted)) {
        return Some(exact);
    }
    let lower = wanted.to_ascii_lowercase();
    let mut starts = cars.iter().filter(|c| c.to_ascii_lowercase().starts_with(&lower));
    match (starts.next(), starts.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

fn switch(scene: &mut CarScene, renderer: &mut Renderer, folder: &str) -> Result<String, String> {
    let source = scene.source.as_ref().ok_or("this viewer cannot load other cars")?;
    let cars = nfsmw_data::car::list(&source.dir);
    let found =
        pick(&cars, folder).ok_or_else(|| format!("no car {folder:?}, or several start with it (cars lists them)"))?;
    let model = nfsmw_data::car::load(&source.dir, found, &source.options).map_err(|e| format!("{e:#}"))?;
    scene.replace_model(renderer, model);
    Ok(format!("showing {found}"))
}

/// Orbit to free: start where the orbit camera is, looking at the car.
fn toggle_free(scene: &mut CarScene) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cars() -> Vec<String> {
        ["BMWM3", "BMWM3GTR", "CARRERAGT", "CAMARO", "CORVETTE"].iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn exact_beats_prefix_and_prefix_must_be_unique() {
        let cars = cars();
        assert_eq!(pick(&cars, "bmwm3"), Some("BMWM3"), "exact match even though BMWM3GTR also starts with it");
        assert_eq!(pick(&cars, "carr"), Some("CARRERAGT"));
        assert_eq!(pick(&cars, "ca"), None, "camaro and carreragt");
        assert_eq!(pick(&cars, "zzz"), None);
    }
}
