//! The sound of a RIDE 6 bike as a mod car of NFS: Most Wanted (2005): `[engine_sound]` and `[car_sound]` tables and the
//! `files/CARS/<ID>/SOUND/*.WAV` set they name.
//!
//! RIDE 6 plays its engines through Wwise; the recordings and the mixing curves live in sound banks the game keeps in
//! an encrypted `.pak` index (`docs/formats/ride6-audio.md`). This module therefore starts from what the user decoded
//! from their own install ([`wav`], [`plan`]) and from the bike's resolved data assets ([`data`]), and never reads the
//! game's files itself. The behaviour it leans on is in `docs/specs/ride6-engine-sound.md`.
//!
//! - [`pak_key`]: the key of the game's pak files, and how the extraction commands pass it to the external reader.
//! - [`pak_cli`]: the `pak-sound` command (extract the banks, resolve every chassis's audio data).
//! - [`bnk`]: cut the recordings out of a sound bank; [`hirc`]: its object hierarchy (events, sounds, plugin objects).
//! - [`rev`]: the engine models of the bikes (Crankcase REV source plugin); [`rev_layers`]: their engine layers;
//!   [`pitch`]: pitch measurement and loop resampling; [`decode`]: `vgmstream-cli`; [`wwise`]: the extracted files.
//! - [`bike_sound`], [`bikes_cli`]: one bike's sound from the banks, and the `sound-bikes` command for all of them.
//! - [`wav`]: read any PCM or float WAV, make loops seamless, mix, write 16-bit PCM.
//! - [`data`]: the Wwise events of a bike and its backfire numbers, from the resolved JSON.
//! - [`plan`]: recordings in, files and manifest texts out.
//! - [`toml_out`]: the table texts, including the `[car_sound.overrun]` extension.
//! - [`cli`]: the `sound` command line.

pub mod bike_sound;
pub mod bikes_cli;
pub mod bnk;
pub mod cli;
pub mod data;
pub mod decode;
pub mod hirc;
pub mod pak_cli;
pub mod pak_key;
pub mod pitch;
pub mod plan;
pub mod rev;
pub mod rev_layers;
pub mod toml_out;
pub mod wav;
pub mod wwise;
