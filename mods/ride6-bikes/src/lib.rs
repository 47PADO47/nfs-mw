//! Bikes of the user's own RIDE 6 install as mod cars of NFS: Most Wanted (2005).
//!
//! A small exporter script (outside the repository) writes a bike's part table and its meshes and textures from the
//! user's install. The importer reads that folder: the part table ([`partfile`]) says what the stock parts are and where
//! they sit, the meshes ([`glb`], [`exports`]) and pictures ([`textures`]) are baked to the 2005 car shader's inputs
//! ([`bake`], [`shading`]), placed on the car frame ([`layout`]) and written as the car files ([`build`], [`solids`]) and
//! the mod's manifests ([`manifest`]). The parts that steer are told from the frame by the skeleton ([`front_end`]) and
//! written as a solid of their own ([`fork`]). The bike's physics data (a resolved JSON the user's tooling writes from the
//! install) is converted to the manifest's `[bike]` table by [`physics`] and [`bike_table`]; the sound tables join the
//! manifest through [`manifest_sound`].

// The tests build the large physics JSON with `json!`, which recurses once per key.
#![cfg_attr(test, recursion_limit = "512")]

pub mod bake;
pub mod batch;
pub mod bike_table;
pub mod build;
pub mod cli;
pub mod config;
pub mod exports;
pub mod fork;
pub mod front_end;
pub mod glb;
pub mod job;
pub mod layout;
pub mod manifest;
pub mod manifest_sound;
pub mod naming;
mod native;
pub mod partfile;
pub mod physics;
pub mod pipeline;
pub mod rider;
pub mod shading;
pub mod solids;
pub mod sound;
pub mod stamp;
pub mod summary;
pub mod textures;
