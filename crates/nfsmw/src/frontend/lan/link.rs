//! What the FEng screens and the LAN menu tell each other across their systems, as the custom race's shared state does
//! (`custom_race/state.rs`): the main menu's LAN icon asks the menu to open, the LAN menu asks for the car select, and the
//! car select answers with a car.

use std::sync::{Mutex, MutexGuard, PoisonError};

use super::super::logic::Args;

/// The shared state.
#[derive(Debug)]
struct Link {
    open_requested: bool,
    car_select: bool,
    /// The car select's answer: `Some(folder)` for a pick, `Some(None)` for Back.
    car: Option<Option<String>>,
}

static LINK: Mutex<Link> = Mutex::new(Link { open_requested: false, car_select: false, car: None });

fn link() -> MutexGuard<'static, Link> {
    LINK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The main menu's LAN icon was pressed.
pub fn request_open() {
    link().open_requested = true;
}

/// Whether the LAN menu should open now (once per request).
pub fn take_open_request() -> bool {
    std::mem::take(&mut link().open_requested)
}

/// The LAN menu asks for the car select.
pub fn request_car_select() {
    link().car_select = true;
}

/// The arguments of the car select the LAN menu asked for, once.
pub fn take_car_select() -> Option<Args> {
    std::mem::take(&mut link().car_select).then_some(Args { custom: true, lan: true, ..Args::default() })
}

/// The car select's answer in a LAN session: the folder picked (`folder:preset` for a preset), or `None` for Back.
pub fn car_answer(folder: Option<String>) {
    link().car = Some(folder);
}

/// The car select's answer, once.
pub fn take_car_answer() -> Option<Option<String>> {
    link().car.take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_taken_once() {
        let _guard = crate::frontend::lan::link_tests_lock();
        request_open();
        assert!(take_open_request());
        assert!(!take_open_request());
        request_car_select();
        let args = take_car_select().unwrap();
        assert!(args.custom && args.lan);
        assert!(take_car_select().is_none());
        car_answer(Some("SL65".into()));
        assert_eq!(take_car_answer(), Some(Some("SL65".into())));
        assert_eq!(take_car_answer(), None);
    }
}
