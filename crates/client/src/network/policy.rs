use lightyear::prelude::{client::InputDelayConfig, *};

// 1.5 seconds at 60 Hz. Keep history and the timeline's prediction limit coordinated:
// Lightyear uses the smaller limit, and its default rollback history is only 20 ticks.
pub(crate) const PREDICTION_TICKS: u16 = 90;

pub(crate) fn timeline_config() -> InputTimelineConfig {
    InputTimelineConfig::new(
        sync_config(),
        InputDelayConfig {
            minimum_input_delay_ticks: 0,
            maximum_input_delay_before_prediction: 0,
            maximum_predicted_ticks: PREDICTION_TICKS,
        },
    )
}

pub(crate) fn sync_config() -> SyncConfig {
    SyncConfig {
        // Retain the measured jitter multiplier, with 50 ms of fixed lead at 60 Hz
        // for scheduling/packetization. This is timeline lead, not input delay.
        jitter_multiple: 4,
        jitter_margin: 3.0,
        ..Default::default()
    }
}

pub(crate) fn prediction_manager() -> PredictionManager {
    let mut manager = PredictionManager::default();
    manager.rollback_policy.max_rollback_ticks = PREDICTION_TICKS;
    manager
}
