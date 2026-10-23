use super::*;
use crate::loadout::{CarInfo, CarRestriction};
use crate::message::{CAP_SPECTATE_ONLY, ServerName};
use crate::version::{DataId, PROTO_MAJOR, ProtoVersion, SimBuild};

struct Cars;

impl Catalog for Cars {
    fn car(&self, car_type: u32) -> Option<CarInfo> {
        (car_type < 10).then_some(CarInfo { class: 1 })
    }
    fn part_fits(&self, _: u32, _: u16, _: u32) -> bool {
        true
    }
    fn paint_known(&self, _: u32) -> bool {
        true
    }
    fn wheel_set_known(&self, _: u32) -> bool {
        true
    }
    fn performance(&self, _: &Loadout) -> Option<u16> {
        Some(100)
    }
}

fn build() -> BuildInfo {
    BuildInfo {
        proto: ProtoVersion { major: PROTO_MAJOR, minor: 0 },
        sim_build: SimBuild::from_parts("abc", "x86_64-pc-windows-msvc"),
        data_id: DataId::compute(&[1; 32], &[]),
    }
}

struct World {
    build: BuildInfo,
    settings: Settings,
    phase: SessionPhase,
    racers: usize,
    spectators: usize,
    gate: JoinGate,
    bans: Bans,
    book: ResumeBook,
}

impl World {
    fn new() -> Self {
        Self {
            build: build(),
            settings: Settings::lan(ServerName::new("lan").unwrap()),
            phase: SessionPhase::Lobby,
            racers: 0,
            spectators: 0,
            gate: JoinGate::new(),
            bans: Bans::new(),
            book: ResumeBook::new(),
        }
    }

    fn admit(&mut self, request: &JoinRequest<'_>, now_ms: u64) -> Result<Admission, Refusal> {
        let view = ServerView {
            build: &self.build,
            policy: ServerPolicy::default(),
            settings: &self.settings,
            phase: self.phase,
            racers: self.racers,
            spectators: self.spectators,
            shutting_down: false,
        };
        self.gate.admit(request, &view, &self.bans, &mut self.book, &Cars, now_ms)
    }
}

fn request<'a>(source: u64, player: u8, loadout: &'a Loadout) -> JoinRequest<'a> {
    let info = build();
    JoinRequest {
        source,
        player_id: [player; 16],
        offer: ClientOffer { proto: info.proto, sim_build: info.sim_build, data_id: info.data_id, mods: &[] },
        caps: 0,
        password: PasswordProof::Missing,
        resume: None,
        loadout,
    }
}

fn reason(result: Result<Admission, Refusal>) -> RejectReason {
    result.unwrap_err().reason
}

#[test]
fn a_lobby_join_is_seated_as_a_racer() {
    let mut world = World::new();
    let loadout = Loadout::stock(1);
    let admission = world.admit(&request(1, 1, &loadout), 0).unwrap();
    assert!(matches!(admission, Admission::Seat { placement: Placement::Racer, .. }));
}

#[test]
fn the_policy_decides_joins_in_progress() {
    let settings = Settings::lan(ServerName::new("p").unwrap());
    let closed = Settings { join_in_progress: JoinInProgress::Closed, ..settings.clone() };
    let between = Settings { join_in_progress: JoinInProgress::BetweenRaces, ..settings.clone() };
    let waiting = Ok(Placement::Spectator(SpectateReason::WaitingForRace));
    assert_eq!(placement(&closed, SessionPhase::Lobby, false), Ok(Placement::Racer));
    assert_eq!(placement(&closed, SessionPhase::Running, false), Err(RejectReason::InProgress));
    assert_eq!(placement(&settings, SessionPhase::Running, false), Ok(Placement::Racer), "free roam is open");
    assert_eq!(placement(&settings, SessionPhase::Loading, false), waiting);
    assert_eq!(placement(&between, SessionPhase::Running, false), waiting);
    assert_eq!(placement(&between, SessionPhase::Results, false), waiting);
    let race = Settings { mode: Mode::Circuit, laps: 2, ..settings };
    assert_eq!(placement(&race, SessionPhase::Running, false), waiting, "a race is watched");
    let chosen = Ok(Placement::Spectator(SpectateReason::Chosen));
    assert_eq!(placement(&closed, SessionPhase::Running, true), chosen);
}

#[test]
fn the_pipeline_refuses_in_order() {
    let mut world = World::new();
    let loadout = Loadout::stock(1);
    let mut old = request(1, 1, &loadout);
    old.offer.proto.major = PROTO_MAJOR - 1;
    world.bans.ban([1; 16], 0, 0);
    assert_eq!(reason(world.admit(&old, 0)), RejectReason::ProtocolMismatch, "the version before the ban");
    let mut other_data = request(1, 1, &loadout);
    other_data.offer.data_id = DataId::compute(&[2; 32], &[]);
    assert_eq!(reason(world.admit(&other_data, 0)), RejectReason::DataMismatch);
    assert_eq!(reason(world.admit(&request(1, 1, &loadout), 0)), RejectReason::Banned);
    world.settings.sim_build_match = true;
    let mut other_build = request(2, 2, &loadout);
    other_build.offer.sim_build = SimBuild::from_parts("def", "x86_64-unknown-linux-gnu");
    assert_eq!(reason(world.admit(&other_build, 0)), RejectReason::SimBuildMismatch);
}

