//! The real [`Link`]: the local Discord client's IPC pipe, through the `discord-presence` crate. The crate runs the
//! pipe on a thread of its own and calls back for Discord's events; the callbacks only pass what they get on: a friend's
//! join secret to the game, a friend's join request to the worker.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use discord_presence::event_handler::EventCallbackHandle;
use discord_presence::models::{Activity, EventData};
use discord_presence::{Client, Event};

use super::presence::Presence;
use super::worker::{Command, Link};

/// How long a connection may take to say it is up before the worker tries again later.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// How often the connection's state is looked at while connecting.
const POLL: Duration = Duration::from_millis(25);

/// What the crate's callbacks tell the link about the connection.
#[derive(Default)]
struct State {
    connected: AtomicBool,
    /// The first connection attempt failed (no Discord client).
    failed: AtomicBool,
    /// The connection is up and has not dropped.
    alive: AtomicBool,
}

/// A connection to the Discord client of this machine, for the application `client_id`.
pub struct IpcLink {
    client_id: u64,
    /// Where a friend's join request goes: the worker's own command queue.
    requests: Sender<Command>,
    /// Where a friend's join secret goes: the game.
    joins: Sender<String>,
    client: Option<Client>,
    state: Arc<State>,
    handlers: Vec<EventCallbackHandle>,
}

impl IpcLink {
    pub fn new(client_id: u64, requests: Sender<Command>, joins: Sender<String>) -> Self {
        Self { client_id, requests, joins, client: None, state: Arc::default(), handlers: Vec::new() }
    }

    fn listen(&mut self, client: &Client) {
        let state = Arc::clone(&self.state);
        self.handlers.push(client.on_connected(move |_| {
            state.alive.store(true, Ordering::Release);
            state.connected.store(true, Ordering::Release);
        }));
        let state = Arc::clone(&self.state);
        self.handlers.push(client.on_error(move |_| {
            if !state.connected.load(Ordering::Acquire) {
                state.failed.store(true, Ordering::Release);
            }
        }));
        let state = Arc::clone(&self.state);
        self.handlers.push(client.on_disconnected(move |_| state.alive.store(false, Ordering::Release)));
        let joins = self.joins.clone();
        self.handlers.push(client.on_activity_join(move |context| {
            let EventData::ActivityJoin(joined) = context.event else { return };
            let Some(secret) = joined.secret else { return };
            let _ = joins.send(secret);
        }));
        let requests = self.requests.clone();
        self.handlers.push(client.on_activity_join_request(move |context| {
            let EventData::ActivityJoinRequest(request) = context.event else { return };
            let Some(user) = request.user.and_then(|user| user.id).and_then(|id| id.parse().ok()) else { return };
            let _ = requests.send(Command::JoinRequest(user));
        }));
    }

    /// The client, while its connection is up.
    fn alive(&mut self) -> Result<&mut Client, String> {
        let up = self.state.alive.load(Ordering::Acquire);
        match (&mut self.client, up) {
            (Some(client), true) => Ok(client),
            _ => Err("the connection closed".into()),
        }
    }
}

impl Link for IpcLink {
    fn connect(&mut self) -> Result<(), String> {
        // One attempt: the worker paces the retries, so the crate's own thread gives up after the first failure.
        let mut client = Client::with_error_config(self.client_id, Duration::from_secs(1), Some(0));
        self.state = Arc::default();
        self.listen(&client);
        client.start();
        let started = Instant::now();
        while !self.state.connected.load(Ordering::Acquire) {
            let failed = self.state.failed.load(Ordering::Acquire);
            if failed || started.elapsed() >= CONNECT_TIMEOUT {
                self.handlers.clear();
                let _ = client.shutdown();
                return Err(if failed { "no Discord client is running" } else { "Discord did not answer" }.into());
            }
            thread::sleep(POLL);
        }
        for event in [Event::ActivityJoin, Event::ActivityJoinRequest] {
            if let Err(e) = client.subscribe(event, |args| args) {
                log::debug!("discord: not subscribed to {event:?} ({e})");
            }
        }
        self.client = Some(client);
        Ok(())
    }

    fn set_activity(&mut self, presence: &Presence) -> Result<(), String> {
        // Queued, not sent at once: the crate sends it from its own thread within Discord's rate limit, and a
        // connection that drops meanwhile shows in `alive`, not as a call that never returns.
        let client = self.alive()?;
        client.queue_activity(|activity| activity_of(activity, presence));
        Ok(())
    }

    fn answer_join_request(&mut self, user: u64, accept: bool) -> Result<(), String> {
        let client = self.alive()?;
        let answered = match accept {
            true => client.send_activity_join_invite(user),
            false => client.close_activity_request(user),
        };
        answered.map(drop).map_err(|e| e.to_string())
    }

    fn close(&mut self) {
        self.handlers.clear();
        self.state.alive.store(false, Ordering::Release);
        let Some(client) = self.client.take() else { return };
        if let Err(e) = client.shutdown() {
            log::debug!("discord: the connection was not closed cleanly ({e})");
        }
    }
}

/// The wire form of a presence: the status line, the start time and the party when there are any.
fn activity_of(activity: Activity, presence: &Presence) -> Activity {
    let mut activity = activity.details(presence.details.as_str());
    if let Some(started) = presence.started_at_ms.and_then(|ms| u64::try_from(ms).ok()) {
        activity = activity.timestamps(|times| times.start(started));
    }
    let Some(party) = &presence.party else {
        return activity;
    };
    activity = activity.party(|wire| wire.id(party.id.as_str()).size(party.size));
    let Some(join) = &party.join else {
        return activity;
    };
    activity.secrets(|secrets| secrets.join(join.as_str()))
}
