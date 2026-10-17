//! Eligibility follows active material terms; it never mutates stored requests.
use crate::settings::CarShading;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ContactCar {
    Direct,
    ProjectedBypass,
    ShadowlessBypass,
}

pub(super) struct Dependencies {
    pub custom_car: bool,
    pub contact_car: ContactCar,
}

pub(super) fn for_selection(car: CarShading, _original_world: bool, projected_shadow: bool) -> Dependencies {
    let contact_car = match car {
        CarShading::OriginalUnshadowed => ContactCar::ShadowlessBypass,
        CarShading::Original if projected_shadow => ContactCar::ProjectedBypass,
        _ => ContactCar::Direct,
    };
    Dependencies { custom_car: car == CarShading::Glossy, contact_car }
}

/// A shadow-family selection owns the projection, not unrelated enhancement requests.
pub(super) fn select_shadow_system(options: &mut blackbox_gfx::RenderExperiments, original: bool) {
    options.shadow_projection = match original {
        true => Some(crate::settings::appearance::shadows::PROJECTION),
        false => None,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn car_enhancements_follow_custom_material_terms_without_disabling_world_contacts() {
        for car in CarShading::ALL {
            for original_world in [false, true] {
                for projected in [false, true] {
                    let dependencies = for_selection(car, original_world, projected);
                    assert_eq!(dependencies.custom_car, matches!(car, CarShading::Glossy));
                    let expected = match (car, projected) {
                        (CarShading::OriginalUnshadowed, _) => ContactCar::ShadowlessBypass,
                        (CarShading::Original, true) => ContactCar::ProjectedBypass,
                        _ => ContactCar::Direct,
                    };
                    assert_eq!(dependencies.contact_car, expected);
                }
            }
        }
        // The car-specific projected bypass must not imply a global contact disable.
        let original = for_selection(CarShading::Original, true, true);
        assert_eq!(original.contact_car, ContactCar::ProjectedBypass);
        let cascaded = for_selection(CarShading::Original, true, false);
        assert_eq!(cascaded.contact_car, ContactCar::Direct);
    }

    #[test]
    fn projection_selection_preserves_contact_and_material_enhancement_requests() {
        let mut options = blackbox_gfx::RenderExperiments {
            contact_shadows: true,
            contact_length: 0.7,
            clearcoat: true,
            car_skylight: true,
            sky_strength: 1.5,
            world_ambient: true,
            ..Default::default()
        };
        let before = options;
        select_shadow_system(&mut options, true);
        let mut expected = before;
        expected.shadow_projection = Some(crate::settings::appearance::shadows::PROJECTION);
        assert_eq!(options, expected);
        select_shadow_system(&mut options, false);
        assert_eq!(options, before);
    }
}
