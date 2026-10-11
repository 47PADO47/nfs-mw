//! Installed AI collision reaction records; `docs/specs/vehicle-collision-reactions.md`.

use std::sync::Arc;

use blackbox_attrib::{CollectionRef, Database, RefSpec, Value};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionReaction {
    pub elasticity: f32,
    pub roll_height: f32,
    pub weight_bias: f32,
    pub mass_scale: f32,
    pub stun_speed: f32,
    pub stun_time: f32,
}

impl CollisionReaction {
    pub fn read(value: &Value) -> Option<Self> {
        let Value::Raw { bytes, .. } = value else { return None };
        let bytes: &[u8; 24] = bytes.as_slice().try_into().ok()?;
        let fields: [f32; 6] = std::array::from_fn(|i| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()));
        if !fields.iter().all(|f| f.is_finite()) {
            return None;
        }
        let [elasticity, roll_height, weight_bias, mass_scale, stun_speed, stun_time] = fields;
        Some(Self { elasticity, roll_height, weight_bias, mass_scale, stun_speed, stun_time })
    }
}

#[derive(Clone, Debug)]
pub struct CollisionReactions {
    pub front: CollisionReaction,
    pub front_side: CollisionReaction,
    pub rear: CollisionReaction,
    pub rear_side: CollisionReaction,
}

impl CollisionReactions {
    pub fn read(collection: CollectionRef<'_>) -> Option<Self> {
        Some(Self {
            front: CollisionReaction::read(collection.get("FRONT_REACTION")?)?,
            front_side: CollisionReaction::read(collection.get("FRONTSIDE_REACTION")?)?,
            rear: CollisionReaction::read(collection.get("REAR_REACTION")?)?,
            rear_side: CollisionReaction::read(collection.get("REARSIDE_REACTION")?)?,
        })
    }

    pub fn choose(&self, front: bool, side: bool) -> &CollisionReaction {
        match (front, side) {
            (true, true) => &self.front_side,
            (true, false) => &self.front,
            (false, true) => &self.rear_side,
            (false, false) => &self.rear,
        }
    }
}

/// A goal entry can intentionally resolve to no profile.
#[derive(Clone, Debug)]
pub struct GoalReaction {
    pub goal: u32,
    pub reactions: Option<Arc<CollisionReactions>>,
}

impl GoalReaction {
    pub fn reference(value: &Value) -> Option<(u32, RefSpec)> {
        let Value::Raw { bytes, .. } = value else { return None };
        let bytes: &[u8; 16] = bytes.as_slice().try_into().ok()?;
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        Some((word(0), RefSpec { class: word(4), collection: word(8) }))
    }
}

#[derive(Clone, Debug, Default)]
pub struct AiCollisionReactions {
    pub default: Option<Arc<CollisionReactions>>,
    pub goals: Vec<GoalReaction>,
}

impl AiCollisionReactions {
    pub fn read(db: &Database, vehicle: CollectionRef<'_>) -> Option<Self> {
        let ai = vehicle.follow("aivehicle")?;
        let default = ai.follow("PlayerCollisionsDefault").and_then(CollisionReactions::read).map(Arc::new);
        let mut goals = Vec::new();
        let count = match ai.get("PlayerCollisions") {
            Some(Value::Array(items)) => items.len(),
            Some(_) => 1,
            None => 0,
        };
        for i in 0..count {
            let (goal, reference) = GoalReaction::reference(ai.get_at("PlayerCollisions", i)?)?;
            let reactions = db.resolve(reference).and_then(CollisionReactions::read).map(Arc::new);
            goals.push(GoalReaction { goal, reactions });
        }
        Some(Self { default, goals })
    }

    pub fn profile(&self, goal: Option<u32>) -> Option<&Arc<CollisionReactions>> {
        if let Some(record) = goal.and_then(|key| self.goals.iter().find(|r| r.goal == key)) {
            return record.reactions.as_ref();
        }
        self.default.as_ref()
    }
}

#[cfg(test)]
mod tests;
