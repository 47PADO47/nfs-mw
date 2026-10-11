use super::*;

fn record(values: [f32; 6]) -> Value {
    Value::Raw { type_key: 1, bytes: values.into_iter().flat_map(f32::to_le_bytes).collect() }
}

#[test]
fn records_preserve_all_fields_and_reject_wrong_lengths_or_nonfinite_data() {
    let r = CollisionReaction::read(&record([0.25, -0.15, 0.9, 0.65, 30.0, 2.0])).unwrap();
    assert_eq!(
        (r.elasticity, r.roll_height, r.weight_bias, r.mass_scale, r.stun_speed, r.stun_time),
        (0.25, -0.15, 0.9, 0.65, 30.0, 2.0)
    );
    assert!(CollisionReaction::read(&Value::Raw { type_key: 1, bytes: vec![0; 23] }).is_none());
    assert!(CollisionReaction::read(&record([f32::NAN; 6])).is_none());
    let value = Value::Raw { type_key: 1, bytes: [7_u32, 11, 13, 0].into_iter().flat_map(u32::to_le_bytes).collect() };
    assert_eq!(GoalReaction::reference(&value), Some((7, RefSpec { class: 11, collection: 13 })));
}

fn profile(scale: f32) -> Arc<CollisionReactions> {
    let r = CollisionReaction { mass_scale: scale, ..Default::default() };
    Arc::new(CollisionReactions { front: r, front_side: r, rear: r, rear_side: r })
}

#[test]
fn goal_selection_keeps_first_match_and_an_explicit_missing_profile() {
    let default = profile(1.0);
    let specific = profile(2.6);
    let reactions = AiCollisionReactions {
        default: Some(default.clone()),
        goals: vec![
            GoalReaction { goal: 7, reactions: Some(specific.clone()) },
            GoalReaction { goal: 7, reactions: Some(default.clone()) },
            GoalReaction { goal: 8, reactions: None },
        ],
    };
    assert!(Arc::ptr_eq(reactions.profile(Some(7)).unwrap(), &specific));
    assert!(Arc::ptr_eq(reactions.profile(Some(9)).unwrap(), &default));
    assert!(Arc::ptr_eq(reactions.profile(None).unwrap(), &default));
    assert!(reactions.profile(Some(8)).is_none());
}
