//! Stable raw navigation state for deterministic simulation diagnostics.

use super::RoadNav;

fn vector(value: glam::Vec3, visit: &mut impl FnMut(u32)) {
    for component in value.to_array() {
        visit(component.to_bits());
    }
}

impl RoadNav {
    /// Visits cursor, path, join and trail state in a stable order. Does not depend on a game or hashing algorithm.
    pub fn visit_state(&self, visit: &mut impl FnMut(u32)) {
        for value in [
            self.kind as u32,
            self.lane_type as u32,
            u32::from(self.filter.traffic),
            u32::from(self.filter.cop),
            u32::from(self.filter.no_decision),
            u32::from(self.segment),
            self.node_ind as u32,
            self.lane as u32,
            u32::from(self.dead_end),
            self.path_type as u32,
        ] {
            visit(value);
        }
        for value in [self.t, self.half_width, self.curvature] {
            visit(value.to_bits());
        }
        visit(u32::from(self.lane_offset.is_some()));
        if let Some(offset) = self.lane_offset {
            visit(offset.to_bits());
        }
        vector(self.position, visit);
        vector(self.forward, visit);
        for point in self.line.points {
            vector(point, visit);
        }
        visit(u32::from(self.bound_starts.is_some()));
        if let Some(bounds) = self.bound_starts {
            for bound in bounds {
                vector(bound, visit);
            }
        }
        visit(self.path.len() as u32);
        for &segment in &self.path {
            visit(u32::from(segment));
        }
        visit(u32::from(self.goal.is_some()));
        if let Some((segment, t)) = self.goal {
            visit(u32::from(segment));
            visit(t.to_bits());
        }
        visit(u32::from(self.trail.is_some()));
        if let Some(trail) = &self.trail {
            visit(trail.cookies().len() as u32);
            for cookie in trail.cookies() {
                vector(cookie.centre, visit);
                for value in [
                    cookie.left.x,
                    cookie.left.y,
                    cookie.right.x,
                    cookie.right.y,
                    cookie.forward.x,
                    cookie.forward.y,
                    cookie.left_offset,
                    cookie.right_offset,
                    cookie.length,
                    cookie.curvature,
                ] {
                    visit(value.to_bits());
                }
                visit(u32::from(cookie.flags));
                visit(u32::from(cookie.segment));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_cursor_poses_with_different_pending_paths_have_different_state() {
        let a = RoadNav::traffic();
        let mut b = a.clone();
        b.path = vec![2, 3];
        let collect = |nav: &RoadNav| {
            let mut words = Vec::new();
            nav.visit_state(&mut |word| words.push(word));
            words
        };
        assert_eq!(a.position, b.position);
        assert_ne!(collect(&a), collect(&b));
    }
}
