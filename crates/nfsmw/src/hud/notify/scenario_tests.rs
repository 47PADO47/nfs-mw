//! The `notify-preempt` HUD scenario (`devtools/hud_scenarios`) does what it is named for, through the real queue.

use super::*;
use crate::devtools::hud_scenarios::{HudScenario, PREEMPT_FRAME, VIEWPORT};

fn visible(notifications: &HudNotifications) -> Vec<NoticeId> {
    notifications.queue.visible().iter().map(|active| active.key).collect()
}

#[test]
fn the_critical_event_displaces_the_low_notice() {
    let mut notifications = HudNotifications::default();
    for frame in 0..PREEMPT_FRAME {
        let state = HudScenario::NotifyPreempt.state(frame, VIEWPORT);
        notifications.take_events(&state, NotificationLevel::All);
        notifications.tick(1.0 / 60.0, intense(&state));
    }
    assert_eq!(visible(&notifications), [NoticeId::Bounty, NoticeId::Checkpoint, NoticeId::Lap]);
    let state = HudScenario::NotifyPreempt.state(PREEMPT_FRAME, VIEWPORT);
    notifications.take_events(&state, NotificationLevel::All);
    let shown = visible(&notifications);
    assert_eq!(shown.len(), 3);
    assert!(shown.contains(&NoticeId::Roadblock), "{shown:?}");
    assert!(!shown.contains(&NoticeId::Bounty), "the Low notice made room: {shown:?}");
}
