//! Offline measurement of a car's sound through the headless mixer (`Audio::replay`): nothing is played. A script drives a
//! car (`scenario`), `rig` renders it, `dsp` reads numbers off the render and `report` collects them per car, so a 2012 car
//! can be compared with a car of the 2005 game number by number. `real_tests` runs it on the installs (ignored without
//! `NFSMW_GAME_DIR`) and holds the regression thresholds.

#[cfg(test)]
mod curve;
mod dsp;
#[cfg(test)]
mod electric;
#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod regression;
mod report;
mod rig;
mod scenario;
