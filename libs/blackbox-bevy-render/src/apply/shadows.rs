//! One shadow-only sun follows the frame light. Bevy owns stable cascade fitting and spatial PCF.

use bevy_ecs::{entity::Entity, system::Commands};
use bevy_light::{CascadeShadowConfigBuilder, DirectionalLight, DirectionalLightShadowMap};
use bevy_math::Vec3;
use bevy_transform::components::{GlobalTransform, Transform};
use blackbox_gfx::settings::ShadowTuning;
use blackbox_gfx::settings::shadows::divided_edge;
use blackbox_gfx::{FrameParams, ShadowQuality};

/// The depth and normal bias the sun has before a scene's bias multiplier.
const DEPTH_BIAS: f32 = 0.02;
const NORMAL_BIAS: f32 = 1.0;

/// The last sun configuration, to avoid rewriting its cascade config every frame.
pub struct Sun {
    pub entity: Entity,
    tuning: ShadowTuning,
    direction: Vec3,
}

/// Makes the sun match `frame` and `tuning`. The map size is one resource shared by every light, so a frame with other
/// tuning than the last one reconfigures it: a front-end scene's divider does not stay on the world.
pub fn sync(commands: &mut Commands, sun: &mut Option<Sun>, frame: &FrameParams, tuning: ShadowTuning) {
    if tuning.quality == ShadowQuality::Off {
        if let Some(sun) = sun.take() {
            commands.entity(sun.entity).despawn();
        }
        return;
    }
    let direction = crate::axes::point(frame.light_dir);
    let direction = Vec3::from_array(direction.to_array()).try_normalize().unwrap_or(Vec3::NEG_Y);
    let transform = Transform::default().looking_to(direction, Vec3::Z);
    let global = GlobalTransform::from(transform);
    if let Some(sun) = sun.as_mut() {
        if sun.tuning != tuning {
            commands.entity(sun.entity).insert(light(tuning));
            configure(commands, sun.entity, tuning);
            sun.tuning = tuning;
        }
        if sun.direction != direction {
            commands.entity(sun.entity).insert((transform, global));
            sun.direction = direction;
        }
        return;
    }
    let entity = commands.spawn((light(tuning), transform, global)).id();
    configure(commands, entity, tuning);
    *sun = Some(Sun { entity, tuning, direction });
}

/// The shadow-only light for a tuning: no radiance, the bias the tuning scales.
fn light(tuning: ShadowTuning) -> DirectionalLight {
    DirectionalLight {
        shadow_maps_enabled: true,
        // Custom shading consumes visibility, not Bevy's radiance.
        illuminance: 0.0,
        shadow_depth_bias: DEPTH_BIAS * tuning.bias,
        shadow_normal_bias: NORMAL_BIAS * tuning.bias,
        ..Default::default()
    }
}

fn configure(commands: &mut Commands, entity: Entity, tuning: ShadowTuning) {
    let (size, num_cascades, maximum_distance) = tuning.quality.sampling();
    let size = divided_edge(size, tuning.divider);
    commands.insert_resource(DirectionalLightShadowMap { size });
    commands.entity(entity).insert(
        CascadeShadowConfigBuilder {
            num_cascades,
            minimum_distance: 0.1,
            first_cascade_far_bound: 16.0,
            maximum_distance,
            overlap_proportion: 0.2,
        }
        .build(),
    );
}

/// Translucency and the sky must never stamp solid silhouettes into the shadow map.
pub fn casts(shading: crate::material::ShadingKind, blend: crate::material::BlendKind) -> bool {
    use crate::material::{BlendKind, ShadingKind};
    shading != ShadingKind::Sky && matches!(blend, BlendKind::Opaque | BlendKind::AlphaTest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{BlendKind, ShadingKind};

    #[test]
    fn only_solid_and_cutout_surfaces_cast() {
        for shading in [ShadingKind::Lit, ShadingKind::Prelit, ShadingKind::Glossy] {
            assert!(casts(shading, BlendKind::Opaque));
            assert!(casts(shading, BlendKind::AlphaTest));
            assert!(!casts(shading, BlendKind::AlphaBlend));
            assert!(!casts(shading, BlendKind::Additive));
        }
        assert!(!casts(ShadingKind::Sky, BlendKind::Opaque));
    }

    #[test]
    fn the_bias_multiplier_scales_both_biases_and_the_world_keeps_the_renderer_values() {
        let world = light(ShadowTuning::world(ShadowQuality::Medium));
        assert_eq!((world.shadow_depth_bias, world.shadow_normal_bias), (0.02, 1.0));
        let scaled = light(ShadowTuning { quality: ShadowQuality::Medium, bias: 2.0, divider: 1 });
        assert_eq!((scaled.shadow_depth_bias, scaled.shadow_normal_bias), (0.04, 2.0));
    }
}