#[test]
fn limits_count_racers_and_spectators_apart() {
    let mut world = World::new();
    let loadout = Loadout::stock(1);
    world.racers = 8;
    assert_eq!(reason(world.admit(&request(1, 1, &loadout), 0)), RejectReason::ServerFull);
    let mut watcher = request(2, 2, &loadout);
    watcher.caps = CAP_SPECTATE_ONLY;
    assert!(world.admit(&watcher, 0).is_ok(), "a spectator place is free");
    world.spectators = 4;
    assert_eq!(reason(world.admit(&watcher, 0)), RejectReason::ServerFull);
}

#[test]
fn a_racer_loadout_is_validated_and_a_chosen_spectator_skips_it() {
    let mut world = World::new();
    let unknown = Loadout::stock(99);
    let refusal = world.admit(&request(1, 1, &unknown), 0).unwrap_err();
    assert_eq!((refusal.reason, refusal.detail), (RejectReason::LoadoutRejected, 1));
    world.settings.allowed_cars = CarRestriction::Class(2);
    let refusal = world.admit(&request(2, 2, &Loadout::stock(1)), 0).unwrap_err();
    assert_eq!(refusal.detail, 7);
    let mut watcher = request(3, 3, &unknown);
    watcher.caps = CAP_SPECTATE_ONLY;
    assert!(world.admit(&watcher, 0).is_ok());
}

#[test]
fn a_join_storm_from_one_source_is_rate_limited() {
    let mut world = World::new();
    let loadout = Loadout::stock(1);
    let outcomes: Vec<bool> = (0..8).map(|player| world.admit(&request(42, player, &loadout), 1_000).is_ok()).collect();
    assert_eq!(outcomes, vec![true, true, true, true, true, false, false, false]);
    assert_eq!(reason(world.admit(&request(42, 9, &loadout), 2_999)), RejectReason::RateLimited);
    assert!(world.admit(&request(42, 9, &loadout), 3_000).is_ok(), "one join per 2 s refills");
    assert!(world.admit(&request(43, 9, &loadout), 3_000).is_ok(), "another source is untouched");
}

#[test]
fn a_join_storm_from_many_sources_fills_the_room_and_no_more() {
    let mut world = World::new();
    let loadout = Loadout::stock(1);
    let mut seated = 0;
    for source in 0..100u64 {
        let result = world.admit(&request(source, source as u8, &loadout), 0);
        if result.is_ok() {
            seated += 1;
            world.racers += 1;
            continue;
        }
        assert_eq!(reason(result), RejectReason::ServerFull);
    }
    assert_eq!(seated, 8);
}

#[test]
fn a_wrong_password_looks_like_a_full_room_and_is_limited() {
    let mut world = World::new();
    world.settings.password_required = true;
    let loadout = Loadout::stock(1);
    let mut guess = request(1, 1, &loadout);
    guess.password = PasswordProof::Failed;
    for _ in 0..GUESS_LIMIT {
        assert_eq!(world.admit(&guess, 0).unwrap_err(), Refusal { reason: RejectReason::ServerFull, detail: 0 });
    }
    let mut right = guess;
    right.password = PasswordProof::Passed;
    right.source = 2;
    assert_eq!(reason(world.admit(&right, 1_000)), RejectReason::ServerFull, "the player id is locked");
    assert!(world.admit(&right, LOCK_MS).is_ok(), "the lock ends");
    let mut missing = request(3, 3, &loadout);
    missing.password = PasswordProof::Missing;
    assert_eq!(reason(world.admit(&missing, LOCK_MS)), RejectReason::ServerFull);
}

#[test]
fn the_guess_lock_doubles_up_to_fifteen_minutes() {
    let mut guard = PasswordGuard::new();
    let mut now = 0;
    let mut locks = Vec::new();
    for _ in 0..6 {
        for _ in 0..GUESS_LIMIT {
            guard.failed([1; 16], 7, now);
        }
        let start = now;
        while guard.locked([1; 16], 7, now) {
            now += 1_000;
        }
        locks.push(now - start);
    }
    assert_eq!(locks, vec![60_000, 120_000, 240_000, 480_000, 900_000, 900_000]);
}

#[test]
fn a_resume_token_takes_the_held_slot_once() {
    let mut world = World::new();
    world.phase = SessionPhase::Running;
    world.settings.join_in_progress = JoinInProgress::Closed;
    let token = world.book.issue(5, [5; 16], [8; 32]);
    world.book.hold(5, 0);
    let loadout = Loadout::stock(1);
    let mut thief = request(2, 6, &loadout);
    thief.resume = Some(&token);
    assert_eq!(reason(world.admit(&thief, 500)), RejectReason::InProgress, "another player id");
    let mut back = request(1, 5, &loadout);
    back.resume = Some(&token);
    let admission = world.admit(&back, 1_000).unwrap();
    assert!(matches!(admission, Admission::Resume { resumed: Resumed { slot: 5, was_live: false }, .. }));
    assert_eq!(reason(world.admit(&back, 3_000)), RejectReason::InProgress, "a replay is a normal join");
}
