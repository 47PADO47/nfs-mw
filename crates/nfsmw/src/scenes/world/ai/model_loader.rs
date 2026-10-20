//! Car models read on a worker thread, so the frame that asks for a car does not wait for its files.
//!
//! The worker finds the car folders, reads and assembles the cars and measures a cop type's performance once per
//! session. It does not touch the physics data: that holds the track's event sequences, and only the main thread has
//! them. So the main thread makes the physics from the cars it gets back, and the rigs from the physics (`models.rs`).
//! A cop car takes two trips to the worker: first its car, then its performance and light textures, which need its
//! physics.

use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use blackbox_tpk::Texture;
use blackbox_vehicle::performance::{Performance, measure_with_integration};
use game_install::GameDir;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::cop_lights::CopLightFx;
use nfsmw_data::car::physics::CarPhysics;
use nfsmw_data::pursuit::AiVehicle;

use super::models::folders;
use crate::audio::traffic::TrafficProfile;
use crate::scenes::world::load_car;

/// What the scene asks the loader for. Names are `pvehicle` names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Request {
    /// A traffic car, and the trailer it pulls when it is a semi tractor.
    Traffic(String),
    /// A patrol cop car.
    Cop(String),
    /// The police helicopter's rig.
    Helicopter,
    /// The models of a roadblock's spike strip and sawhorse.
    RoadblockProps,
}

impl Request {
    /// The request as the pending set keeps it: car names are not case sensitive.
    fn key(&self) -> Self {
        match self {
            Self::Traffic(name) => Self::Traffic(name.to_ascii_lowercase()),
            Self::Cop(name) => Self::Cop(name.to_ascii_lowercase()),
            Self::Helicopter => Self::Helicopter,
            Self::RoadblockProps => Self::RoadblockProps,
        }
    }
}

/// A `pvehicle` whose car folder the worker looks for: its name and the names of its ancestors, which the folders may
/// come from (`PhysicsData::lineage`, read on the main thread).
pub struct Lookup {
    pub vehicle: String,
    pub lineage: Vec<String>,
}

/// A car as the main thread makes its physics from it: everything but the rig.
pub struct CarParts {
    /// The name the model is known by: the upper-case `pvehicle` for traffic, the folder for a cop.
    pub name: String,
    pub model: CarModel,
    pub physics: CarPhysics,
    /// Its installed rigid-body behaviour is a tractor.
    pub tractor: bool,
    pub audio_profile: TrafficProfile,
}

/// A cop car between its two trips: its car is loaded, its performance and textures are on their way.
pub struct StagedCop {
    pub car: CarParts,
    pub ai: AiVehicle,
    pub light_fx: Option<CopLightFx>,
}

/// The work the worker does for one request.
pub enum Job {
    Traffic {
        vehicle: Lookup,
        trailer: Option<Lookup>,
    },
    Cop(Lookup),
    /// A cop car's second trip: its performance (measured once per `car_type`) and the textures of its lights.
    CopExtras {
        car_type: String,
        hashes: Vec<u32>,
        staged: Box<StagedCop>,
    },
    Helicopter,
    RoadblockProps,
}

/// The models of a roadblock's props, from the track's shared scenery.
pub struct RoadblockModels {
    pub spike: CarModel,
    pub sawhorse: CarModel,
}

/// A traffic car as the worker loaded it: its model and, for a semi tractor, its trailer's `pvehicle` and model.
pub struct TrafficLoad {
    pub body: CarModel,
    pub trailer: Option<(String, CarModel)>,
}

/// A cop car's model and folder, from its first trip.
pub struct CopLoad {
    pub folder: String,
    pub model: CarModel,
}

/// A cop car after its second trip, with the staged data it came with.
pub struct CopExtrasLoad {
    pub staged: StagedCop,
    pub performance: Performance,
    pub textures: Vec<Texture>,
}

