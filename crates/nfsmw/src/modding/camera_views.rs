//! The extra camera views mods registered through `bb.camera` 1.1: what the world draws from while one is the player's view,
//! and what the camera setting and the camera hotkey list.
//!
//! A view is named by a [`ViewKey`] (a hash of `<mod id>/<view id>`), so the setting can hold it, and keeps the callbacks the
//! mod gave. The registry lives for the thread the game and the mods run on, like the other feeds of this module
//! ([`super::outbox`]). The calls into a mod's callbacks are made with no borrow held, so a callback may not re-enter the
//! registry but nothing breaks if the host does.

use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND, BbStatus};
use blackbox_modding::host::{ViewHandlers, ViewInput, ViewPose, ViewSpec};

use crate::settings::{CameraMode, ViewKey};

struct Entry {
    handle: u64,
    key: ViewKey,
    owner: String,
    name: String,
    handlers: Rc<ViewHandlers>,
}

#[derive(Default)]
struct Registry {
    next: u64,
    views: Vec<Entry>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// A registered view as the camera setting lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub key: ViewKey,
    /// The name the mod gave it.
    pub name: String,
}

/// Add a view of mod `owner`. Returns the handle the mod holds. `BB_ERR_EXISTS` when the mod already has a view of that
/// id, `BB_ERR_ARG` for an empty id.
pub fn register(owner: &str, spec: &ViewSpec<'_>, handlers: ViewHandlers) -> Result<u64, BbStatus> {
    if spec.id.is_empty() || owner.is_empty() {
        return Err(BB_ERR_ARG);
    }
    let key = ViewKey::of(owner, spec.id);
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        if registry.views.iter().any(|view| view.key == key) {
            return Err(BB_ERR_EXISTS);
        }
        registry.next += 1;
        let handle = registry.next;
        let name = if spec.name.is_empty() { spec.id } else { spec.name };
        registry.views.push(Entry { handle, key, owner: owner.to_owned(), name: name.to_owned(), handlers: Rc::new(handlers) });
        Ok(handle)
    })
}

/// Take back the view `handle` of mod `owner`.
pub fn unregister(owner: &str, handle: u64) -> Result<(), BbStatus> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let Some(at) = registry.views.iter().position(|view| view.handle == handle && view.owner == owner) else {
            return Err(BB_ERR_NOT_FOUND);
        };
        registry.views.remove(at);
        Ok(())
    })
}

/// The key of the view `handle` of mod `owner`.
pub fn key_of(owner: &str, handle: u64) -> Option<ViewKey> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        registry.views.iter().find(|view| view.handle == handle && view.owner == owner).map(|view| view.key)
    })
}

/// Every registered view, in registration order.
pub fn all() -> Vec<ViewInfo> {
    REGISTRY.with(|registry| {
        registry.borrow().views.iter().map(|view| ViewInfo { key: view.key, name: view.name.clone() }).collect()
    })
}

/// Every camera the setting can hold: the built-in ones, then the registered extra views.
pub fn choices() -> Vec<CameraMode> {
    CameraMode::ALL.into_iter().chain(all().into_iter().map(|view| CameraMode::Extra(view.key))).collect()
}

/// The name of a view, if it is registered.
pub fn name_of(key: ViewKey) -> Option<String> {
    REGISTRY.with(|registry| registry.borrow().views.iter().find(|view| view.key == key).map(|view| view.name.clone()))
}

fn handlers_of(key: ViewKey) -> Option<Rc<ViewHandlers>> {
    REGISTRY.with(|registry| registry.borrow().views.iter().find(|view| view.key == key).map(|view| view.handlers.clone()))
}

/// Whether the view is registered and fits the car `car`.
pub fn available(key: ViewKey, car: &str) -> bool {
    handlers_of(key).is_some_and(|handlers| (handlers.available)(car))
}

/// The views that fit the car `car`, in registration order.
pub fn available_keys(car: &str) -> Vec<ViewKey> {
    all().into_iter().map(|view| view.key).filter(|key| available(*key, car)).collect()
}

/// Step the view: the pose it answers for this frame, `None` when it is unknown or declined the frame.
pub fn update(key: ViewKey, input: &ViewInput<'_>) -> Option<ViewPose> {
    let handlers = handlers_of(key)?;
    (handlers.update)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handlers(fits: &'static str) -> ViewHandlers {
        ViewHandlers {
            available: Box::new(move |car| car == fits),
            update: Box::new(|input| {
                Some(ViewPose { position: input.position, forward: input.forward, up: input.up, fov: 50.0, hide_car: false })
            }),
        }
    }

    fn input(car: &str) -> ViewInput<'_> {
        ViewInput {
            reset: false,
            dt: 0.016,
            speed: 0.0,
            steer: 0.0,
            throttle: 0.0,
            brake: 0.0,
            impact: 0.0,
            grounded: true,
            position: [1.0, 2.0, 3.0],
            forward: [1.0, 0.0, 0.0],
            up: [0.0, 0.0, 1.0],
            velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            car,
        }
    }

    #[test]
    fn views_register_once_per_owner_and_id_and_come_back_out() {
        let spec = ViewSpec { id: "chase", name: "2012 Chase" };
        let handle = register("tests.views", &spec, handlers("CARA")).unwrap();
        let key = key_of("tests.views", handle).unwrap();
        assert_eq!(register("tests.views", &spec, handlers("CARA")), Err(BB_ERR_EXISTS));
        assert!(register("other.mod", &spec, handlers("CARA")).is_ok(), "ids are per mod");
        assert_eq!(name_of(key).as_deref(), Some("2012 Chase"));
        assert!(all().iter().any(|view| view.key == key));
        assert_eq!(unregister("someone.else", handle), Err(BB_ERR_NOT_FOUND));
        assert_eq!(unregister("tests.views", handle), Ok(()));
        assert_eq!(name_of(key), None);
        assert_eq!(register("tests.views", &ViewSpec { id: "", name: "x" }, handlers("CARA")), Err(BB_ERR_ARG));
    }

    #[test]
    fn availability_follows_the_car_and_an_unknown_view_is_never_available() {
        let handle = register("tests.avail", &ViewSpec { id: "fits", name: "" }, handlers("CARA")).unwrap();
        let key = key_of("tests.avail", handle).unwrap();
        assert_eq!(name_of(key).as_deref(), Some("fits"), "an empty name falls back to the id");
        assert!(available(key, "CARA"));
        assert!(!available(key, "CARB"));
        assert!(available_keys("CARA").contains(&key));
        assert!(!available_keys("CARB").contains(&key));
        unregister("tests.avail", handle).unwrap();
        assert!(!available(key, "CARA"));
        assert_eq!(update(key, &input("CARA")), None);
    }

    #[test]
    fn update_returns_the_mods_pose() {
        let handle = register("tests.update", &ViewSpec { id: "v", name: "V" }, handlers("CARA")).unwrap();
        let key = key_of("tests.update", handle).unwrap();
        let pose = update(key, &input("CARA")).unwrap();
        assert_eq!((pose.position, pose.fov), ([1.0, 2.0, 3.0], 50.0));
        unregister("tests.update", handle).unwrap();
    }
}
