//! Particle emitters of EA Black Box games: a deterministic simulation that yields camera-facing sprites.
//! Rules: `docs/specs/exhaust-flames.md`, sections 5 (spawning) and 6 (update and drawing).
//!
//! An [`Emitter`] owns the live particles of one [`EmitterSpec`]. The owner places it every step with a
//! [`Frame`] (where the emitter is, the velocity particles may inherit, the intensity), calls
//! [`Emitter::step`], and reads the particles back as [`Sprite`]s. Nothing is drawn here; the sprite holds the
//! position, the half size, the rotation and the colour of one billboard.
//!
//! Units are the caller's (metres and seconds in the games); `+z` is up. Not modelled: start delays,
//! on/off cycles, one-shot emitters, the disc spread and animated textures.
//!
//! ```
//! use blackbox_particles::{Emitter, EmitterSpec, Frame};
//!
//! let spec = EmitterSpec { rate: 120.0, life: 0.5, speed: 4.0, size: [0.2; 4], colors: [[255; 4]; 4], ..Default::default() };
//! let mut emitter = Emitter::new(spec, 1);
//! emitter.step(1.0 / 60.0, &Frame::default());
//! emitter.step(1.0 / 60.0, &Frame::default());
//! assert_eq!(emitter.len(), 4);
//! assert_eq!(emitter.sprites().count(), 4);
//! ```

mod curve;
mod emitter;
mod rng;
mod spec;
#[cfg(test)]
mod tests;

pub use curve::Curve;
pub use emitter::{Emitter, Frame, MAX_PARTICLES, Sprite};
pub use rng::Rng;
pub use spec::EmitterSpec;
