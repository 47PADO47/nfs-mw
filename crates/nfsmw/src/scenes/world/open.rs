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
        let _trace = blackbox_trace::scope("World: build on the frame thread");
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
        let at = options.at;
        // These loads need nothing another one produces, so they run at once. Every thread is joined before
        // any result is used, and errors are reported in the order the steps used to run. The index thread goes on
        // to start the streaming: the shared sets and the tiles around the start load while everything else is read.
        let (index, audio_environment, physics, globals, car, gameplay, atmosphere) = std::thread::scope(|s| {
            let index = s.spawn(|| start_streaming(dir, at));
            let audio = s.spawn(|| audio_environment(dir));
            let physics = s.spawn(|| PhysicsData::load(dir));
            let globals = s.spawn(|| load_global_textures(dir));
            let car = s.spawn(move || drive.map(|d| load_pending_car(dir, d)).transpose());
            let gameplay = s.spawn(|| nfsmw_data::gameplay::cached(dir));
            let atmosphere = s.spawn(|| atmosphere(dir));
            (
                joined(index),
                joined(audio),
                joined(physics),
                joined(globals),
                joined(car),
                joined(gameplay),
                joined(atmosphere),
            )
        });
        let (mut index, started) = index?;
        let override_groups = std::mem::take(&mut index.override_groups);
        let group_table = std::mem::take(&mut index.scenery_groups);
        let mut physics = physics?;
        physics.set_sequences(index.sequences.clone());
        let globals = globals?;
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
        let props = PropCatalog::new(index.prop_bounds.clone(), physics.database())
            .with_hierarchies(index.model_hierarchies.clone());
        // The loader threads place each section's scenery as far as it needs no renderer. A catalog of their own,
        // built from the same tables, keeps them off the one the frame thread uses.
        prepare_on_loaders(
            &started.streamer,
            PropCatalog::new(index.prop_bounds.clone(), physics.database())
                .with_hierarchies(index.model_hierarchies.clone()),
            override_groups.clone(),
        );
        log::info!("{} props have collision bounds", props.len());
        // The mods' map packs take residency indices after the install's sections; their interiors refer to them
        // by those.
        let installed: std::collections::HashSet<i16> = index.sections.iter().map(|s| s.number).collect();
        let first_pack = index.sections.len();
        // The garage catalog reads the physics, sequences included, so it waits for `set_sequences` above. The mod
        // packs need neither and are read on this thread while the catalog is built. The helicopter sheet walks the
        // stream file's chunk headers, which takes a while, so it runs beside them.
        let (garage_catalog, map_packs, interiors, heli_sheet) = std::thread::scope(|s| {
            let garage_catalog = s.spawn(|| garage::catalog(dir, &physics));
            let heli_sheet = s.spawn(|| heli_sheet(dir, &index.stream_file, road_network.is_some()));
            let map_packs =
                crate::modding::content::scenery::packs(dir, crate::modding::content::mod_dirs(), &installed);
            let interiors =
                crate::modding::content::interior::load(crate::modding::content::mod_dirs(), &map_packs, first_pack);
            (joined(garage_catalog), map_packs, interiors, joined(heli_sheet))
        });
        let garage_catalog = garage_catalog?;
        let (map_blips, race_starts, entry_zones, gameplay) = match gameplay {
            Ok(data) => (
                super::hud::interior_blips(&data),
                super::hud::race_start_blips(&data),
                entry_zones::EntryZones::from_data(&data),
                Some(data),
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
                // The police car a NIS scene spawns loads now, so a bust or a race intro usually finds it (`nis_poses.rs`).
                world.request_nis_cop();
                world.set_roadblock_physics(&props);
                world.set_track_barriers(std::mem::take(&mut index.track_barriers));
                if let Some(sheet) = heli_sheet {
                    world.set_heli_sheet(sheet);
                }
            }
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
                    let mut residency = Residency::adopt(
                        index.sections,
                        index.visible,
                        started,
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
                reflection_scratch: Default::default(),
                reflection_last: None,
                clock: 0.0,
                environment_lights: super::environment_lights::EnvironmentLights::new(physics.database()),
                sun: sun::Sun::new(),
                day: super::day::DayClock::new(nfsmw_data::day_night::DayNightConfig::default()),
                physics_aspect: 16.0 / 9.0,
                physics_clock: drive::FixedClock::default(),
                speedbreaker_clock: drive::SpeedbreakerClock::default(),
                view: if pending_car.is_some() { View::Chase } else { View::Fly },
                last_car: pending_car.as_ref().map_or_else(|| DEFAULT_CAR.to_owned(), |c| c.name.clone()),
                speech_look: Default::default(),
                pending_car,
                players: players::Players::default(),
                car_keys: Default::default(),
                rules: Default::default(),
                traffic,
                map_blips,
                race_starts,
                gameplay,
                race: None,
                queued_race: None,
                queued_again: None,
                queued_split: None,
                split: None,
                second_view: split::SecondView::new(),
                inputs: Default::default(),
                coop: None,
                queued_coop: None,
                end_coop_queued: false,
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
                    pursuit: Default::default(),
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
                nis: Default::default(),
                story_movie: Default::default(),
                photo_shot: None,
                breaker_icons: Default::default(),
                #[cfg(feature = "multiplayer")]
                net_client: None,
            };
            scene.switch_default_scenery_groups();
            scene
        })))
    }
}

/// Reads the streaming index and starts the streaming on it, which is the first thing the world needs and the one
/// the rest waits on the least: the stream's loader threads start on the shared sets and the tiles around `at` (the
/// middle of the map when the caller names no start) while the other loads run.
fn start_streaming(dir: &GameDir, at: Option<[f32; 2]>) -> Result<(WorldIndex, super::residency::start::Started)> {
    let index = WorldIndex::open(dir, DEFAULT_TRACK)?;
    let stream =
        dir.resolve(&index.stream_file).with_context(|| format!("{} is missing", index.stream_file))?.to_path_buf();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
    let at = at.unwrap_or_else(|| index.centre());
    let started = super::residency::start::begin(stream, &index.sections, &index.visible, at, threads);
    Ok((index, started))
}

/// The authored cloud map of the track, or none (with a warning) when it cannot be read. The track file is the one the
/// streaming index came from, so this reads the copy that is already in memory.
fn atmosphere(dir: &GameDir) -> lighting::Clouds {
    nfsmw_data::shared_unwrapped(dir, &format!("TRACKS/{DEFAULT_TRACK}.BUN"))
        .and_then(|bytes| lighting::Clouds::read(&bytes))
        .map_err(|error| log::warn!("authored cloud map unavailable: {error:#}"))
        .unwrap_or_default()
}

/// Has the stream's loader threads place each section's scenery as far as that needs no renderer
/// (`resident/prepare.rs`).
fn prepare_on_loaders(streamer: &Streamer, catalog: PropCatalog, groups: std::collections::HashMap<u16, u16>) {
    streamer.set_prepare(super::resident::worker_hook(catalog, groups));
}

/// The helicopter's height sheet, when the track has a road network to fly over: `None` when there is no network,
/// else the sheet or `None` (with a warning) when it cannot be read.
fn heli_sheet(dir: &GameDir, stream_file: &str, has_roads: bool) -> Option<blackbox_roads::HeliSheet> {
    if !has_roads {
        return None;
    }
    nfsmw_data::world::heli_sheet::read(dir, stream_file)
        .inspect(|sheet| log::info!("helicopter sheet: {} triangles", sheet.len()))
        .map_err(|e| log::warn!("helicopter sheet: {e}; the helicopter flies at a flat floor"))
        .ok()
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
