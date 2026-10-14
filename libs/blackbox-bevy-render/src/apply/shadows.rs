//! One shadow-only sun follows the frame light. Bevy owns stable cascade fitting and spatial PCF.

use bevy_ecs::{entity::Entity, system::Commands};
use bevy_light::{CascadeShadowConfigBuilder, DirectionalLight, DirectionalLightShadowMap};
use bevy_math::Vec3;
use bevy_transform::components::{GlobalTransform, Transform};
use blackbox_gfx::{FrameParams, ShadowQuality};

/// The last sun configuration, to avoid rewriting its cascade config every frame.
pub struct Sun {
    pub entity: Entity,
    quality: ShadowQuality,
    direction: Vec3,
}

pub fn sync(commands: &mut Commands, sun: &mut Option<Sun>, frame: &FrameParams, quality: ShadowQuality) {
    if quality == ShadowQuality::Off {
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
        if sun.quality != quality {
            configure(commands, sun.entity, quality);
            sun.quality = quality;
        }
        if sun.direction != direction {
            commands.entity(sun.entity).insert((transform, global));
            sun.direction = direction;
        }
        return;
    }
    let entity = commands
        .spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                // Custom shading consumes visibility, not Bevy's radiance.
                illuminance: 0.0,
                shadow_depth_bias: 0.02,
                shadow_normal_bias: 1.0,
                ..Default::default()
            },
            transform,
            global,
        ))
        .id();
    configure(commands, entity, quality);
    *sun = Some(Sun { entity, quality, direction });
}

fn configure(commands: &mut Commands, entity: Entity, quality: ShadowQuality) {
    let (size, num_cascades, maximum_distance) = quality.sampling();
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
}
