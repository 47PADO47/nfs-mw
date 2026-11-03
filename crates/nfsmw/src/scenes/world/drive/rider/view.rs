//! The rider as the first-person camera sees him: where his eyes are, and which of his meshes are not drawn from inside his
//! helmet (`docs/specs/ride6-bike-cameras.md`). The names of the bones and of the meshes come from the bike's `[bike.camera.helmet]`
//! table (`nfsmw_data::car::mods::bike_camera`); nothing here names a game.

use blackbox_skeleton::{Skeleton, Transform};
use glam::{Mat4, Vec3};
use nfsmw_data::car::mods::bike_camera::Helmet;

/// Where the eyes are in the rider's skeleton and what to leave out of the picture.
#[derive(Debug, Default)]
pub struct EyeView {
    /// The bones whose middle is the eye point (found in the skeleton by the table's names).
    eye_bones: Vec<usize>,
    /// The head bone and the offset from it to the eyes in the model's axes, used when no eye bone was found.
    head: Option<(usize, Vec3)>,
    /// The indices (in the rider's drawn parts) of the meshes that are not drawn from inside the helmet.
    pub hidden: Vec<usize>,
    /// The eye point in the model's frame (the bike's own, so a frame's own bike pose places it in the world), as of the last
    /// pose; `None` while the rider is not on the bike or has no eye bone.
    eye: Option<Vec3>,
}

impl EyeView {
    /// The eye view of a rider with `skeleton` whose drawn parts are the meshes `parts` (names), for the table `helmet`.
    pub fn new(helmet: Option<&Helmet>, skeleton: &Skeleton, parts: &[&str]) -> Self {
        let Some(helmet) = helmet else { return Self::default() };
        let eye_bones: Vec<usize> = helmet.eye_bones.iter().filter_map(|name| skeleton.find(name)).collect();
        let head = skeleton.find(&helmet.head_bone).map(|bone| (bone, Vec3::from_array(helmet.head_eye_offset)));
        let hidden = parts
            .iter()
            .enumerate()
            .filter(|(_, part)| helmet.hide_meshes.iter().any(|name| name.eq_ignore_ascii_case(part)))
            .map(|(at, _)| at)
            .collect();
        Self { eye_bones, head, hidden, eye: None }
    }

    /// Follow the pose `component` (the rider's component transforms), whose space is the model's by `to_model`; `on_bike` is
    /// false while he is off the bike.
    pub fn track(&mut self, component: &[Transform], to_model: Mat4, on_bike: bool) {
        self.eye = on_bike.then(|| self.eye_in(component).map(|point| to_model.transform_point3(point))).flatten();
    }

    /// The eye point in the rider's component space.
    fn eye_in(&self, component: &[Transform]) -> Option<Vec3> {
        let bones: Vec<Vec3> =
            self.eye_bones.iter().filter_map(|&bone| component.get(bone)).map(|t| t.translation).collect();
        if !bones.is_empty() {
            return Some(bones.iter().sum::<Vec3>() / bones.len() as f32);
        }
        let (bone, offset) = self.head?;
        let transform = component.get(bone)?;
        Some(transform.translation + offset)
    }

    /// The eye point in the model's frame as of the last pose.
    pub fn eye(&self) -> Option<Vec3> {
        self.eye
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    fn at(x: f32, y: f32, z: f32) -> Transform {
        Transform { translation: Vec3::new(x, y, z), rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }

    fn helmet(eye_bones: &[&str], head_bone: &str) -> Helmet {
        let mut helmet = nfsmw_data::car::mods::bike_camera::BikeCamera::derived(1.45, 0.73).helmet;
        helmet.eye_bones = eye_bones.iter().map(|name| (*name).to_owned()).collect();
        helmet.head_bone = head_bone.to_owned();
        helmet.hide_meshes = vec!["Helmet".to_owned(), "visor".to_owned()];
        helmet
    }

    fn skeleton() -> Skeleton {
        let bone = |name: &str, parent: Option<usize>| blackbox_skeleton::Bone {
            name: name.to_owned(),
            parent,
            rest: Transform::IDENTITY,
        };
        Skeleton::new(vec![bone("head", None), bone("eye_l", Some(0)), bone("eye_r", Some(0))]).expect("a skeleton")
    }

    #[test]
    fn the_eye_is_the_middle_of_the_eye_bones_in_the_models_frame() {
        let mut view = EyeView::new(Some(&helmet(&["eye_l", "eye_r", "gone"], "head")), &skeleton(), &[]);
        let component = [at(0.0, 0.0, 1.0), at(0.1, 0.03, 1.1), at(0.1, -0.03, 1.1)];
        view.track(&component, Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)), true);
        let eye = view.eye().expect("the rider has eyes");
        assert!(eye.distance(Vec3::new(10.1, 0.0, 1.1)) < 1e-5, "{eye:?}");
    }

    #[test]
    fn without_eye_bones_the_head_bone_and_its_offset_place_the_eye() {
        let mut table = helmet(&["nothing"], "head");
        table.head_eye_offset = [0.08, 0.0, 0.03];
        let mut view = EyeView::new(Some(&table), &skeleton(), &[]);
        view.track(&[at(0.0, 0.0, 1.0), at(0.0, 0.0, 0.0), at(0.0, 0.0, 0.0)], Mat4::IDENTITY, true);
        assert!(view.eye().unwrap().distance(Vec3::new(0.08, 0.0, 1.03)) < 1e-5);
    }

    #[test]
    fn a_rider_off_the_bike_or_with_no_such_bone_has_no_eye_point() {
        let component = [at(0.0, 0.0, 1.0), at(0.0, 0.0, 1.0), at(0.0, 0.0, 1.0)];
        let mut view = EyeView::new(Some(&helmet(&["eye_l"], "head")), &skeleton(), &[]);
        view.track(&component, Mat4::IDENTITY, false);
        assert_eq!(view.eye(), None, "off the bike");
        let mut view = EyeView::new(Some(&helmet(&["nothing"], "nothing")), &skeleton(), &[]);
        view.track(&component, Mat4::IDENTITY, true);
        assert_eq!(view.eye(), None, "no bone found");
        assert_eq!(EyeView::new(None, &skeleton(), &[]).eye(), None);
    }

    #[test]
    fn the_helmet_and_the_visor_are_the_parts_left_out_whatever_the_case_of_their_names() {
        let view = EyeView::new(Some(&helmet(&[], "head")), &skeleton(), &["suit", "helmet", "boots", "VISOR"]);
        assert_eq!(view.hidden, [1, 3]);
    }
}
