use std::time::Duration;

use project_protocol::AbilityCharge;

#[test]
fn drain_consumes_the_configured_rate() {
    let mut charge = AbilityCharge::default();

    for _ in 0..60 {
        assert!(charge.drain(45, Duration::from_secs_f64(1.0 / 60.0)));
    }

    assert_eq!(charge.units(), AbilityCharge::FULL - 45);
}

#[test]
fn drain_accumulates_fractional_units_across_activations() {
    let mut charge = AbilityCharge::default();

    assert!(charge.drain(45, Duration::from_millis(10)));
    assert!(charge.drain(45, Duration::from_millis(10)));
    assert_eq!(charge.units(), AbilityCharge::FULL);

    assert!(charge.drain(45, Duration::from_millis(10)));
    assert_eq!(charge.units(), AbilityCharge::FULL - 1);
}

#[test]
fn drain_deactivates_after_exhausting_charge() {
    let mut charge = AbilityCharge::default();

    assert!(charge.drain(45, Duration::from_secs(30)));
    assert_eq!(charge.units(), 0);
    assert!(!charge.drain(45, Duration::from_secs(1)));
}