/// What the worker made of a job. The payloads are boxed so the variants stay the same size.
pub enum Loaded {
    Traffic(Box<TrafficLoad>),
    Cop(Box<CopLoad>),
    CopExtras(Box<CopExtrasLoad>),
    Helicopter(Box<CarModel>),
    RoadblockProps(Box<RoadblockModels>),
}

/// A request the worker has finished, or failed.
pub struct Done {
    pub request: Request,
    pub result: Result<Loaded, String>,
}

/// The scene's end of the loader: requests go to the worker thread, finished ones come back. When the loader is dropped
/// the worker stops after its current job. It is not joined, so a long measurement does not hold up the scene's drop.
pub struct ModelLoader {
    jobs: Option<Sender<(Request, Job)>>,
    results: Receiver<Done>,
    /// The requests submitted and not yet taken with `next`.
    pending: HashSet<Request>,
    /// The finished requests not yet taken.
    ready: VecDeque<Done>,
}

impl ModelLoader {
    /// Starts the worker, which reads the install at `dir`.
    pub fn new(dir: GameDir) -> Self {
        let (jobs, job_rx) = channel();
        let (done_tx, results) = channel();
        let worker = Worker { dir, performance: HashMap::new() };
        let spawned =
            std::thread::Builder::new().name("model-loader".to_owned()).spawn(move || serve(worker, job_rx, done_tx));
        let jobs = match spawned {
            // The handle is dropped, which detaches the thread.
            Ok(_detached) => Some(jobs),
            Err(e) => {
                log::error!("cannot start the model loader: {e}");
                None
            }
        };
        Self { jobs, results, pending: HashSet::new(), ready: VecDeque::new() }
    }

    /// Asks the worker for `request`, unless it is pending already.
    pub fn submit(&mut self, request: Request, job: Job) {
        let key = request.key();
        if self.pending.contains(&key) {
            return;
        }
        self.pending.insert(key);
        let sent = self.jobs.as_ref().is_some_and(|jobs| jobs.send((request.clone(), job)).is_ok());
        if !sent {
            self.ready.push_back(Done { request, result: Err("the model loader is not running".to_owned()) });
        }
    }

    /// The next finished request. Without `block` this does not wait; with it, this waits while requests are pending.
    pub fn next(&mut self, block: bool) -> Option<Done> {
        self.receive();
        if block && self.ready.is_empty() && !self.pending.is_empty() {
            self.wait();
        }
        let done = self.ready.pop_front()?;
        self.pending.remove(&done.request.key());
        Some(done)
    }

    /// Moves what the worker has finished into `ready`, without waiting.
    fn receive(&mut self) {
        loop {
            match self.results.try_recv() {
                Ok(done) => self.ready.push_back(done),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.lost();
                    return;
                }
            }
        }
    }

    /// Waits for the worker to finish one request.
    fn wait(&mut self) {
        match self.results.recv() {
            Ok(done) => self.ready.push_back(done),
            Err(_) => self.lost(),
        }
    }

    /// The worker is gone: the pending requests it did not finish fail.
    fn lost(&mut self) {
        let finished: HashSet<Request> = self.ready.iter().map(|done| done.request.key()).collect();
        let lost: Vec<Request> = self.pending.iter().filter(|key| !finished.contains(*key)).cloned().collect();
        for request in lost {
            self.ready.push_back(Done { request, result: Err("the model loader stopped".to_owned()) });
        }
    }
}

/// The worker thread's state: the install it reads, and the performance of each cop car type it has measured.
struct Worker {
    dir: GameDir,
    performance: HashMap<String, Performance>,
}

impl Worker {
    fn run(&mut self, job: Job) -> Result<Loaded, String> {
        match job {
            Job::Traffic { vehicle, trailer } => self.traffic(&vehicle, trailer.as_ref()),
            Job::Cop(lookup) => self.cop(&lookup),
            Job::CopExtras { car_type, hashes, staged } => Ok(self.extras(&car_type, &hashes, *staged)),
            Job::Helicopter => self.helicopter(),
            Job::RoadblockProps => self.roadblock_props(),
        }
    }

