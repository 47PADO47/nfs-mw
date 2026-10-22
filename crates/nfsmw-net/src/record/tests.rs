use nfsmw_sim::PlayerId;

use super::*;
use crate::version::{DataId, SimBuild};

fn header(kind: RecordKind) -> ReplayHeader {
    let mut header = ReplayHeader::new(
        kind,
        0x0123_4567_89AB_CDEF,
        SimBuild::from_parts("abc123", "x86_64-pc-windows-msvc"),
        DataId::compute(&[7; 32], &[("GLOBAL/GLOBALB.BUN", 1_000)]),
        600,
    );
    header.player = PlayerId(match kind {
        RecordKind::Server => 0,
        RecordKind::Client => 3,
    });
    header
}

fn snapshot_payload(tick: u32) -> Vec<u8> {
    let mut payload = tick.to_le_bytes().to_vec();
    payload.extend(std::iter::repeat_n(0xA5, (tick % 7) as usize));
    payload
}

/// A finished server recording: a key snapshot every 30 ticks, deltas between, an event every 50 ticks.
fn server_file(ticks: u32) -> Vec<u8> {
    let mut recorder = Recorder::new(&header(RecordKind::Server), DEFAULT_MAX_PENDING);
    let mut file = Vec::new();
    for tick in 600..600 + ticks {
        recorder.snapshot(tick, &snapshot_payload(tick), tick % 30 == 0).expect("recorded");
        if tick % 50 == 0 {
            recorder.event(tick, b"event").expect("recorded");
        }
        if tick % 100 == 0 {
            file.extend(recorder.take());
        }
    }
    file.extend(recorder.finish().expect("finished"));
    file
}

/// The offset of the index frame, from the trailer.
fn index_offset(file: &[u8]) -> usize {
    let at = file.len() - TRAILER_LEN + 8;
    usize::try_from(u64::from_le_bytes(file[at..at + 8].try_into().expect("8 bytes"))).expect("fits")
}

#[test]
fn crc32_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b""), 0);
}

#[test]
fn the_header_round_trips_and_damage_is_detected() {
    for kind in [RecordKind::Server, RecordKind::Client] {
        let mut bytes = Vec::new();
        header(kind).encode(&mut bytes);
        assert_eq!(bytes.len(), HEADER_LEN);
        assert_eq!(ReplayHeader::decode(&bytes), Ok((header(kind), HEADER_LEN)));
    }
    let mut bytes = Vec::new();
    header(RecordKind::Server).encode(&mut bytes);
    assert_eq!(ReplayHeader::decode(&bytes[..HEADER_LEN - 1]), Err(ReplayError::Truncated));
    assert_eq!(Replay::open(b"NFSMW").err(), Some(ReplayError::Truncated));
    let mut flipped = bytes.clone();
    flipped[30] ^= 1;
    assert_eq!(ReplayHeader::decode(&flipped), Err(ReplayError::BadHeader));
    let mut magic = bytes.clone();
    magic[0] = b'X';
    assert_eq!(ReplayHeader::decode(&magic), Err(ReplayError::BadMagic));
    let mut format = bytes;
    format[8] = 0;
    assert_eq!(ReplayHeader::decode(&format), Err(ReplayError::BadFormat(0)));
}

#[test]
fn every_frame_reads_back_in_order() {
    let file = server_file(300);
    let replay = Replay::open(&file).expect("opens");
    assert_eq!(replay.header(), &header(RecordKind::Server));
    assert!(replay.has_index());
    let mut frames = replay.frames();
    let all: Vec<Frame<'_>> = frames.by_ref().collect();
    assert_eq!(frames.damaged(), 0);
    let snapshots: Vec<&Frame<'_>> = all.iter().filter(|frame| frame.kind == FrameKind::Snapshot).collect();
    assert_eq!(snapshots.len(), 300);
    assert!(snapshots.iter().all(|frame| frame.payload == snapshot_payload(frame.tick)));
    assert_eq!(snapshots.iter().filter(|frame| frame.key).count(), 10);
    assert_eq!(all.iter().filter(|frame| frame.kind == FrameKind::Event).count(), 6);
    assert!(all.windows(2).all(|pair| pair[0].tick <= pair[1].tick));
}

#[test]
fn seek_finds_the_last_key_snapshot_at_or_before_a_tick() {
    let file = server_file(300);
    let replay = Replay::open(&file).expect("opens");
    let first = replay.frames_from(replay.seek(745)).next().expect("a frame");
    assert_eq!((first.tick, first.key), (720, true));
    let between = replay.frames_from(replay.seek(775)).next().expect("a frame");
    assert_eq!(between.tick, 750, "the index entry at 720 starts a short scan to the key at 750");
    let exact = replay.frames_from(replay.seek(780)).next().expect("a frame");
    assert_eq!(exact.tick, 780);
    let early = replay.frames_from(replay.seek(10)).next().expect("a frame");
    assert_eq!(early.tick, 600, "before any key: the first frame");
}

#[test]
fn a_file_cut_short_keeps_every_whole_frame_and_seeks_by_scanning() {
    let file = server_file(120);
    let cut = &file[..index_offset(&file) - 10];
    let replay = Replay::open(cut).expect("the header is intact");
    assert!(!replay.has_index(), "the trailer is gone");
    let mut frames = replay.frames();
    let snapshots = frames.by_ref().filter(|frame| frame.kind == FrameKind::Snapshot).count();
    assert_eq!(snapshots, 119, "the last frame is cut");
    assert!(frames.damaged() > 0, "the cut frame is counted as damage");
    let found = replay.frames_from(replay.seek(700)).next().expect("a frame");
    assert_eq!((found.tick, found.key), (690, true));
}

