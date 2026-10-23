//! Finishing the room-password exchange with an arbitrary peer message and checking an arbitrary tag.
#![no_main]

use blackbox_net_core::secure::password::{PakeRole, start};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok((state, _)) = start(b"password", &[0; 32], PakeRole::Host) else {
        return;
    };
    let (message, tag) = data.split_at(data.len().min(33));
    if let Ok(confirm) = state.finish(message) {
        let _ = confirm.verify(tag);
    }
});
