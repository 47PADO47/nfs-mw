//! The preset quick lines (MP-44): ids into a table the game owns (`nfsmw_net::chat::QUICK_LINES`). The server checks the
//! id; each client shows the line in its own language.

use crate::i18n::{self, Args};

/// The preset lines' messages, by id.
pub const QUICK_LINES: [&str; 8] = [
    "lan-quick-police-behind",
    "lan-quick-roadblock-ahead",
    "lan-quick-wait-for-me",
    "lan-quick-go-go-go",
    "lan-quick-nice-one",
    "lan-quick-sorry",
    "lan-quick-good-race",
    "lan-quick-ready-when-you-are",
];

/// The line of quick message `id`.
pub fn quick_line(id: u16) -> Option<String> {
    let message = QUICK_LINES.get(usize::from(id))?;
    Some(i18n::text(message, &Args::new()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_preset_is_a_quick_id_the_server_accepts() {
        for id in 0..super::QUICK_LINES.len() as u16 {
            assert!(nfsmw_net::chat::is_quick_id(id), "{id}");
            assert!(super::quick_line(id).is_some());
        }
        assert_eq!(super::quick_line(99), None);
        assert_eq!(super::quick_line(3).as_deref(), Some("Go go go!"));
    }
}
