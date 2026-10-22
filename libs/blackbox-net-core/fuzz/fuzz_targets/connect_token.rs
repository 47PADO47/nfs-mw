//! Opening an arbitrary connect token.
#![no_main]

use blackbox_net_core::secure::token::{TokenKey, TokenKeyring, open_token};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut keyring = TokenKeyring::new();
    keyring.install(TokenKey::new(1, [9; 32]));
    let _ = open_token(&keyring, data);
});
