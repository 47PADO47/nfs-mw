//! The thread that owns the connection to Discord. It connects when there is something to show, retries at a
//! fixed pace while Discord is not running, and keeps the shown activity in step with the commands it receives.
//!
//! The connection is generic over [`Link`] so the state machine can be tested without a Discord client. A friend's
//! request to join arrives as a [`Command`] too: the worker answers it from what the presence shows.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::presence::Presence;

/// How long to wait before trying Discord again after it was not there (or dropped the connection).
pub const RETRY_EVERY: Duration = Duration::from_secs(15);

/// What the game asks of the presence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Show this activity, connecting to Discord if needed.
    Show(Presence),
    /// Clear the activity and let go of the connection.
    Hide,
    /// The Discord user with this id asks to join the game the presence shows (sent by the [`Link`], not the game).
    JoinRequest(u64),
    /// The game is closing: hide, and stop the thread.
    Quit,
}

/// One connection to a Discord client. Every method reports failure as text, for the debug log only.
pub trait Link {
    fn connect(&mut self) -> Result<(), String>;
    fn set_activity(&mut self, presence: &Presence) -> Result<(), String>;
    /// Accept (invite) or decline the join request of Discord user `user`.
    fn answer_join_request(&mut self, user: u64, accept: bool) -> Result<(), String>;
    /// Let go of the connection. Discord ends the activity of a connection that closes.
    fn close(&mut self);
}

/// The worker's state: what is wanted, what the open link shows, and when to try connecting again.
pub struct Worker<L: Link, F: FnMut() -> L> {
    open: F,
    link: Option<L>,
    /// The activity the connected Discord shows, if it was set since the last connect.
    shown: Option<Presence>,
    /// The activity the game wants, `None` for nothing.
    wanted: Option<Presence>,
    /// The users whose join requests wait for an answer.
    requests: Vec<u64>,
    next_attempt: Instant,
    quit: bool,
}

impl<L: Link, F: FnMut() -> L> Worker<L, F> {
    /// A worker that makes its links with `open`. It starts disconnected with nothing wanted.
    pub fn new(open: F) -> Self {
        Self {
            open,
            link: None,
            shown: None,
            wanted: None,
            requests: Vec::new(),
            next_attempt: Instant::now(),
            quit: false,
        }
    }

    /// Serve commands until the game quits or drops its sender, then close the connection.
    pub fn run(mut self, commands: Receiver<Command>) {
        loop {
            let received = match self.next_wait(Instant::now()) {
                Some(wait) => commands.recv_timeout(wait),
                None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match received {
                Ok(command) => self.apply(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.hide();
                    return;
                }
            }
            self.sync(Instant::now());
            if self.quit {
                self.hide();
                return;
            }
        }
    }

    /// Record what the game now wants. The change reaches Discord at the next [`Worker::sync`].
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Show(presence) => self.wanted = Some(presence),
            Command::Hide => self.wanted = None,
            Command::JoinRequest(user) => self.requests.push(user),
            Command::Quit => {
                self.wanted = None;
                self.quit = true;
            }
        }
    }

    /// How long the worker may sleep before it has to try connecting again. `None` waits for the next command.
    pub fn next_wait(&self, now: Instant) -> Option<Duration> {
        if self.wanted.is_none() || self.link.is_some() {
            return None;
        }
        Some(self.next_attempt.saturating_duration_since(now))
    }

    /// Bring Discord in line with what is wanted: connect if due, set the activity if it differs, or hide.
    pub fn sync(&mut self, now: Instant) {
        let Some(wanted) = self.wanted.clone() else {
            self.requests.clear();
            return self.hide();
        };
        if self.link.is_none() && now >= self.next_attempt {
            self.connect(now);
        }
        let Some(link) = self.link.as_mut() else {
            return;
        };
        if self.shown.as_ref() != Some(&wanted) {
            match link.set_activity(&wanted) {
                Ok(()) => self.shown = Some(wanted),
                Err(e) => {
                    log::debug!("discord: the activity was not set ({e}); trying again in {RETRY_EVERY:?}");
                    self.drop_link(now);
                    return;
                }
            }
        }
        self.answer_requests();
    }

    /// Answer the waiting join requests: a friend may join while the presence offers a join secret, else the request
    /// is declined. The game and the party the presence shows are the ones the friend asked about.
    fn answer_requests(&mut self) {
        let open = self.shown.as_ref().and_then(|shown| shown.party.as_ref()).is_some_and(|party| party.join.is_some());
        let Some(link) = self.link.as_mut() else {
            return;
        };
        for user in std::mem::take(&mut self.requests) {
            if let Err(e) = link.answer_join_request(user, open) {
                log::debug!("discord: the join request of {user} was not answered ({e})");
            }
        }
    }

    fn connect(&mut self, now: Instant) {
        let mut link = (self.open)();
        if let Err(e) = link.connect() {
            log::debug!("discord: not connected ({e}); trying again in {RETRY_EVERY:?}");
            self.next_attempt = now + RETRY_EVERY;
            return;
        }
        log::debug!("discord: connected");
        self.link = Some(link);
        self.shown = None;
    }

    fn drop_link(&mut self, now: Instant) {
        if let Some(mut link) = self.link.take() {
            link.close();
        }
        self.shown = None;
        self.next_attempt = now + RETRY_EVERY;
    }

    fn hide(&mut self) {
        if let Some(mut link) = self.link.take() {
            link.close();
        }
        self.shown = None;
    }
}