#[test]
fn a_cut_at_every_length_never_panics() {
    let file = server_file(20);
    for len in 0..file.len() {
        let replay = match Replay::open(&file[..len]) {
            Ok(replay) => replay,
            Err(_) => continue,
        };
        let _ = replay.frames().count();
        let _ = replay.seek(610);
    }
}

#[test]
fn a_damaged_frame_is_skipped_and_reading_resumes() {
    let file = server_file(60);
    let replay = Replay::open(&file).expect("opens");
    let victim = replay.frames().find(|frame| frame.tick == 620).expect("tick 620").offset;
    let mut damaged = file.clone();
    damaged[victim + 13] ^= 0x40;
    let replay = Replay::open(&damaged).expect("opens");
    let mut frames = replay.frames();
    let ticks: Vec<u32> =
        frames.by_ref().filter(|frame| frame.kind == FrameKind::Snapshot).map(|frame| frame.tick).collect();
    assert_eq!(ticks.len(), 59);
    assert!(!ticks.contains(&620) && ticks.contains(&621));
    assert!(frames.damaged() > 0);
}

#[test]
fn a_damaged_index_falls_back_to_a_scan() {
    let mut file = server_file(120);
    let at = index_offset(&file) + 20;
    file[at] ^= 0xFF;
    let replay = Replay::open(&file).expect("opens");
    assert!(!replay.has_index());
    let found = replay.frames_from(replay.seek(700)).next().expect("a frame");
    assert_eq!(found.tick, 690);
}

#[test]
fn ticks_never_go_back() {
    let mut recorder = Recorder::new(&header(RecordKind::Server), DEFAULT_MAX_PENDING);
    recorder.snapshot(700, b"a", true).expect("recorded");
    recorder.event(700, b"same tick").expect("the same tick is fine");
    assert_eq!(recorder.snapshot(699, b"b", false), Err(RecordError::OutOfOrder { last: 700, tick: 699 }));
}

#[test]
fn a_full_buffer_drops_frames_until_drained() {
    let mut recorder = Recorder::new(&header(RecordKind::Server), 0);
    let big = vec![1u8; MAX_FRAME_PAYLOAD];
    recorder.snapshot(600, &big, true).expect("the floor holds one largest frame");
    assert_eq!(recorder.snapshot(601, &big, false), Err(RecordError::Full));
    assert_eq!(recorder.dropped(), 1);
    let drained = recorder.take();
    assert_eq!(recorder.pending(), 0);
    assert!(drained.len() > MAX_FRAME_PAYLOAD);
    recorder.snapshot(602, &big, true).expect("room again");
    assert_eq!(recorder.snapshot(603, &vec![0u8; MAX_FRAME_PAYLOAD + 1], false), Err(RecordError::TooLarge));
}

#[test]
fn the_index_stays_bounded_on_a_long_recording() {
    let mut recorder = Recorder::new(&header(RecordKind::Server), DEFAULT_MAX_PENDING);
    let seconds = (MAX_INDEX_ENTRIES as u32) * 3;
    for second in 0..seconds {
        recorder.snapshot(600 + second * 60, b"k", true).expect("recorded");
        recorder.take();
    }
    let index = recorder.index();
    assert!(index.len() <= MAX_INDEX_ENTRIES);
    assert!(index.len() > MAX_INDEX_ENTRIES / 4, "{}", index.len());
    assert!(index.windows(2).all(|pair| pair[0].0 < pair[1].0 && pair[0].1 < pair[1].1));
}

#[test]
fn a_finished_recording_takes_no_more_frames() {
    let mut recorder = Recorder::new(&header(RecordKind::Server), DEFAULT_MAX_PENDING);
    recorder.finish().expect("finished");
    assert_eq!(recorder.event(700, b"late"), Err(RecordError::Finished));
    assert_eq!(recorder.finish(), Err(RecordError::Finished));
}

#[test]
fn a_client_recording_holds_inputs_and_predicted_states() {
    let state = PredictedState {
        base_tick: 640,
        position: [1.0, 2.0, 3.0],
        orientation: [0.0, 0.0, 0.0, 1.0],
        linear_velocity: [10.0, 0.0, -1.5],
        angular_velocity: [0.0, 0.25, 0.0],
    };
    let mut server = Recorder::new(&header(RecordKind::Server), DEFAULT_MAX_PENDING);
    assert_eq!(server.input(650, b"in"), Err(RecordError::WrongKind));
    assert_eq!(server.predicted(650, &state), Err(RecordError::WrongKind));
    let mut client = Recorder::new(&header(RecordKind::Client), DEFAULT_MAX_PENDING);
    client.snapshot(648, b"received", true).expect("recorded");
    client.input(650, b"input frame").expect("recorded");
    client.predicted(650, &state).expect("recorded");
    let mut file = client.take();
    file.extend(client.finish().expect("finished"));
    let replay = Replay::open(&file).expect("opens");
    assert_eq!(replay.header().kind, RecordKind::Client);
    assert_eq!(replay.header().player, PlayerId(3));
    let kinds: Vec<FrameKind> = replay.frames().map(|frame| frame.kind).collect();
    assert_eq!(kinds, [FrameKind::Snapshot, FrameKind::Input, FrameKind::Predicted]);
    let predicted = replay.frames().find(|frame| frame.kind == FrameKind::Predicted).expect("a state");
    assert_eq!(predicted.payload.len(), PREDICTED_STATE_LEN);
    assert_eq!(PredictedState::decode(predicted.payload), Some(state));
    assert_eq!(PredictedState::decode(&predicted.payload[1..]), None);
}
