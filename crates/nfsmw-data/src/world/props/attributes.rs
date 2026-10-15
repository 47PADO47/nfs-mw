//! Original `smackable` data; `docs/specs/prop-attributes.md`.

use blackbox_attrib::CollectionRef;
use blackbox_collision::Shape;
use blackbox_vehicle::rigid_body::RigidBodySpec;
use glam::Vec3;

use crate::car::physics::{Fields, rigid_body_spec};

#[derive(Debug, Clone)]
pub struct PropDynamics {
    pub sound_profile: u32,
    pub mass: f32,
    pub tensor_scale: Vec3,
    pub body: RigidBodySpec,
    pub fallback_shape: Shape,
    pub base_surface: u32,
    pub detach_impulse: f32,
    pub no_car_effect: bool,
    /// Eligible for the native camera's object-avoidance query.
    pub camera_avoidable: bool,
    pub immobile: bool,
    pub respawn_time: f32,
    pub kill_off_screen: f32,
    pub dropout: [f32; 2],
    pub allow_off_world: bool,
    pub event_sequencer: u32,
    pub start_sequencer: bool,
    pub no_trigger: bool,
    pub explosion_effect: f32,
}

impl PropDynamics {
    pub fn read(collection: CollectionRef<'_>) -> Option<Self> {
        let fields = Fields(collection);
        let body = fields.link("rigidbodyspecs")?;
        let mass = fields.f32("MASS");
        if !mass.is_finite() || mass <= 0.0 {
            return None;
        }
        let moment = fields.vec3("MOMENT");
        let tensor_scale = Vec3::from_array(moment.to_array().map(|v| match v > 0.0 {
            true => 2.0 * v,
            false => 2.0,
        }));
        Some(Self {
            sound_profile: collection.key(),
            mass,
            tensor_scale,
            body: rigid_body_spec(body),
            fallback_shape: match body.0.get_str("DEFAULT_COL_BOX") {
                Some("SPHERE") => Shape::Sphere,
                _ => Shape::Box,
            },
            base_surface: body.0.get_string_key("BASE_MATERIAL").map_or(0, |k| k.hash32),
            detach_impulse: fields.f32("DETACH_FORCE"),
            no_car_effect: fields.bool("NO_CAR_EFFECT"),
            camera_avoidable: fields.bool("CAMERA_AVOIDABLE"),
            immobile: body.bool("IMMOBILE_OBJECT_COLLISIONS"),
            respawn_time: fields.f32("RESPAWN_TIME"),
            kill_off_screen: fields.f32("KILL_OFF_SCREEN"),
            dropout: fields.pair("DROPOUT"),
            allow_off_world: fields.bool("ALLOW_OFF_WORLD"),
            event_sequencer: collection.get_string_key("EventSequencer").map_or(0, |k| k.hash32),
            start_sequencer: fields.bool("start_sequencer"),
            no_trigger: fields.bool("no_trigger"),
            explosion_effect: fields.f32("ExplosionEffect"),
        })
    }
}
