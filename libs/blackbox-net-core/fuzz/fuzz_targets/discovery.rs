//! The LAN discovery request and reply decoders on arbitrary bytes.
#![no_main]

use blackbox_net_core::discovery::{Reply, Request};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = Request::decode(data);
    let _ = Reply::decode(data);
});
