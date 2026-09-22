use std::time::Duration;

use crate::network::recovery::{MAX_FRAME_GAP, Recovery, Suspension};

#[test]
fn normal_frames_and_short_visibility_changes_preserve_the_session() {
    let mut policy = Suspension::default();
    for millis in [0, 16, 32, 48] {
        assert_eq!(
            policy.observe(
                Duration::from_millis(millis),
                Duration::from_millis(16),
                false
            ),
            Recovery::None
        );
    }
    assert_eq!(
        policy.observe(Duration::from_millis(50), Duration::ZERO, true),
        Recovery::None
    );
    assert_eq!(
        policy.observe(Duration::from_millis(100), Duration::from_millis(50), false),
        Recovery::None
    );
}

#[test]
fn unclamped_frame_gap_reconnects_at_the_boundary_once() {
    let mut policy = Suspension::default();
    assert_eq!(
        policy.observe(MAX_FRAME_GAP, MAX_FRAME_GAP, false),
        Recovery::Reconnect
    );
    assert_eq!(
        policy.observe(MAX_FRAME_GAP, Duration::ZERO, false),
        Recovery::None
    );
}

#[test]
fn continuously_rendering_hidden_tab_retires_once_and_waits_for_visibility() {
    let mut policy = Suspension::default();
    assert_eq!(
        policy.observe(Duration::ZERO, Duration::ZERO, true),
        Recovery::None
    );
    assert_eq!(
        policy.observe(MAX_FRAME_GAP, Duration::from_millis(16), true),
        Recovery::Suspend
    );
    assert_eq!(
        policy.observe(Duration::from_secs(10), Duration::from_secs(9), true),
        Recovery::None
    );
    assert_eq!(
        policy.observe(Duration::from_secs(11), Duration::from_millis(16), false),
        Recovery::Reconnect
    );
    assert!(!policy.is_suspended());
}

#[test]
fn hidden_duration_is_checked_even_with_a_short_resume_frame() {
    let mut policy = Suspension::default();
    policy.observe(Duration::ZERO, Duration::ZERO, true);
    assert_eq!(
        policy.observe(Duration::from_secs(2), Duration::from_millis(16), false),
        Recovery::Reconnect
    );
}
