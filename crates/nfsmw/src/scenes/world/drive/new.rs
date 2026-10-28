use glam::Vec3;
use nfsmw_data::car::physics::CarPhysics;

use super::breaker_camera::Zoom;
use super::cracked::CrackedZones;
use super::fall::FallWatch;
use super::input::Presses;
use super::look::PendingLook;
use super::sim::Telemetry;
use super::sound::SoundFeed;
use super::speedbreaker::SpeedBreaker;
use super::{CarPose, CarRig, ContactMarkers, Drive, DriveInput, DriveScript, ScriptRun, SpawnRequest};
use crate::scenes::world::effects::TireEffects;
use crate::scenes::world::exhaust::ExhaustFlames;
use crate::scenes::world::vehicle_effects::VehicleEffects;
use crate::settings::{Transmission, WheelOptions};
use crate::viewer::camera::{CameraCut, ChaseCamera, PursuitCamera};

impl Drive {
    pub fn new(
        car_name: String,
        rig: CarRig,
        physics: CarPhysics,
        request: SpawnRequest,
        script: Option<DriveScript>,
        visuals: nfsmw_data::vehicle_effects::VisualEffectsData,
    ) -> Self {
        let pose = CarPose {
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            wheels: [nfsmw_data::car::WheelPose::default(); 4],
        };
        Self {
            car_name,
            effects: TireEffects::default(),
            brake_lamp: 0.0,
            steer_lamp: 0.0,
            lamp_time: 0.0,
            headlights: true,
            vehicle_effects: VehicleEffects::new(visuals),
            flames: ExhaustFlames::default(),
            tail_trails: Default::default(),
            tail_style: Default::default(),
            performance: Default::default(),
            tuning: Default::default(),
            tunings: Default::default(),
            rig,
            physics,
            sim: None,
            request: Some(request),
            alpha: 0.0,
            frame_input: DriveInput::default(),
            remote_input: None,
            cinematic_input: None,
            arrested: false,
            breaker_barred: false,
            sabotaged: false,
            engine: Default::default(),
            gridded: false,
            launch_cancel_on_shift: false,
            assist_aim: None,
            pending_speed: 0.0,
            previous: pose,
            current: pose,
            nis_pose: None,
            net_pose: None,
            telemetry: Telemetry::default(),
            gauges: Default::default(),
            breaker: SpeedBreaker::default(),
            breaker_camera: Zoom::default(),
            sound: SoundFeed::default(),
            chase: ChaseCamera::default(),
            pursuit: PursuitCamera::default(),
            queued_hits: Vec::new(),
            cracked: CrackedZones::default(),
            cut: CameraCut::default(),
            look: PendingLook::default(),
            script: script.map(|script| ScriptRun { script, time: 0.0, previous_time: None }),
            batch_run: false,
            steps: 0,
            manual_resets: 0,
            last_good: None,
            strict_since: None,
            fall: FallWatch::default(),
            road_check_elapsed: 0.0,
            presses: Presses::default(),
            transmission: Transmission::default(),
            wheel: WheelOptions::default(),
            markers: ContactMarkers::default(),
            pending_knocks: Vec::new(),
            prop_touch: Default::default(),
        }
    }
}
