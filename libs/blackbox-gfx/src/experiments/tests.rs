//! Bounds of the render experiment controls (`RenderExperiments::sanitized`).
mod ray_shadow_tests {
    use super::super::RenderExperiments;
    #[test]
    fn invalid_sun_ray_controls_are_bounded_and_old_profiles_stay_opt_in() {
        let defaults = RenderExperiments::default();
        assert!(!defaults.ray_sun_shadows);
        let invalid = RenderExperiments {
            ray_shadow_samples: 100,
            ray_shadow_resolution: 999,
            far_shadow_scale: f32::INFINITY,
            ray_shadow_filter: f32::NAN,
            ray_shadow_range: f32::NAN,
            ray_shadow_length: f32::INFINITY,
            ray_shadow_bias: -1.0,
            ..defaults
        }
        .sanitized();
        assert_eq!(invalid.ray_shadow_samples, 16);
        assert_eq!(RenderExperiments { ray_shadow_samples: 2, ..defaults }.sanitized().ray_shadow_samples, 2);
        assert_eq!(RenderExperiments { ray_shadow_samples: 0, ..defaults }.sanitized().ray_shadow_samples, 4);
        assert_eq!(invalid.ray_shadow_resolution, 100);
        assert_eq!(invalid.far_shadow_scale, 3.0);
        let range = RenderExperiments {
            sun_shadows: true,
            far_shadows: true,
            shadow_distance: 160.0,
            far_shadow_scale: 2.5,
            ..defaults
        };
        assert_eq!(range.caster_distance(), 400.0);
        assert_eq!(invalid.ray_shadow_filter, 0.75);
        assert_eq!(invalid.ray_shadow_range, 100.0);
        assert_eq!(invalid.ray_shadow_length, 300.0);
        assert_eq!(invalid.ray_shadow_bias, 0.001);
        let large =
            RenderExperiments { ray_shadow_range: 900.0, ray_shadow_length: 900.0, ray_shadow_bias: 1.0, ..defaults }
                .sanitized();
        assert_eq!((large.ray_shadow_range, large.ray_shadow_length, large.ray_shadow_bias), (240.0, 800.0, 0.05));
    }
}

mod light_fog_tests {
    use super::super::RenderExperiments;
    #[test]
    fn the_fog_light_budget_is_bounded_and_defaults_to_the_old_haze_count() {
        let defaults = RenderExperiments::default();
        assert_eq!(defaults.local_light_fog_count, defaults.local_street_haze_count);
        let large = RenderExperiments { local_light_fog_count: 500, ..defaults }.sanitized();
        assert_eq!(large.local_light_fog_count, 64);
        assert_eq!(RenderExperiments { local_light_fog_count: 0, ..defaults }.sanitized().local_light_fog_count, 0);
    }
}

mod night_tests {
    use super::super::RenderExperiments;
    #[test]
    fn night_car_and_moonlight_controls_are_bounded_and_invalid_values_take_the_defaults() {
        let defaults = RenderExperiments::default();
        let invalid = RenderExperiments {
            car_night_reflections: f32::NAN,
            car_light_strength: f32::INFINITY,
            car_lamp_size: f32::NAN,
            moonlight: f32::NAN,
            moon_cloud_block: f32::NEG_INFINITY,
            ..defaults
        }
        .sanitized();
        assert_eq!(
            (invalid.car_night_reflections, invalid.car_light_strength, invalid.car_lamp_size),
            (2.0, 2.0, 0.25)
        );
        assert_eq!((invalid.moonlight, invalid.moon_cloud_block), (0.2, 0.9));
        let large = RenderExperiments {
            car_night_reflections: 10.0,
            car_light_strength: 10.0,
            car_lamp_size: 5.0,
            moonlight: 2.0,
            moon_cloud_block: 2.0,
            ..defaults
        }
        .sanitized();
        assert_eq!((large.car_night_reflections, large.car_light_strength, large.car_lamp_size), (3.0, 5.0, 1.0));
        assert_eq!((large.moonlight, large.moon_cloud_block), (1.0, 1.0));
        let small = RenderExperiments { car_night_reflections: 0.0, moonlight: -1.0, ..defaults }.sanitized();
        assert_eq!((small.car_night_reflections, small.moonlight), (0.25, 0.0));
    }
}
