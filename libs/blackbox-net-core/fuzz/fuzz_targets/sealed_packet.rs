//! A sealed connection reading an arbitrary datagram: the clear header, the window check and the tag check.
#![no_main]

use blackbox_net_core::conn::{ConnConfig, Connection};
use blackbox_net_core::secure::{NoiseSealer, SealConfig};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let sealer = NoiseSealer::new([1; 32], [2; 32], SealConfig::default());
    let mut conn = Connection::with_sealer(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() }, sealer);
    let mut datagram = data.to_vec();
    let opened = conn.read_packet(0, &mut datagram, |_, _| {});
    assert!(opened.is_err() || data.len() >= 31, "nothing shorter than a header and a tag opens");
});
