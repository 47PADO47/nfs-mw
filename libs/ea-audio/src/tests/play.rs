use super::build::*;
use super::graph_build::*;
use crate::error::Error;
use crate::mus::graph::{Graph, WalkEnd};
use crate::mus::{Chain, ChainReader, Mpf};

/// Head, three audio nodes (streams 0 to 2), end. The streams sit at 0x100, 0x200 and 0x300 in the `.mus`.
fn map() -> Map {
    Map {
        nodes: vec![
            control(0, vec![(0, 127, 1)], 0),
            audio(0, 2),
            audio(1, 3),
            audio(2, 4),
            control(0xFFFF, vec![], 0),
        ],
        events: vec![song_event(5, 0)],
        streams: vec![(2, 1000), (4, 2000), (6, 1000)],
        ..Map::default()
    }
}

fn header(rate: u32) -> Vec<u8> {
    gstr_header(&[tag(0x80, 3), tag(0x84, rate), tag(0x85, 28)])
}

fn block(start: i16) -> Vec<u8> {
    scdl(28, &[0], &xa_pcm_frame(0, 0, &ramp(start)), true)
}

/// A `.mus` whose second stream has two blocks and whose third has the rate `third_rate`.
fn mus(third_rate: u32) -> Vec<u8> {
    let mut file = vec![0xFA, 0xCE, 0xA5, 0x8C];
    for (at, bytes) in [
        (0x100, stream(&header(36000), &[block(100)])),
        (0x200, stream(&header(36000), &[block(1000), block(2000)])),
        (0x300, stream(&header(third_rate), &[block(5000)])),
    ] {
        file.resize(at, 0);
        file.extend(bytes);
    }
    file
}

fn chain() -> (Mpf, Chain) {
    let bytes = map().build();
    let (mpf, graph) = (Mpf::parse(&bytes).unwrap(), Graph::parse(&bytes).unwrap());
    let chain = mpf.song_chain(&graph, 5, 0).unwrap();
    assert_eq!((chain.segments.len(), chain.end), (3, WalkEnd::EndNode));
    (mpf, chain)
}

#[test]
fn the_streams_of_a_chain_play_as_one_run_of_samples() {
    let (mpf, chain) = chain();
    let mus = mus(36000);
    let mut reader = ChainReader::new(&mpf, &mus[..], &chain).unwrap();
    assert_eq!((reader.sample_rate(), reader.channels()), (36000, 1));
    let (mut out, mut segments) = (Vec::new(), Vec::new());
    while let Some(frames) = reader.next_chunk(&mut out).unwrap() {
        assert_eq!(frames, 28);
        segments.push(reader.segment_index());
    }
    assert_eq!(segments, [0, 1, 1, 2]);
    let expected: Vec<i16> = [100, 1000, 2000, 5000].into_iter().flat_map(ramp).collect();
    assert_eq!(out, expected);
    // Past the end there is nothing more, however often it is asked.
    assert_eq!(reader.next_chunk(&mut out).unwrap(), None);
    assert_eq!(reader.next_chunk(&mut out).unwrap(), None);
    assert_eq!(out.len(), 112);
}

#[test]
fn streams_that_differ_in_rate_are_an_error() {
    let (mpf, chain) = chain();
    let mus = mus(32000);
    let mut reader = ChainReader::new(&mpf, &mus[..], &chain).unwrap();
    let mut out = Vec::new();
    let result = (0..4).map(|_| reader.next_chunk(&mut out)).find(Result::is_err);
    assert!(matches!(result, Some(Err(Error::Corrupt(_)))));
}

#[test]
fn an_empty_chain_or_a_missing_stream_cannot_be_opened() {
    let (mpf, chain) = chain();
    let mus = mus(36000);
    let empty = Chain { segments: vec![], end: WalkEnd::DeadEnd };
    assert!(matches!(ChainReader::new(&mpf, &mus[..], &empty), Err(Error::Corrupt(_))));
    // A source that is too short to hold the first stream.
    assert!(ChainReader::new(&mpf, &mus[..0x80], &chain).is_err());
}
