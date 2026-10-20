//! The queue of cop spawn requests (`docs/specs/ai-pursuit-cops.md` section 1, step 7): a script, a mod or the console asks
//! for a cop car by name at a point, and the manager places it when it can. A request whose spot overlaps another car, whose
//! model is not loaded or that comes during the lockout stays queued and is tried again next run.

use glam::Vec3;

/// What the cop does once it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// It cruises the streets like a patrol (`StartPatrol`).
    Patrol,
    /// It joins the player's chase (`StartPursuit`).
    Chase,
}

/// One request: the car, where, which way it faces, and what it does.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub name: String,
    /// Physics space.
    pub at: Vec3,
    pub forward: Vec3,
    pub role: Role,
}

/// The requests waiting to be placed, oldest first.
#[derive(Debug, Default)]
pub struct Queue {
    waiting: Vec<Request>,
}

impl Queue {
    pub fn push(&mut self, request: Request) {
        self.waiting.push(request);
    }

    /// Whether any request waits: the patrol pass then asks for no new patrol (`wantpatrolcars = 0`).
    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }

    pub fn len(&self) -> usize {
        self.waiting.len()
    }

    /// Hands every waiting request to `place`, oldest first; the ones it returns `false` for stay queued.
    pub fn service(&mut self, mut place: impl FnMut(&Request) -> bool) {
        self.waiting.retain(|request| !place(request));
    }

    /// Drops every request of `role` (a chase request has no chase to join once the chase is over).
    pub fn drop_role(&mut self, role: Role) {
        self.waiting.retain(|request| request.role != role);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(name: &str, role: Role) -> Request {
        Request { name: name.to_owned(), at: Vec3::ZERO, forward: Vec3::Z, role }
    }

    #[test]
    fn a_request_that_cannot_be_placed_stays_for_the_next_run() {
        let mut queue = Queue::default();
        assert!(queue.is_empty());
        queue.push(request("a", Role::Patrol));
        queue.push(request("b", Role::Patrol));
        queue.service(|request| request.name == "a");
        assert_eq!(queue.len(), 1, "a was placed, b waits");
        queue.service(|_| false);
        assert_eq!(queue.len(), 1);
        queue.service(|_| true);
        assert!(queue.is_empty());
    }

    #[test]
    fn requests_are_tried_oldest_first() {
        let mut queue = Queue::default();
        for name in ["a", "b", "c"] {
            queue.push(request(name, Role::Patrol));
        }
        let mut order = Vec::new();
        queue.service(|request| {
            order.push(request.name.clone());
            true
        });
        assert_eq!(order, ["a", "b", "c"]);
    }

    #[test]
    fn the_chase_requests_can_be_dropped_when_the_chase_is_over() {
        let mut queue = Queue::default();
        queue.push(request("a", Role::Chase));
        queue.push(request("b", Role::Patrol));
        queue.drop_role(Role::Chase);
        assert_eq!(queue.len(), 1);
    }
}
