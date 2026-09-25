use std::time::Duration;

use crate::network::policy::{PREDICTION_TICKS, prediction_manager, sync_config, timeline_config};

#[test]
fn rollback_and_timeline_share_a_bounded_budget() {
    let config = timeline_config();
    let manager = prediction_manager();
    let limit = manager
        .rollback_policy
        .effective_max_rollback_ticks(&config);
    assert_eq!(limit, PREDICTION_TICKS);
    assert_eq!(config.maximum_predicted_ticks(), PREDICTION_TICKS);
    // Regress the observed 27-28 tick abort and leave headroom for a 400 ms
    // downlink plus the client's RTT-derived timeline lead and jitter.
    assert!((60..=100).contains(&limit));
    assert!(limit > 28);
}

#[test]
fn jitter_margin_includes_fixed_tick_lead() {
    assert_eq!(
        sync_config().jitter_margin(Duration::from_millis(10), Duration::from_millis(20)),
        Duration::from_millis(100)
    );
}
