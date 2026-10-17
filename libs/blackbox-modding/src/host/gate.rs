//! The call-state gate (ADR 0005 section 4): a mod may call a host interface only on the main thread and only while a
//! mod callback runs.
//!
//! The runtime marks the thread that starts it as the main thread ([`mark_main_thread`]). Each mod call the loader makes
//! (`bb_mod_create`, `on_event`, `destroy`) runs inside a [`CallScope`], which raises the call-state flag until it is
//! dropped. The interface trampolines check the gate before they touch the game ([`check`]): a call from another thread
//! gets `BB_ERR_WRONG_THREAD`, and a call on the main thread outside any callback gets `BB_ERR_NOT_IN_CALLBACK`.
//!
//! Both facts are per thread. The game's mods run on the main thread, which is where the flag is raised and read; a
//! thread that is not marked, such as a mod's own worker thread or a parallel test, never passes the gate.

use std::cell::Cell;

use crate::abi::{BB_ERR_NOT_IN_CALLBACK, BB_ERR_WRONG_THREAD, BbStatus};

thread_local! {
    /// Whether this thread is the main thread. Set once by [`mark_main_thread`] when the runtime starts.
    static MAIN: Cell<bool> = const { Cell::new(false) };
    /// How many mod callbacks are running on this thread. A callback that makes a mod call nests a scope inside.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Record the calling thread as the main thread. The runtime calls this when it starts, on the thread that runs the
/// game; from then on the host calls of mods on this thread are checked against the call-state flag.
pub fn mark_main_thread() {
    MAIN.with(|main| main.set(true));
}

/// Raise the call-state flag for the calling thread until the returned scope is dropped. The loader enters one around
/// every call it makes into a mod.
#[must_use = "the call-state flag drops when the scope does"]
pub fn enter() -> CallScope {
    DEPTH.with(|depth| depth.set(depth.get() + 1));
    CallScope
}

/// The call-state flag while a mod callback runs. Dropping it lowers the flag again.
#[derive(Debug)]
pub struct CallScope;

impl Drop for CallScope {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Whether a host call may run now, on the calling thread. `Err` carries the status the mod sees.
pub fn check() -> Result<(), BbStatus> {
    decide(MAIN.with(Cell::get), DEPTH.with(Cell::get))
}

/// The rule [`check`] applies, on the two facts it reads: whether this thread is the main thread, and how many callbacks
/// are running on it. The thread is checked first, so a worker thread gets `BB_ERR_WRONG_THREAD` whatever the flag says.
pub const fn decide(main: bool, depth: u32) -> Result<(), BbStatus> {
    if !main {
        return Err(BB_ERR_WRONG_THREAD);
    }
    if depth == 0 {
        return Err(BB_ERR_NOT_IN_CALLBACK);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worker_thread_is_refused_whatever_the_flag() {
        assert_eq!(decide(false, 0), Err(BB_ERR_WRONG_THREAD));
        assert_eq!(decide(false, 3), Err(BB_ERR_WRONG_THREAD));
    }

    #[test]
    fn the_main_thread_outside_a_callback_is_refused() {
        assert_eq!(decide(true, 0), Err(BB_ERR_NOT_IN_CALLBACK));
    }

    #[test]
    fn the_main_thread_inside_a_callback_passes() {
        assert_eq!(decide(true, 1), Ok(()));
        assert_eq!(decide(true, 2), Ok(()));
    }

    #[test]
    fn the_flag_follows_the_scope_on_the_marked_thread() {
        mark_main_thread();
        assert_eq!(check(), Err(BB_ERR_NOT_IN_CALLBACK));
        {
            let _outer = enter();
            assert_eq!(check(), Ok(()));
            {
                let _inner = enter();
                assert_eq!(check(), Ok(()));
            }
            assert_eq!(check(), Ok(()), "a nested scope ends without clearing the outer one");
        }
        assert_eq!(check(), Err(BB_ERR_NOT_IN_CALLBACK));
    }

    #[test]
    fn a_thread_that_was_never_marked_is_refused_even_inside_a_scope() {
        let status = std::thread::spawn(|| {
            let _scope = enter();
            check()
        })
        .join()
        .expect("the worker thread does not panic");
        assert_eq!(status, Err(BB_ERR_WRONG_THREAD));
    }
}
