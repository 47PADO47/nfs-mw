//! Console and overlay edit the same validated preference state.
use super::*;
use serde::{Serialize, de::DeserializeOwned};
fn changed<T: Serialize + DeserializeOwned>(current: &T, key: &str, value: &str) -> Result<T> {
    let mut table = toml::Value::try_from(current)?;
    let old = table.get(key).context("Unknown extra option")?;
    let value = match old.is_str() {
        true => toml::Value::String(value.into()),
        false => {
            let value = match value {
                "on" => "true",
                "off" => "false",
                other => other,
            };
            let input: toml::Table = format!("value = {value}").parse().context("Invalid extra option value")?;
            input.get("value").context("Missing value")?.clone()
        }
    };
    table.as_table_mut().context("Invalid preference table")?.insert(key.into(), value);
    table.try_into().context("Invalid extra option type")
}
pub(in crate::scenes::world) fn run(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
    run_inner(scene, renderer, args).map_err(|e| format!("{e:#}"))
}
fn run_inner(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, args: &[&str]) -> Result<String> {
    match args {
        ["camera"] => return Ok(toml::to_string(&scene.extra.camera)?),
        ["testing"] => return Ok(toml::to_string(&scene.extra.testing)?),
        ["camera", key, value] => {
            let camera: crate::extra_options::CameraPreferences = changed(&scene.extra.camera, key, value)?;
            camera.validate()?;
            let before = std::mem::replace(&mut scene.extra.camera, camera);
            if let Err(e) = crate::extra_options::storage::save(&scene.extra) {
                scene.extra.camera = before;
                return Err(e);
            }
        }
        ["testing", key, value] => {
            ensure!(scene.extra_options_allowed(), "Testing overrides require local single player");
            let testing: crate::extra_options::Overrides = changed(&scene.extra.testing, key, value)?;
            testing.validate()?;
            scene.extra.testing = testing;
        }
        [action] => {
            let action = match *action {
                "center" => Action::Center,
                "freecam" => Action::Freecam,
                "refill" => Action::Refill,
                "clear" => Action::Clear,
                "recover" => Action::Recover,
                "repair" => Action::Repair,
                "pursuit" => Action::StartPursuit,
                "set-heat" => Action::SetHeat,
                "end-pursuit" => Action::EndPursuit,
                "marker" => Action::Marker,
                "return" => Action::Return,
                _ => bail!("Unknown extra-options action"),
            };
            scene.extra_action(renderer, action)?;
        }
        _ => bail!(
            "usage: extra camera|testing [key value] | center|freecam|refill|clear|recover|repair|pursuit|end-pursuit|marker|return"
        ),
    }
    Ok("Extra option applied".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn console_updates_use_the_same_types_and_reject_unknown_or_invalid_values() {
        let defaults = crate::extra_options::CameraPreferences::default();
        let mut camera: crate::extra_options::CameraPreferences = changed(&defaults, "auto_center", "off").unwrap();
        assert!(!camera.auto_center);
        camera = changed(&camera, "distance", "12.5").unwrap();
        assert_eq!(camera.distance, 12.5);
        assert!(changed::<crate::extra_options::CameraPreferences>(&camera, "typo", "true").is_err());
        camera = changed(&camera, "fov", "999").unwrap();
        assert!(camera.validate().is_err());
    }
}
