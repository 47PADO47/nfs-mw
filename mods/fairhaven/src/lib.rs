//! Fairhaven, the city of NFS: Most Wanted (2012), as a second world of NFS: Most Wanted (2005).
//!
//! Nothing of the city is copied. The 169 streaming units of the install's `HAWAII` folder are read **by byte ranges** when the
//! camera comes near one ([`ranges`], [`install`], [`unit`]) and made into a section file of the game (solids, textures,
//! collision and the placements of the unit's models: [`convert`], [`section`]) in memory, by the maker the host calls when it
//! reads the unit's lazy file ([`maker`]). The mod's own files are only the manifests of the world and a stub for each unit
//! ([`manifest`]). Formats: `docs/formats/chameleon-world.md`; design and numbers: `docs/modding/fairhaven.md`.

pub mod collision;
pub mod config;
pub mod convert;
pub mod install;
pub mod instances;
pub mod maker;
pub mod manifest;
pub mod material;
pub mod mesh;
mod native;
pub mod pvs;
pub mod ranges;
pub mod sea;
pub mod section;
pub mod solid;
pub mod space;
pub mod start;
pub mod terrain;
pub mod textures;
pub mod unit;
