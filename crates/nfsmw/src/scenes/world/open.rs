//! Opening the world scene: the start of a track, its streaming, cars and traffic from the install.

use super::*;

impl WorldScene {
    pub fn open(dir: &GameDir, options: Options) -> Result<Self> {
        let index = WorldIndex::open(dir, DEFAULT_TRACK)?;
        let mut physics = PhysicsData::load(dir)?;
        physics.set_sequences(index.sequences.clone());
        let colour_treatment = nfsmw_data::visual_treatment::midday(physics.database());
        let time_of_day = nfsmw_data::time_of_day::load(physics.database());
        let camera_views = nfsmw_data::camera::load(physics.database());
        if colour_treatment.is_none() {
            log::warn!("midday visual treatment is missing or invalid; grading disabled");
        }
        let globals = load_global_textures(dir)?;
        let gain_map = nfsmw_data::visual_treatment::gain_map(&globals.textures);
        let mut traffic = index.road_network.clone().map(ai::TrafficWorld::new);
        if let Some(world) = traffic.as_mut() {
            world.load_data(physics.database(), index.track_zones.clone());
            match nfsmw_data::world::heli_sheet::read(dir, &index.stream_file) {
                Ok(sheet) => {
                    log::info!("helicopter sheet: {} triangles", sheet.len());
                    world.set_heli_sheet(sheet);
                }
                Err(e) => log::warn!("helicopter sheet: {e}; the helicopter flies at a flat floor"),
            }
        }
        let props = PropCatalog::new(index.prop_bounds.clone(), physics.database())
            .with_hierarchies(index.model_hierarchies.clone());
        log::info!("{} props have collision bounds", props.len());
        let stream = dir.resolve(&index.stream_file).with_context(|| format!("{} is missing", index.stream_file))?;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
        let streamer = Streamer::spawn(stream.to_path_buf(), index.sections.clone(), threads);
        let start = options.at.unwrap_or_else(|| index.centre());
        let camera = FlyCamera {
            position: Vec3::new(start[0], start[1], options.height),
            yaw: options.heading.to_radians(),
            pitch: options.pitch.to_radians(),
            speed: 40.0,
        };
        log::info!("camera starts at ({:.0}, {:.0}), {:.0} m above the ground", start[0], start[1], options.height);
        let pending_car = options
            .drive
            .map(|d| {
                let (name, model) = load_car(dir, &d.car)?;
                let script = d.script.as_deref().map(DriveScript::parse).transpose().map_err(anyhow::Error::msg)?;
                Ok::<_, anyhow::Error>(PendingCar { name, model, script })
            })
            .transpose()?;
        let (map_blips, entry_zones) = match nfsmw_data::gameplay::load(dir) {
            Ok(data) => (super::hud::static_blips(&data), entry_zones::EntryZones::from_data(&data)),
            Err(e) => {
                log::warn!("race and interior blips and entry zones are off: {e:#}");
                (Vec::new(), entry_zones::EntryZones::default())
            }
        };
        Ok(Self {
            garage: garage::catalog(dir, &physics)?,
            garage_parts: None,
            garage_open: false,
            dir: dir.clone(),
            track: index.track.clone(),
            camera,
            residency: Residency::new(index.sections, index.visible, streamer, globals, index.collision_grid, props),
            start,
            start_height: options.height,
            // The car is placed on a road, not the camera on the ground.
            grounded: pending_car.is_some(),
            wait_for_load: options.wait_for_load,
            fog_distance: options.fog_distance,
            visible: Vec::new(),
            rear_visible: Vec::new(),
            colour_treatment,
            time_of_day,
            camera_views,
            gain_map,
            reflection_faces: std::array::from_fn(|_| Vec::new()),
            reflection_last: None,
            clock: 0.0,
            sun: sun::Sun::new(),
            physics_aspect: 16.0 / 9.0,
            physics_clock: drive::FixedClock::default(),
            speedbreaker_clock: drive::SpeedbreakerClock::default(),
            view: if pending_car.is_some() { View::Chase } else { View::Fly },
            last_car: pending_car.as_ref().map_or_else(|| DEFAULT_CAR.to_owned(), |c| c.name.clone()),
            pending_car,
            drive: None,
            traffic,
            map_blips,
            entry: entry::Entry { zones: entry_zones, popup: Default::default() },
            traffic_setting: crate::settings::TrafficSettings {
                cars: 0,
                cop_share: crate::settings::Percent(0),
                lights: false,
            },
            lamp_meshes: None,
            traffic_manual: None,
            physics,
            tire_effects: [true; 2],
            vehicle_effects: [false; 2],
            exhaust_flames: false,
            spark_style: crate::settings::SparkStyle::OriginalPc,
            smoke_quality: crate::settings::SmokeQuality::Standard,
            rain: crate::weather::rain::Rain::new(),
            tuning: crate::settings::WorldTuning::default(),
            car_shading: crate::settings::CarShading::default(),
            markers_on: false,
            marker_meshes: None,
            cut: CameraCut::default(),
            waiting_since: None,
            camera_mode: CameraMode::default(),
        })
    }
}
