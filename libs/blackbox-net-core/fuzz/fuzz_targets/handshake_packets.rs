//! Every handshake datagram decoder on arbitrary bytes.
#![no_main]

use blackbox_net_core::secure::classify;
use blackbox_net_core::secure::packet::{Challenge, Hello, HelloCookie, HsType, NoiseMsg, RejectReason};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = classify(data);
    let _ = Hello::decode(data);
    let _ = Challenge::decode(data);
    let _ = HelloCookie::decode(data);
    let _ = NoiseMsg::decode(data, HsType::NoiseMsg2);
    let _ = NoiseMsg::decode(data, HsType::NoiseMsg3);
    let _ = RejectReason::decode(data);
});
