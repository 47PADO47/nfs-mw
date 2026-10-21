//! Opening the world scene: the start of a track, its streaming, cars and traffic from the install.

use super::*;

/// A world read from the install and not yet put together: [`WorldScene::load`] does the slow part (the tables, the
/// physics, the car, the streaming index) and holds nothing of the renderer, so it can run on a thread; [`build`] runs
/// on the thread that owns the scene (the scene holds `Rc`s, so it cannot move between threads).
///
/// [`build`]: WorldLoad::build
pub struct WorldLoad(Box<dyn FnOnce() -> WorldScene + Send>);

impl WorldLoad {
    /// Puts the world together: starts its streaming threads and makes the scene.
    pub fn build(self) -> WorldScene {
        (self.0)()
    }
}

impl WorldScene {
    pub fn open(dir: &GameDir, options: Options) -> Result<Self> {
        Ok(Self::load(dir, options)?.build())
    }

    /// Reads the world from the install (see [`WorldLoad`]).
    pub fn load(dir: &GameDir, options: Options) -> Result<WorldLoad> {
        let drive = options.drive;
        // These loads need nothing another one produces, so they run at once. Every thread is joined before
        // any result is used, and errors are reported in the order the steps used to run.
        let (index, audio_environment, physics, globals, car, gameplay) = std::thread::scope(|s| {
            let index = s.spawn(|| WorldIndex::open(dir, DEFAULT_TRACK));
            let audio = s.spawn(|| audio_environment(dir));
            let physics = s.spawn(|| PhysicsData::load(dir));
            let globals = s.spawn(|| load_global_textures(dir));
            let car = s.spawn(move || drive.map(|d| load_pending_car(dir, d)).transpose());
            let gameplay = s.spawn(|| nfsmw_data::gameplay::load(dir));
            (joined(index), joined(audio), joined(physics), joined(globals), joined(car), joined(gameplay))
        });
        let mut index = index?;
        let override_groups = std::mem::take(&mut index.override_groups);
        let group_table = std::mem::take(&mut index.scenery_groups);
        let mut physics = physics?;
        physics.set_sequences(index.sequences.clone());
        let globals = globals?;
        let stream =
            dir.resolve(&index.stream_file).with_context(|| format!("{} is missing", index.stream_file))?.to_path_buf();
        let pending_car = car?;
        if let Some(environment) = &audio_environment {
            log::info!(
                "audio environment: map {}, zones {}",
                environment.map_available(),
                environment.zones_available()
            );
        }
        let colour_treatment = nfsmw_data::visual_treatment::midday(physics.database());
        let time_of_day = nfsmw_data::time_of_day::load(physics.database());
        let atmosphere = nfsmw_data::read_unwrapped(dir, &format!("TRACKS/{DEFAULT_TRACK}.BUN"))
            .and_then(|bytes| lighting::Clouds::read(&bytes))
            .map_err(|error| log::warn!("authored cloud map unavailable: {error:#}"))
            .unwrap_or_default();
        let camera_views = nfsmw_data::camera::load(physics.database());
        if colour_treatment.is_none() {
            log::warn!("midday visual treatment is missing or invalid; grading disabled");
        }
        let gain_map = nfsmw_data::visual_treatment::gain_map(&globals.textures);
        let breaker_visuals = breaker_visuals::Presentation::new(
            nfsmw_data::visual_treatment::speedbreaker::load(physics.database()),
            nfsmw_data::visual_treatment::speedbreaker::mask(&globals.textures),
        );
        // The traffic world holds `Rc`s, so it is made in `build`; the road network and the helicopter sheet are read here.
        let road_network = index.road_network.clone();
        let heli_sheet = road_network.is_some().then(|| {
            nfsmw_data::world::heli_sheet::read(dir, &index.stream_file)
                .inspect(|sheet| log::info!("helicopter sheet: {} triangles", sheet.len()))
                .map_err(|e| log::warn!("helicopter sheet: {e}; the helicopter flies at a flat floor"))
                .ok()
        });
        let props = PropCatalog::new(index.prop_bounds.clone(), physics.database())
            .with_hierarchies(index.model_hierarchies.clone());
        log::info!("{} props have collision bounds", props.len());
        // The mods' map packs take residency indices after the install's sections; their interiors refer to them
        // by those.
        let installed: std::collections::HashSet<i16> = index.sections.iter().map(|s| s.number).collect();
        let first_pack = index.sections.len();
        // The garage catalog reads the physics, sequences included, so it waits for `set_sequences` above. The mod
        // packs need neither and are read on this thread while the catalog is built.
        let (garage_catalog, map_packs, interiors) = std::thread::scope(|s| {
            let garage_catalog = s.spawn(|| garage::catalog(dir, &physics));
            let map_packs =
                crate::modding::content::scenery::packs(dir, crate::modding::content::mod_dirs(), &installed);
            let interiors =
                crate::modding::content::interior::load(crate::modding::content::mod_dirs(), &map_packs, first_pack);
            (joined(garage_catalog), map_packs, interiors)
        });
        let garage_catalog = garage_catalog?;
        let (map_blips, race_starts, entry_zones, gameplay) = match gameplay {
            Ok(data) => (
                super::hud::interior_blips(&data),
                super::hud::race_start_blips(&data),
                entry_zones::EntryZones::from_data(&data),
                Some(std::sync::Arc::new(data)),
            ),
            Err(e) => {
                log::warn!("race and interior blips, entry zones and races are off: {e:#}");
                (Vec::new(), Vec::new(), entry_zones::EntryZones::default(), None)
            }
        };
        let entry_zones = entry_zones.with_interiors(interiors);
        let dir = dir.clone();
        Ok(WorldLoad(Box::new(move || {
            let mut traffic = road_network.map(ai::TrafficWorld::new);
            if let Some(world) = traffic.as_mut() {
                world.load_data(physics.database(), index.track_zones.clone());
                world.set_roadblock_physics(&props);
                world.set_track_barriers(std::mem::take(&mut index.track_barriers));
                if let Some(sheet) = heli_sheet.flatten() {
                    world.set_heli_sheet(sheet);
                }
            }
            let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
            let streamer = Streamer::spawn(stream, index.sections.clone(), threads);
            let start = options.at.unwrap_or_else(|| index.centre());
            let camera = FlyCamera {
                position: Vec3::new(start[0], start[1], options.height),
                yaw: options.heading.to_radians(),
                pitch: options.pitch.to_radians(),
                speed: 40.0,
            };
            log::info!("camera starts at ({:.0}, {:.0}), {:.0} m above the ground", start[0], start[1], options.height);
            let mut scene = Self {
                detail: Default::default(),
                garage: garage_catalog,
                garage_parts: None,
                garage_open: false,
                audio_environment,
                common_sounds: Vec::new(),
                sound_speedbreaker_active: false,
                dir,
                track: index.track.clone(),
                camera,
                residency: {
                    let mut residency = Residency::new(
                        index.sections,
                        index.visible,
                        streamer,
                        globals,
                        index.collision_grid,
                        props,
                        map_packs,
                    );
                    residency.set_override_groups(override_groups);
                    residency
                },
                start,
                start_height: options.height,
                // The car is placed on a road, not the camera on the ground.
                grounded: pending_car.is_some(),
                wait_for_load: options.wait_for_load,
                fog_distance: options.fog_distance,
                visible: Vec::new(),
                rear_visible: Vec::new(),
                shadow_quality: blackbox_gfx::ShadowQuality::Off,
                shadow_feed: Default::default(),
                original_shadows: true,
                original_sky: false,
                colour_treatment,
                time_of_day,
                atmosphere,
                camera_environment: camera_original::Environment::new(
                    camera_views,
                    index.track_zones.clone().unwrap_or_default(),
                ),
                original_camera: Default::default(),
                camera_ground: Default::default(),
                original_player_step: None,
                gain_map,
                breaker_visuals,
                reflection_faces: std::array::from_fn(|_| Vec::new()),
                reflection_last: None,
                clock: 0.0,
                sun: sun::Sun::new(),
                day: super::day::DayClock::new(nfsmw_data::day_night::DayNightConfig::default()),
                physics_aspect: 16.0 / 9.0,
                physics_clock: drive::FixedClock::default(),
                speedbreaker_clock: drive::SpeedbreakerClock::default(),
                view: if pending_car.is_some() { View::Chase } else { View::Fly },
                last_car: pending_car.as_ref().map_or_else(|| DEFAULT_CAR.to_owned(), |c| c.name.clone()),
                speech_look: Default::default(),
                pending_car,
                drive: None,
                traffic,
                map_blips,
                race_starts,
                gameplay,
                race: None,
                queued_race: None,
                queued_split: None,
                split: None,
                race_mods: Default::default(),
                mod_blips: Vec::new(),
                career: Default::default(),
                scenery_groups: scenery_groups::SceneryGroups::new(group_table),
                entry: entry::Entry {
                    zones: entry_zones,
                    popup: Default::default(),
                    interior: Default::default(),
                    current: None,
                },
                traffic_setting: crate::settings::TrafficSettings {
                    cars: 0,
                    cop_share: crate::settings::Percent(0),
                    lights: false,
                    avoidance: false,
                    policy: Default::default(),
                },
                lamp_meshes: None,
                traffic_manual: None,
                physics,
                tire_effects: [true; 2],
                vehicle_effects: [false; 2],
                speed_trail_style: crate::settings::SpeedTrailStyle::default(),
                exhaust_flames: false,
                spark_style: crate::settings::SparkStyle::OriginalPc,
                smoke_quality: crate::settings::SmokeQuality::Standard,
                rain: crate::weather::rain::Rain::new(),
                wet_road: Default::default(),
                explosions: Default::default(),
                heli_wash: Default::default(),
                finish_flares: Default::default(),
                object_fx: Default::default(),
                car_damage_fx: Default::default(),
                impact_fx: Default::default(),
                tuning: crate::settings::WorldTuning::default(),
                car_shading: crate::settings::CarShading::default(),
                markers_on: false,
                marker_meshes: None,
                guide: Default::default(),
                cut: CameraCut::default(),
                waiting_since: None,
                camera_mode: CameraMode::default(),
                cop_health: Default::default(),
                moment: Default::default(),
                photo_shot: None,
                breaker_icons: Default::default(),
            };
            scene.switch_default_scenery_groups();
            scene
        })))
    }
}

