//! The sound of a RIDE 6 bike as a mod car of NFS: Most Wanted (2005): `[engine_sound]` and `[car_sound]` tables and the
//! `files/CARS/<ID>/SOUND/*.WAV` set they name.
//!
//! RIDE 6 plays its engines through Wwise; the recordings and the mixing curves live in sound banks the game keeps in
//! an encrypted `.pak` index (`docs/formats/ride6-audio.md`). This module therefore starts from what the user decoded
//! from their own install ([`wav`], [`plan`]) and from the bike's resolved data assets ([`data`]), and never reads the
//! game's files itself. The behaviour it leans on is in `docs/specs/ride6-engine-sound.md`.
//!
//! - [`bnk`]: cut the recordings out of a sound bank the user exported.
//! - [`wav`]: read any PCM or float WAV, make loops seamless, mix, write 16-bit PCM.
//! - [`data`]: the Wwise events of a bike and its backfire numbers, from the resolved JSON.
//! - [`plan`]: recordings in, files and manifest texts out.
//! - [`toml_out`]: the table texts, including the `[car_sound.overrun]` extension.
//! - [`cli`]: the `sound` command line.

pub mod bnk;
pub mod cli;
pub mod data;
pub mod plan;
pub mod toml_out;
pub mod wav;