    /// The car folder of `lookup` (the first that loads), with its folder name.
    fn car(&self, lookup: &Lookup) -> Result<(String, CarModel), String> {
        let cars = nfsmw_data::car::list(&self.dir);
        let mut last = format!("no car folder for {}", lookup.vehicle);
        for folder in folders(&cars, &lookup.vehicle, &lookup.lineage) {
            match load_car(&self.dir, &folder) {
                Ok(car) => return Ok(car),
                Err(e) => last = format!("{e:#}"),
            }
        }
        Err(last)
    }

    fn traffic(&self, vehicle: &Lookup, trailer: Option<&Lookup>) -> Result<Loaded, String> {
        let (_, body) = self.car(vehicle)?;
        let trailer = trailer
            .map(|lookup| {
                self.car(lookup)
                    .map(|(_, model)| (lookup.vehicle.clone(), model))
                    .map_err(|e| format!("its trailer {}: {e}", lookup.vehicle))
            })
            .transpose()?;
        Ok(Loaded::Traffic(Box::new(TrafficLoad { body, trailer })))
    }

    fn cop(&self, lookup: &Lookup) -> Result<Loaded, String> {
        let (folder, model) = self.car(lookup)?;
        Ok(Loaded::Cop(Box::new(CopLoad { folder, model })))
    }

    /// The cop's performance (measured the first time its type is asked for) and its light textures.
    fn extras(&mut self, car_type: &str, hashes: &[u32], staged: StagedCop) -> Loaded {
        let performance = *self
            .performance
            .entry(car_type.to_owned())
            .or_insert_with(|| measure_with_integration(&staged.car.physics.spec, staged.car.physics.tire_integration));
        let textures = self.textures(hashes);
        Loaded::CopExtras(Box::new(CopExtrasLoad { staged, performance, textures }))
    }

    fn textures(&self, hashes: &[u32]) -> Vec<Texture> {
        if hashes.is_empty() {
            return Vec::new();
        }
        nfsmw_data::car::exhaust::particle_textures(&self.dir, hashes).unwrap_or_default()
    }

    /// The sawhorse and the spike strip are scenery models of the track's shared sections (`XO_SAWHORSE_1B_00`,
    /// `XO_SPIKEBELT_1B_DW_00`); the rest of the shared set is dropped.
    fn roadblock_props(&self) -> Result<Loaded, String> {
        use crate::scenes::world::ai::roadblock::props::{SAWHORSE_MODEL, SPIKE_MODEL};
        let shared = nfsmw_data::world::SharedAssets::load(&self.dir, nfsmw_data::world::DEFAULT_TRACK)
            .map_err(|e| format!("the shared scenery: {e:#}"))?;
        let model = |name: &str| {
            shared
                .find(name)
                .map(|index| shared.model(index))
                .ok_or_else(|| format!("{name} is not in the shared scenery"))
        };
        Ok(Loaded::RoadblockProps(Box::new(RoadblockModels {
            spike: model(SPIKE_MODEL)?,
            sawhorse: model(SAWHORSE_MODEL)?,
        })))
    }

    /// The helicopter is not a car: only its model is read (`docs/specs/ai-helicopter.md`).
    fn helicopter(&self) -> Result<Loaded, String> {
        let (_, model) = load_car(&self.dir, "COPHELI").map_err(|e| format!("copheli: {e:#}"))?;
        Ok(Loaded::Helicopter(Box::new(model)))
    }
}

/// Serves the jobs until the loader is dropped (its job sender closes) or nobody reads the results any more.
fn serve(mut worker: Worker, jobs: Receiver<(Request, Job)>, done: Sender<Done>) {
    while let Ok((request, job)) = jobs.recv() {
        // A panic in the readers fails that request only; the worker goes on with the next one.
        let result = match catch_unwind(AssertUnwindSafe(|| worker.run(job))) {
            Ok(result) => result,
            Err(_) => Err("the model loader failed on it".to_owned()),
        };
        if done.send(Done { request, result }).is_err() {
            return;
        }
    }
}
