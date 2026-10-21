//! Two connections joined by simulated wires that lose, duplicate and delay datagrams. Time moves 16 ms per round; each
//! round both sides write a packet and the other side reads the datagrams that are due.

use crate::MAX_DATAGRAM;
use crate::conn::{ConnConfig, ConnError, ConnEvent, Connection};
use crate::rng::SplitMix64;

/// Simulated milliseconds per round.
pub const ROUND_MS: u64 = 16;

/// Datagrams in flight on one wire, each with the round it becomes readable.
type Wire = Vec<(u64, Vec<u8>)>;

/// One end of the pair, with the packet numbers it has sent and the outcome events it has seen.
pub struct Side {
    /// The connection itself.
    pub conn: Connection,
    /// The round each packet number was sent in; the index is the packet number.
    pub sent_round: Vec<u64>,
    /// Delivered or Lost events seen for each packet number.
    pub outcomes: Vec<u32>,
}

impl Side {
    fn new(config: ConnConfig) -> Self {
        Self { conn: Connection::new(config), sent_round: Vec::new(), outcomes: Vec::new() }
    }
}

/// The two ends and the wires between them.
pub struct Harness {
    /// Connection with id 1 talking to id 2.
    pub a: Side,
    /// Connection with id 2 talking to id 1.
    pub b: Side,
    /// Application frames `b` received, in arrival order, with their types.
    pub app_received: Vec<(u8, Vec<u8>)>,
    /// Read errors other than duplicates. Must stay empty.
    pub errors: Vec<ConnError>,
    /// Datagrams refused as duplicates (the wire duplicates some on purpose).
    pub duplicates: u64,
    /// Rounds run so far.
    pub round: u64,
    /// Probability that a datagram is dropped.
    pub loss: f64,
    /// Probability that a datagram is delivered twice.
    pub dup: f64,
    rng: SplitMix64,
    a_to_b: Wire,
    b_to_a: Wire,
    buf: Vec<u8>,
}

impl Harness {
    /// A pair with the given seed, loss and duplication probabilities.
    pub fn new(seed: u64, loss: f64, dup: f64) -> Self {
        Self {
            a: Side::new(ConnConfig { local_id: 1, remote_id: 2, ..ConnConfig::default() }),
            b: Side::new(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() }),
            app_received: Vec::new(),
            errors: Vec::new(),
            duplicates: 0,
            round: 0,
            loss,
            dup,
            rng: SplitMix64(seed),
            a_to_b: Vec::new(),
            b_to_a: Vec::new(),
            buf: vec![0; MAX_DATAGRAM],
        }
    }

    /// The simulated time of the current round.
    pub fn now(&self) -> u64 {
        self.round * ROUND_MS
    }

    /// Runs one round: `a` writes a packet carrying `a_frames`, `b` writes one without frames, each datagram goes onto
    /// its wire (possibly lost or duplicated, delayed 0 to 3 rounds), the due datagrams are read, and events are drained.
    pub fn step(&mut self, a_frames: &[(u8, &[u8])]) {
        self.round += 1;
        let now = self.now();
        let round = self.round;
        let (loss, dup) = (self.loss, self.dup);

        let from_a = emit(&mut self.a, now, round, a_frames, &mut self.buf);
        transmit(&mut self.a_to_b, &mut self.rng, round, loss, dup, from_a);
        let from_b = emit(&mut self.b, now, round, &[], &mut self.buf);
        transmit(&mut self.b_to_a, &mut self.rng, round, loss, dup, from_b);

        let app = &mut self.app_received;
        for datagram in take_due(&mut self.a_to_b, round) {
            let result = self.b.conn.read_packet(now, &datagram, |ftype, payload| app.push((ftype, payload.to_vec())));
            note(result, &mut self.errors, &mut self.duplicates);
        }
        for datagram in take_due(&mut self.b_to_a, round) {
            let result = self.a.conn.read_packet(now, &datagram, |_, _| {});
            note(result, &mut self.errors, &mut self.duplicates);
        }

        drain(&mut self.a);
        drain(&mut self.b);
    }

    /// Runs `rounds` rounds with no loss and no duplication, so that every outstanding packet gets its outcome.
    pub fn settle(&mut self, rounds: u64) {
        self.loss = 0.0;
        self.dup = 0.0;
        for _ in 0..rounds {
            self.step(&[]);
        }
    }
}

/// Writes one packet on `side` and returns the datagram. Records the round and an outcome slot for its number.
fn emit(side: &mut Side, now: u64, round: u64, frames: &[(u8, &[u8])], buf: &mut [u8]) -> Vec<u8> {
    let sent = side.conn.write_packet(now, frames, buf).expect("write_packet");
    side.sent_round.push(round);
    side.outcomes.push(0);
    assert_eq!(sent.number as usize + 1, side.sent_round.len(), "packet numbers run in order");
    buf[..sent.len].to_vec()
}

/// Puts a datagram on a wire unless it is lost; a duplicate is queued separately. Each copy is due 0 to 3 rounds later.
fn transmit(wire: &mut Wire, rng: &mut SplitMix64, round: u64, loss: f64, dup: f64, datagram: Vec<u8>) {
    if rng.next_f64() < loss {
        return;
    }
    let copies = 1 + u64::from(rng.next_f64() < dup);
    for _ in 0..copies {
        wire.push((round + rng.below(4), datagram.clone()));
    }
}

/// Removes and returns the datagrams due by `round`, in the order they were queued.
fn take_due(wire: &mut Wire, round: u64) -> Vec<Vec<u8>> {
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(wire).into_iter().partition(|(at, _)| *at <= round);
    *wire = waiting;
    due.into_iter().map(|(_, datagram)| datagram).collect()
}

/// Counts an outcome of a read: duplicates are expected, anything else is kept for the test to check.
fn note(result: Result<u32, ConnError>, errors: &mut Vec<ConnError>, duplicates: &mut u64) {
    match result {
        Ok(_) => {}
        Err(ConnError::Duplicate) => *duplicates += 1,
        Err(error) => errors.push(error),
    }
}

/// Counts every Delivered or Lost event against its packet number.
fn drain(side: &mut Side) {
    while let Some(event) = side.conn.poll_event() {
        match event {
            ConnEvent::Delivered(number) | ConnEvent::Lost(number) => {
                let count = side.outcomes.get_mut(number as usize).expect("event for a packet never sent");
                *count += 1;
            }
            ConnEvent::Disconnected(_) => {}
        }
    }
}