/// The audio environment of the track, or `None` (with a warning) when it cannot be read.
fn audio_environment(dir: &GameDir) -> Option<crate::audio::environment_context::EnvironmentMap> {
    nfsmw_data::shared_unwrapped(dir, &format!("TRACKS/{DEFAULT_TRACK}.BUN"))
        .and_then(|bytes| crate::audio::environment_context::EnvironmentMap::read(&bytes))
        .map_err(|e| log::warn!("audio environment unavailable: {e:#}"))
        .ok()
}

/// A car waiting for the renderer to exist.
pub(super) struct PendingCar {
    pub(super) name: String,
    pub(super) model: CarModel,
    pub(super) script: Option<DriveScript>,
}

/// The player's car for `--drive`: its model and its driving script.
fn load_pending_car(dir: &GameDir, d: DriveOptions) -> Result<PendingCar> {
    let (name, model) = super::car_loader::load_named_car(dir, &d.car)?;
    let script = d.script.as_deref().map(DriveScript::parse).transpose().map_err(anyhow::Error::msg)?;
    Ok(PendingCar { name, model, script })
}

/// The result of a scoped load. A panic on its thread is raised again here, as it would be on this thread.
fn joined<T>(handle: std::thread::ScopedJoinHandle<'_, T>) -> T {
    handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}
