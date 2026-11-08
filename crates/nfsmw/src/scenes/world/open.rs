//! Opening the world scene: the start of a track, its streaming, cars and traffic from the install.

use super::*;
use crate::modding::content::worlds::World;

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
        Self::load_in(dir, options, None)
    }

    /// [`WorldScene::open`] in a second world a mod provides (`docs/specs/second-world.md`), or in the city for `None`.
    pub fn open_in(dir: &GameDir, options: Options, world: Option<World>) -> Result<Self> {
        Ok(Self::load_in(dir, options, world)?.build())
    }

    /// [`WorldScene::load`] in a second world, or in the city for `None`. A second world keeps the shared sets of the city
    /// (the sky, the common models) and replaces everything that is the city's: its tiles, collision, roads, traffic,
    /// races, weather maps and sounds by place. It starts at the world's start unless `options.at` says otherwise.
    pub fn load_in(dir: &GameDir, options: Options, world: Option<World>) -> Result<WorldLoad> {
        let drive = options.drive;
        let default_start = world.as_ref().map(|w| [w.manifest.start[0], w.manifest.start[1]]);
        let at = options.at.or(default_start);
        let heading = match (options.at, world.as_ref()) {
            (None, Some(world)) => world.manifest.heading,
            _ => options.heading,
        };
        let world_id = world.as_ref().map(|w| w.id().to_owned());
        // These loads need nothing another one produces, so they run at once. Every thread is joined before
        // any result is used, and errors are reported in the order the steps used to run. The index thread goes on
        // to start the streaming: the shared sets and the tiles around the start load while everything else is read.
        let (index, audio_environment, physics, globals, car, gameplay, atmosphere) = std::thread::scope(|s| {
            let index = s.spawn(|| start_streaming(dir, at, world_id.as_deref()));
            let audio = s.spawn(|| audio_environment(dir, world_id.is_some()));
            let physics = s.spawn(|| PhysicsData::load(dir));
            let globals = s.spawn(|| load_global_textures(dir));
            let car = s.spawn(move || drive.map(|d| load_pending_car(dir, d)).transpose());
            let gameplay = s.spawn(|| world_id.is_none().then(|| nfsmw_data::gameplay::cached(dir)));
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
        // The weather maps and the mesh animations are placed in the city; a second world has none.
        let track = match world_id {
            Some(_) => None,
            None => nfsmw_data::read_unwrapped(dir, &format!("TRACKS/{DEFAULT_TRACK}.BUN"))
                .map_err(|error| log::warn!("track parameter maps unavailable: {error:#}"))
                .ok(),
        };
        let mesh_animations = mesh_animation::MeshAnimations::load(dir, track.as_deref());
        let rain = track
            .as_deref()
            .map(crate::weather::rain::Rain::read)
            .and_then(|read| read.map_err(|error| log::warn!("authored rain map unavailable: {error:#}")).ok())
            .unwrap_or_default();
        let wind = track
            .as_deref()
            .map(crate::weather::wind::Wind::read)
            .and_then(|read| read.map_err(|error| log::warn!("authored wind map unavailable: {error:#}")).ok())
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
        let pursuit_look =
            pursuit_look::PursuitLook::new(nfsmw_data::visual_treatment::pursuit::load(physics.database()));
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
            let world = world.as_ref();
            let garage_catalog = s.spawn(|| garage::catalog(dir, &physics));
            let heli_sheet = s.spawn(|| heli_sheet(dir, &index.stream_file, road_network.is_some()));
            let map_packs = match world {
                Some(world) => crate::modding::content::worlds::packs(dir, world, &installed),
                None => crate::modding::content::scenery::packs(dir, crate::modding::content::mod_dirs(), &installed),
            };
            let interiors = match world {
                Some(_) => Default::default(),
                None => {
                    crate::modding::content::interior::load(crate::modding::content::mod_dirs(), &map_packs, first_pack)
                }
            };
            (joined(garage_catalog), map_packs, interiors, joined(heli_sheet))
        });
        let garage_catalog = garage_catalog?;
        let (map_blips, race_starts, entry_zones, gameplay) = match gameplay {
            // A second world has no races, shops or interiors of the city.
            None => (Vec::new(), Vec::new(), entry_zones::EntryZones::default(), None),
            Some(Ok(data)) => (
                super::hud::interior_blips(&data),
                super::hud::race_start_blips(&data),
                entry_zones::EntryZones::from_data(&data),
                Some(data),
            ),
            Some(Err(e)) => {
                log::warn!("race and interior blips, entry zones and races are off: {e:#}");
                (Vec::new(), Vec::new(), entry_zones::EntryZones::default(), None)
            }
        };
        let entry_zones = entry_zones.with_interiors(interiors);
        // A second world has no collision grid in a file: its packs list themselves in an empty one as they arrive.
        let world_grid = world.as_ref().map(|_| world_collision_grid(&map_packs));
        let second_world = world.is_some();
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
            // The Custom map's roads: simplified once per network, then shared with the HUD every frame.
            let map_roads = traffic.as_ref().map(|world| {
                let roads = crate::hud::MapRoads::build(world.road_graph().0, |p| {
                    let at = space::to_render(p.to_array());
                    [at.x, at.y]
                });
                crate::hud::MapRoadsHandle::new(roads)
            });
            let start = at.unwrap_or_else(|| index.centre());
            let camera = FlyCamera {
                position: Vec3::new(start[0], start[1], options.height),
                yaw: heading.to_radians(),
                pitch: options.pitch.to_radians(),
                speed: 40.0,
            };
            log::info!("camera starts at ({:.0}, {:.0}), {:.0} m above the ground", start[0], start[1], options.height);
            let mut scene = Self {
                detail: Default::default(),
                garage: garage_catalog,
                garage_parts: None,
                garage_open: false,
                overlay_open: false,
                garage_backdrop: None,
                garage_exit: None,
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
                        world_grid.or(index.collision_grid),
                        props,
                        map_packs,
                    );
                    residency.set_dynamic_grid(second_world);
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
                pursuit_look,
                reflection_faces: std::array::from_fn(|_| Vec::new()),
                reflection_scratch: Default::default(),
                reflection_last: None,
                clock: 0.0,
                environment_lights: super::environment_lights::EnvironmentLights::new(physics.database()),
                street_glows: Vec::new(),
                fixture_emission: Default::default(),
                local_scratch: Default::default(),
                sun: sun::Sun::new(),
                day: super::day::DayClock::new(nfsmw_data::day_night::DayNightConfig::default()),
                night_headlights_restore: None,
                cycle_lights_night: None,
                physics_aspect: 16.0 / 9.0,
                drawn_aspect: 16.0 / 9.0,
                hud_feed: Default::default(),
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
                map_roads,
                map_blips,
                race_starts,
                gameplay,
                race: None,
                screen_fade: Default::default(),
                queued_race: None,
                queued_again: None,
                queued_split: None,
                race_people: Vec::new(),
                race_hold: false,
                race_lead_idle: false,
                split: None,
                second_view: split::SecondView::new(),
                inputs: Default::default(),
                coop: None,
                queued_coop: None,
                end_coop_queued: false,
                race_mods: Default::default(),
                mod_blips: Vec::new(),
                nitrous_fx: None,
                grading: Default::default(),
                wind_fx: None,
                nitrous_screen: Default::default(),
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
                tire_smoke_variant: None,
                rain,
                wind,
                wet_road: Default::default(),
                explosions: Default::default(),
                heli_wash: Default::default(),
                finish_flares: Default::default(),
                ambient_fx: Default::default(),
                mesh_animations,
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
                extra_view: Default::default(),
                pursuit_trace: Default::default(),
                extra: crate::extra_options::storage::load().unwrap_or_else(|e| {
                    log::warn!("Extra options: {e:#}");
                    crate::extra_options::State {
                        persistence_error: Some(format!("{e:#}")),
                        message: Some(format!(
                            "Config could not be loaded: {e:#}. Fix the file and reload before saving preferences."
                        )),
                        ..Default::default()
                    }
                }),
                cop_health: Default::default(),
                moment: Default::default(),
                moment_shots: Default::default(),
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
fn start_streaming(
    dir: &GameDir,
    at: Option<[f32; 2]>,
    world: Option<&str>,
) -> Result<(WorldIndex, super::residency::start::Started)> {
    let index = WorldIndex::open(dir, DEFAULT_TRACK)?;
    let index = match world {
        Some(id) => index.without_city(id),
        None => index,
    };
    let stream =
        dir.resolve(&index.stream_file).with_context(|| format!("{} is missing", index.stream_file))?.to_path_buf();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
    let at = at.unwrap_or_else(|| index.centre());
    let started = super::residency::start::begin(stream, &index.sections, &index.visible, at, threads);
    Ok((index, started))
}

/// The empty collision grid of a second world, laid over the anchors of its map packs (physics space: x is minus render y,
/// z is render x) with a margin, in cells of 64 m or larger when the world is so wide that 64 m would take more than 60,000.
fn world_collision_grid(packs: &[crate::modding::content::scenery::MapPack]) -> blackbox_collision::Grid {
    const MARGIN: f32 = 200.0;
    const CELLS: f32 = 60_000.0;
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for pack in packs {
        let [x, y] = pack.section.center;
        let (px, pz, r) = (-y, x, pack.section.radius);
        low = [low[0].min(px - r), low[1].min(pz - r)];
        high = [high[0].max(px + r), high[1].max(pz + r)];
    }
    if low[0] > high[0] {
        return blackbox_collision::Grid::empty([0.0, 0.0], 64.0, 1, 1);
    }
    let (min, size) =
        ([low[0] - MARGIN, low[1] - MARGIN], [high[0] - low[0] + 2.0 * MARGIN, high[1] - low[1] + 2.0 * MARGIN]);
    let edge = 64.0_f32.max((size[0] * size[1] / CELLS).sqrt().ceil());
    let (cols, rows) = ((size[0] / edge).ceil() as u32 + 1, (size[1] / edge).ceil() as u32 + 1);
    blackbox_collision::Grid::empty(min, edge, rows, cols)
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
fn audio_environment(dir: &GameDir, second_world: bool) -> Option<crate::audio::environment_context::EnvironmentMap> {
    if second_world {
        return None;
    }
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
