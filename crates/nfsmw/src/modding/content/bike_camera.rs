//! A bike manifest's `[bike.camera]` as the game's camera set for the bike: what the table gives, and for what it leaves out
//! the cameras derived from the bike's size (`nfsmw_data::car::mods::bike_camera`).

use blackbox_modding::content::{BikeCamera as Table, CarManifest};
use nfsmw_data::car::mods::bike_camera::{BikeCamera, Chase, Front, Helmet};

/// The cameras of the bike a manifest describes; `None` for a manifest that is not a bike.
pub(super) fn cameras_of(manifest: &CarManifest) -> Option<BikeCamera> {
    let bike = manifest.bike.as_ref()?;
    let wheelbase = bike.chassis.wheelbase_m;
    let front_hub = manifest.wheels.as_ref().map_or(wheelbase * 0.5, |layout| layout.front.x_m);
    Some(cameras_from(&bike.camera, wheelbase, front_hub))
}

/// The cameras of a bike `wheelbase` long with its front hub at `front_hub` ahead of the origin, from its `[bike.camera]`.
fn cameras_from(table: &Table, wheelbase: f32, front_hub: f32) -> BikeCamera {
    merge(table, BikeCamera::derived(wheelbase, front_hub))
}

/// `table` over `base`: each camera the table has replaces the base's.
fn merge(table: &Table, base: BikeCamera) -> BikeCamera {
    let chase = |own: &blackbox_modding::content::BikeCameraChase| Chase {
        distance: own.distance_m,
        height: own.height_m,
        target_ahead: own.target_ahead_m,
        target_height: own.target_height_m,
        fov: own.fov_deg,
    };
    BikeCamera {
        chase_near: table.chase_near.as_ref().map_or(base.chase_near, chase),
        chase_far: table.chase_far.as_ref().map_or(base.chase_far, chase),
        front: table.front.as_ref().map_or(base.front, |own| Front {
            position: own.position_m,
            look_back: own.look_back,
            pitch_deg: own.pitch_deg,
            fov: own.fov_deg,
            near: own.near_m,
            pitch_factor: own.pitch_factor,
            roll_factor: own.roll_factor,
        }),
        helmet: table.helmet.as_ref().map_or(base.helmet, |own| Helmet {
            eye_bones: own.eye_bones.clone(),
            head_bone: own.head_bone.clone(),
            head_eye_offset: own.head_eye_offset_m,
            hide_meshes: own.hide_meshes.clone(),
            fixed_position: own.fixed_position_m,
            pitch_deg: own.pitch_deg,
            fov: own.fov_deg,
            near: own.near_m,
            pitch_factor: own.pitch_factor,
            roll_factor: own.roll_factor,
            follow_roll_max_deg: own.follow_roll_max_deg,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        toml::from_str(text).expect("a camera table")
    }

    #[test]
    fn a_bike_without_a_camera_table_gets_the_derived_cameras() {
        let derived = cameras_from(&table(""), 1.45, 0.7372);
        assert!((derived.chase_far.distance - 2.8 * 1.45).abs() < 1e-4, "{derived:?}");
        assert!((derived.front.position[0] - (0.7372 + 1.1 * 1.45)).abs() < 1e-3, "ahead of the front hub");
    }

    #[test]
    fn a_table_replaces_the_cameras_it_has_and_keeps_the_derived_rest() {
        let text = "[helmet]
fixed_position_m = [0.1, 0.0, 1.2]
fov_deg = 91.5
hide_meshes = [\"helmet\"]
";
        let cameras = cameras_from(&table(text), 1.45, 0.7372);
        assert_eq!(cameras.helmet.fixed_position, [0.1, 0.0, 1.2]);
        assert_eq!(cameras.helmet.hide_meshes, ["helmet"]);
        assert_eq!(cameras.helmet.fov, 91.5);
        assert!((cameras.chase_near.distance - 2.3 * 1.45).abs() < 1e-4, "the chase views stay derived");
    }
}
