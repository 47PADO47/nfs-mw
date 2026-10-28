//! Bikes of the user's own RIDE 6 install as mod cars of NFS: Most Wanted (2005).
//!
//! The conversion starts from glTF files the user exports from their own install; it reads them ([`glb`]), places the
//! parts of a bike on its frame ([`layout`]), writes the car files the game reads ([`build`], [`textures`]) and the
//! mod's manifests ([`manifest`]).

pub mod build;
pub mod glb;
pub mod layout;
pub mod manifest;
pub mod textures;
