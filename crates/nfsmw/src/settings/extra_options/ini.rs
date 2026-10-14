//! The Extra Options file reads through the generic `ini-tuning` reader (`[Section]` headers, `Key = value`
//! lines and `//` comments). The reader itself lives in `libs/ini-tuning`.

pub use ini_tuning::{entries, number, switch};
